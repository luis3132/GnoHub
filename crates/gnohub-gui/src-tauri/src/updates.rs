//! Actualizaciones desde las releases de GitHub (mismo esquema que ControlCode).
//!
//! Dos niveles, según cómo se compiló la app:
//!
//! - **Siempre**: se consulta la última release del repo y, si es más nueva, se avisa con
//!   sus notas y el paquete exacto de este sistema y arquitectura.
//! - **Con llave pública** (`GNOHUB_UPDATER_PUBKEY` al compilar, ver el workflow de
//!   release): el plugin de Tauri baja la versión nueva de `latest.json`, verifica su firma
//!   e instala el mismo formato con que se instaló la app (`.rpm` o `.deb`, con `pkexec`:
//!   el sistema pide la contraseña).
//!
//! Sin la llave no se instala nada solo: nada que no se pueda verificar.

use rust_i18n::t;
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Runtime};

const REPO: &str = "luis3132/GnoHub";

/// La llave PÚBLICA con la que se verifican las actualizaciones. No es secreta (va en el
/// binario), pero sin ella no hay actualización automática.
const PUBKEY: &str = match option_env!("GNOHUB_UPDATER_PUBKEY") {
    Some(k) => k,
    None => "",
};

/// El plugin de actualización. Se registra siempre, pero sin llave no instala nada:
/// `update_install` lo rechaza antes, y la verificación de firma fallaría de todos modos.
pub fn plugin<R: Runtime>() -> tauri::plugin::TauriPlugin<R, tauri_plugin_updater::Config> {
    let builder = tauri_plugin_updater::Builder::new();
    if PUBKEY.trim().is_empty() {
        builder.build()
    } else {
        builder.pubkey(PUBKEY).build()
    }
}

#[derive(Debug, Serialize)]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub newer: bool,
    pub notes: Option<String>,
    pub page_url: String,
    pub published_at: Option<String>,
    /// `auto`: se instala desde la app. `download`: se ofrece el paquete.
    pub install: &'static str,
    pub download_url: Option<String>,
    pub asset_name: Option<String>,
}

/// `1.7.10` > `1.7.9`: por números, no por texto. Lo que no es número se ignora.
pub(crate) fn newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        v.trim_start_matches('v')
            .split(['.', '-', '+'])
            .map_while(|p| p.parse().ok())
            .collect()
    };
    parse(latest) > parse(current)
}

/// Cómo se instaló esta app: `deb`, `rpm`, o `None` (desarrollo, o ejecutada desde
/// `target/`). El bundler lo deja marcado en el binario.
fn bundle() -> Option<&'static str> {
    use tauri::utils::config::BundleType;
    match tauri::utils::platform::bundle_type()? {
        BundleType::Deb => Some("deb"),
        BundleType::Rpm => Some("rpm"),
        _ => None,
    }
}

/// Sin saber cómo se instaló, el formato de la distribución: `deb` en las derivadas de
/// Debian, `rpm` en el resto.
fn distro_format() -> &'static str {
    let release = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let debian_like = release.lines().any(|l| {
        (l.starts_with("ID=") || l.starts_with("ID_LIKE="))
            && (l.contains("debian") || l.contains("ubuntu"))
    });
    if debian_like { "deb" } else { "rpm" }
}

/// El nombre de paquete que le corresponde a esta arquitectura y formato, según cómo
/// nombra el bundler cada uno (`GnoHub-0.0.1-1.x86_64.rpm`, `GnoHub_0.0.1_arm64.deb`).
pub(crate) fn asset_matches(name: &str, arch: &str, format: &str) -> bool {
    let arm = arch == "aarch64";
    match format {
        "deb" => name.ends_with(if arm { "_arm64.deb" } else { "_amd64.deb" }),
        _ => name.ends_with(if arm { ".aarch64.rpm" } else { ".x86_64.rpm" }),
    }
}

