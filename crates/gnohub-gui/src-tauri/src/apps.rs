//! Aplicaciones instaladas (para elegirlas en un perfil) y la extensión de
//! GNOME que informa de la ventana activa.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Serialize;
use tokio::process::Command;

const EXTENSION_UUID: &str = "gnohub@gnohub.dev";
#[derive(Debug, Clone, Serialize)]
pub struct InstalledApp {
    /// Identificador de escritorio sin `.desktop` (`org.mozilla.firefox`).
    pub id: String,
    pub name: String,
    /// `StartupWMClass`, si el archivo lo indica (otros entornos usan la clase).
    pub wm_class: Option<String>,
}

fn data_dirs() -> Vec<PathBuf> {
    let home = directories::BaseDirs::new().map(|d| d.data_local_dir().to_path_buf());
    let system =
        std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    home.into_iter()
        .chain(std::env::split_paths(&system))
        .chain([PathBuf::from("/var/lib/flatpak/exports/share")])
        .chain(
            directories::BaseDirs::new().map(|d| d.data_local_dir().join("flatpak/exports/share")),
        )
        .map(|d| d.join("applications"))
        .collect()
}

/// Lee la sección `[Desktop Entry]` de un `.desktop`.
fn parse_desktop(text: &str) -> Option<(String, Option<String>)> {
    let mut in_entry = false;
    let (mut name, mut wm_class, mut hidden, mut is_app) = (None, None, false, false);
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        match line.split_once('=') {
            Some(("Name", v)) => name = Some(v.trim().to_owned()),
            Some(("StartupWMClass", v)) => wm_class = Some(v.trim().to_owned()),
            Some(("Type", v)) => is_app = v.trim() == "Application",
            Some(("NoDisplay" | "Hidden", v)) => hidden |= v.trim() == "true",
            _ => {}
        }
    }
    (is_app && !hidden).then_some(())?;
    Some((name?, wm_class))
}

pub fn installed() -> Vec<InstalledApp> {
    // Por id: el primer directorio (el del usuario) tiene prioridad.
    let mut apps: BTreeMap<String, InstalledApp> = BTreeMap::new();
    for dir in data_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(id) = path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(".desktop"))
            else {
                continue;
            };
            if apps.contains_key(id) {
                continue;
            }
            let Some((name, wm_class)) = std::fs::read_to_string(&path)
                .ok()
                .as_deref()
                .and_then(parse_desktop)
            else {
                continue;
            };
            apps.insert(
                id.to_owned(),
                InstalledApp {
                    id: id.to_owned(),
                    name,
                    wm_class,
                },
            );
        }
    }
    let mut list: Vec<_> = apps.into_values().collect();
    list.sort_by_key(|a| a.name.to_lowercase());
    list
}

#[derive(Debug, Clone, Serialize)]
pub struct FocusStatus {
    /// Entorno detectado: "gnome", "hyprland", "sway", "i3", "x11" u "otro".
    pub desktop: String,
    /// Solo GNOME: la extensión está instalada / activa.
    pub extension_installed: bool,
    pub extension_active: bool,
}

fn desktop() -> String {
    let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    if env("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
        "hyprland"
    } else if env("SWAYSOCK").is_some() {
        "sway"
    } else if env("I3SOCK").is_some() {
        "i3"
    } else if env("XDG_CURRENT_DESKTOP").is_some_and(|d| d.contains("GNOME")) {
        "gnome"
    } else if env("XDG_SESSION_TYPE").as_deref() == Some("x11") {
        "x11"
    } else {
        "otro"
    }
    .to_owned()
}

/// La extensión la instala el paquete en el directorio del sistema.
fn extension_installed() -> bool {
    let system =
        std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    let user = directories::BaseDirs::new().map(|d| d.data_local_dir().to_path_buf());
    user.into_iter()
        .chain(std::env::split_paths(&system))
        .any(|d| {
            d.join("gnome-shell/extensions")
                .join(EXTENSION_UUID)
                .join("extension.js")
                .is_file()
        })
}

pub async fn focus_status() -> FocusStatus {
    let desktop = desktop();
    let mut status = FocusStatus {
        extension_installed: extension_installed(),
        extension_active: false,
        desktop,
    };
    if status.desktop == "gnome"
        && let Ok(out) = Command::new("gnome-extensions")
            .args(["info", EXTENSION_UUID])
            .output()
            .await
    {
        let info = String::from_utf8_lossy(&out.stdout);
        status.extension_active = info.lines().any(|l| {
            let l = l.trim();
            l == "State: ACTIVE" || l == "State: ENABLED"
        });
    }
    status
}

/// Habilita la extensión (instalada por el paquete). En Wayland, GNOME solo
/// carga una extensión recién instalada al volver a iniciar sesión.
pub async fn enable_extension() -> Result<FocusStatus, String> {
    if !extension_installed() {
        return Err(rust_i18n::t!("extension.not_installed").into_owned());
    }
    // Si GNOME aún no la conoce (recién instalada), esto falla; se habilita
    // igualmente en la configuración para que arranque al volver a entrar.
    let enabled = Command::new("gnome-extensions")
        .args(["enable", EXTENSION_UUID])
        .output()
        .await
        .is_ok_and(|o| o.status.success());
    if !enabled {
        enable_in_gsettings().await?;
    }
    Ok(focus_status().await)
}

async fn enable_in_gsettings() -> Result<(), String> {
    let out = Command::new("gsettings")
        .args(["get", "org.gnome.shell", "enabled-extensions"])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    let current = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    if current.contains(EXTENSION_UUID) {
        return Ok(());
    }
    let list = match current.trim_start_matches("@as ").trim() {
        "[]" | "" => format!("['{EXTENSION_UUID}']"),
        l => format!("{}, '{EXTENSION_UUID}']", l.trim_end_matches(']')),
    };
    let status = Command::new("gsettings")
        .args(["set", "org.gnome.shell", "enabled-extensions", &list])
        .status()
        .await
        .map_err(|e| e.to_string())?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| rust_i18n::t!("extension.enable_failed").into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_desktop_entries() {
        let firefox = "[Desktop Entry]\nType=Application\nName=Firefox\nStartupWMClass=firefox\n[Desktop Action new]\nName=Nueva ventana\n";
        assert_eq!(
            parse_desktop(firefox),
            Some(("Firefox".into(), Some("firefox".into())))
        );
        assert_eq!(
            parse_desktop("[Desktop Entry]\nType=Application\nName=X\nNoDisplay=true\n"),
            None
        );
        assert_eq!(parse_desktop("[Desktop Entry]\nType=Link\nName=X\n"), None);
    }
}