fn http() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("GnoHub-App")
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_check(app: AppHandle) -> Result<UpdateInfo, String> {
    let current = app.package_info().version.to_string();
    let release: Value = http()?
        .get(format!(
            "https://api.github.com/repos/{REPO}/releases/latest"
        ))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| t!("updates.github_unreachable", error = e).into_owned())?
        .error_for_status()
        .map_err(|e| t!("updates.github_error", error = e).into_owned())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let str_of = |k: &str| release.get(k).and_then(Value::as_str).map(str::to_string);
    let tag = str_of("tag_name").ok_or_else(|| t!("updates.no_tag").into_owned())?;
    let latest = tag.trim_start_matches('v').to_string();
    let bundle = bundle();
    let format = bundle.unwrap_or_else(distro_format);
    let asset = release
        .get("assets")
        .and_then(Value::as_array)
        .and_then(|assets| {
            assets.iter().find(|a| {
                a.get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|n| asset_matches(n, std::env::consts::ARCH, format))
            })
        });
    Ok(UpdateInfo {
        newer: newer(&latest, &current),
        current,
        latest,
        notes: str_of("body").filter(|b| !b.trim().is_empty()),
        page_url: str_of("html_url")
            .unwrap_or_else(|| format!("https://github.com/{REPO}/releases/tag/{tag}")),
        published_at: str_of("published_at"),
        install: if !PUBKEY.trim().is_empty() && bundle.is_some() {
            "auto"
        } else {
            "download"
        },
        download_url: asset
            .and_then(|a| a.get("browser_download_url"))
            .and_then(Value::as_str)
            .map(str::to_string),
        asset_name: asset
            .and_then(|a| a.get("name"))
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

#[derive(Clone, Serialize)]
struct Progress {
    downloaded: u64,
    total: Option<u64>,
}

/// Baja la versión nueva, verifica la firma e instala. Después reinicia el servicio
/// `gnohubd` para que también corra la versión nueva. La app no se reinicia sola: eso lo
/// decide la persona (`update_restart`).
#[tauri::command]
pub async fn update_install(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;
    if PUBKEY.trim().is_empty() || bundle().is_none() {
        return Err(t!("updates.no_auto").into_owned());
    }
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| t!("updates.check_failed", error = e).into_owned())?
        .ok_or_else(|| t!("updates.none").into_owned())?;
    let mut downloaded = 0u64;
    let emitter = app.clone();
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                let _ = emitter.emit("gnohub-update-progress", Progress { downloaded, total });
            },
            || {},
        )
        .await
        .map_err(|e| t!("updates.install_failed", error = e).into_owned())?;
    // El paquete reemplazó también /usr/bin/gnohubd: que el servicio use el nuevo.
    let _ = crate::service::run(crate::service::Action::Restart).await;
    Ok(())
}

/// Reinicia la app con la versión recién instalada.
#[tauri::command]
pub fn update_restart(app: AppHandle) {
    app.restart();
}

/// Abre en el navegador una página de las releases de GitHub (solo de este repo).
#[tauri::command]
pub fn open_release_url(url: String) -> Result<(), String> {
    let allowed = [
        format!("https://github.com/{REPO}/releases/"),
        format!("https://github.com/{REPO}/"),
    ];
    if !allowed.iter().any(|p| url.starts_with(p)) {
        return Err("URL no permitida".into());
    }
    std::process::Command::new("xdg-open")
        .arg(&url)
        .spawn()
        .map(drop)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_by_number() {
        assert!(newer("0.0.10", "0.0.9"));
        assert!(newer("v0.1.0", "0.0.1"));
        assert!(!newer("0.0.1", "0.0.1"));
        assert!(!newer("0.0.1", "0.0.2"));
        assert!(newer("1.0.0", "0.99.99"));
    }

    #[test]
    fn each_system_picks_its_package() {
        let names = [
            "GnoHub-0.0.2-1.x86_64.rpm",
            "GnoHub-0.0.2-1.aarch64.rpm",
            "GnoHub_0.0.2_amd64.deb",
            "GnoHub_0.0.2_arm64.deb",
        ];
        let pick = |arch: &str, format: &str| {
            names
                .iter()
                .filter(|n| asset_matches(n, arch, format))
                .copied()
                .collect::<Vec<_>>()
        };
        assert_eq!(pick("x86_64", "rpm"), vec!["GnoHub-0.0.2-1.x86_64.rpm"]);
        assert_eq!(pick("aarch64", "rpm"), vec!["GnoHub-0.0.2-1.aarch64.rpm"]);
        assert_eq!(pick("x86_64", "deb"), vec!["GnoHub_0.0.2_amd64.deb"]);
        assert_eq!(pick("aarch64", "deb"), vec!["GnoHub_0.0.2_arm64.deb"]);
    }
}
