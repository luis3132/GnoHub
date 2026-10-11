//! Sigue qué aplicación tiene el foco, para los perfiles por aplicación.
//!
//! No hay una API común en Linux, así que hay un backend por entorno:
//! - GNOME (Wayland y X11): la extensión de GnoHub publica el foco por D-Bus.
//! - Hyprland: socket de eventos.
//! - Sway e i3: suscripción por su IPC (`swaymsg` / `i3-msg`).
//! - Otros entornos X11: `xprop`.
//!
//! Si ninguno está disponible, el foco queda en `None` y solo se aplican las
//! asignaciones generales.

use std::process::Stdio;
use std::time::Duration;

use futures_util::StreamExt;
use gnohub_core::ActiveApp;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::watch;
use tracing::{debug, info, warn};

pub type Focus = watch::Receiver<Option<ActiveApp>>;
type Sink = watch::Sender<Option<ActiveApp>>;

/// Cuánto esperar antes de reintentar un backend que se cayó.
const RETRY: Duration = Duration::from_secs(5);

pub fn spawn() -> Focus {
    let (tx, rx) = watch::channel(None);
    let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());

    if let Some(signature) = env("HYPRLAND_INSTANCE_SIGNATURE") {
        info!("foco de ventana: Hyprland");
        tokio::spawn(hyprland(tx, signature));
    } else if env("SWAYSOCK").is_some() {
        info!("foco de ventana: Sway");
        tokio::spawn(i3_ipc(tx, "swaymsg"));
    } else if env("I3SOCK").is_some() {
        info!("foco de ventana: i3");
        tokio::spawn(i3_ipc(tx, "i3-msg"));
    } else if env("XDG_SESSION_TYPE").as_deref() == Some("x11")
        && env("DISPLAY").is_some()
        && !env("XDG_CURRENT_DESKTOP").is_some_and(|d| d.contains("GNOME"))
    {
        info!("foco de ventana: X11");
        tokio::spawn(x11(tx));
    } else {
        // GNOME, o un entorno sin variables en el servicio: se escucha la
        // extensión; si no está instalada, simplemente no llega nada.
        info!("foco de ventana: extensión de GNOME (dev.gnohub.Focus)");
        tokio::spawn(gnome(tx));
    }
    rx
}

fn publish(tx: &Sink, app: ActiveApp) {
    tx.send_if_modified(|current| {
        if current.as_ref() == Some(&app) {
            return false;
        }
        debug!(app_id = %app.app_id, wm_class = %app.wm_class, "foco");
        *current = Some(app);
        true
    });
}

#[zbus::proxy(
    interface = "dev.gnohub.Focus",
    default_service = "dev.gnohub.Focus",
    default_path = "/dev/gnohub/Focus"
)]
trait FocusService {
    fn get(&self) -> zbus::Result<(String, String)>;

    #[zbus(signal)]
    fn changed(&self, app_id: String, wm_class: String) -> zbus::Result<()>;
}

async fn gnome(tx: Sink) {
    loop {
        if let Err(e) = gnome_once(&tx).await {
            warn!("foco de GNOME: {e}");
        }
        tokio::time::sleep(RETRY).await;
    }
}

async fn gnome_once(tx: &Sink) -> zbus::Result<()> {
    let conn = zbus::Connection::session().await?;
    let proxy = FocusServiceProxy::new(&conn).await?;
    let mut changes = proxy.receive_changed().await?;
    // Al (re)aparecer la extensión se pide el estado actual.
    let mut owners = proxy.inner().receive_owner_changed().await?;
    if let Ok((app_id, wm_class)) = proxy.get().await {
        publish(tx, ActiveApp { app_id, wm_class });
    } else {
        info!("la extensión de GNOME no está activa; los perfiles por aplicación no se aplicarán");
    }
    loop {
        tokio::select! {
            Some(signal) = changes.next() => {
                let args = signal.args()?;
                publish(tx, ActiveApp { app_id: args.app_id, wm_class: args.wm_class });
            }
            Some(owner) = owners.next() => {
                if owner.is_some() {
                    if let Ok((app_id, wm_class)) = proxy.get().await {
                        publish(tx, ActiveApp { app_id, wm_class });
                    }
                } else {
                    tx.send_replace(None);
                }
            }
            else => return Ok(()),
        }
    }
}

async fn hyprland(tx: Sink, signature: String) {
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    let candidates = [
        format!("{runtime}/hypr/{signature}/.socket2.sock"),
        format!("/tmp/hypr/{signature}/.socket2.sock"),
    ];
    loop {
        for path in &candidates {
            let Ok(stream) = tokio::net::UnixStream::connect(path).await else {
                continue;
            };
            let mut lines = BufReader::new(stream).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                // activewindow>>CLASE,TÍTULO
                if let Some(rest) = line.strip_prefix("activewindow>>") {
                    let class = rest.split(',').next().unwrap_or_default().to_owned();
                    publish(
                        &tx,
                        ActiveApp {
                            app_id: class.clone(),
                            wm_class: class,
                        },
                    );
                }
            }
        }
        tokio::time::sleep(RETRY).await;
    }
}

async fn i3_ipc(tx: Sink, tool: &'static str) {
    loop {
        let child = Command::new(tool)
            .args(["-t", "subscribe", "-m", r#"["window"]"#])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn();
        match child {
            Ok(mut child) => {
                let stdout = child.stdout.take().expect("stdout con pipe");
                let mut lines = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if let Some(app) = parse_i3_event(&line) {
                        publish(&tx, app);
                    }
                }
            }
            Err(e) => warn!("no se pudo ejecutar {tool}: {e}"),
        }
        tokio::time::sleep(RETRY).await;
    }
}

fn parse_i3_event(line: &str) -> Option<ActiveApp> {
    let event: serde_json::Value = serde_json::from_str(line).ok()?;
    if event["change"] != "focus" {
        return None;
    }
    let container = &event["container"];
    let class = container["window_properties"]["class"]
        .as_str()
        .unwrap_or_default();
    // Sway: `app_id` para ventanas Wayland nativas; `class` para XWayland.
    let app_id = container["app_id"]
        .as_str()
        .filter(|s| !s.is_empty())
        .unwrap_or(class);
    Some(ActiveApp {
        app_id: app_id.to_owned(),
        wm_class: if class.is_empty() { app_id } else { class }.to_owned(),
    })
}

async fn x11(tx: Sink) {
    loop {
        let child = Command::new("xprop")
            .args(["-spy", "-root", "_NET_ACTIVE_WINDOW"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn();
        match child {
            Ok(mut child) => {
                let stdout = child.stdout.take().expect("stdout con pipe");
                let mut lines = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    // _NET_ACTIVE_WINDOW(WINDOW): window id # 0x3a00007
                    let Some(id) = line.rsplit(' ').next().filter(|id| id.starts_with("0x")) else {
                        continue;
                    };
                    if let Some(class) = x11_class(id).await {
                        publish(
                            &tx,
                            ActiveApp {
                                app_id: class.clone(),
                                wm_class: class,
                            },
                        );
                    }
                }
            }
            Err(e) => warn!("no se pudo ejecutar xprop: {e}"),
        }
        tokio::time::sleep(RETRY).await;
    }
}

/// Segunda cadena de `WM_CLASS` (la clase; la primera es la instancia).
async fn x11_class(window: &str) -> Option<String> {
    let out = Command::new("xprop")
        .args(["-id", window, "WM_CLASS"])
        .output()
        .await
        .ok()?;
    parse_wm_class(&String::from_utf8_lossy(&out.stdout))
}

fn parse_wm_class(text: &str) -> Option<String> {
    // WM_CLASS(STRING) = "navigator", "firefox"
    let values = text.split_once('=')?.1;
    let parts: Vec<&str> = values
        .split(',')
        .map(|p| p.trim().trim_matches('"'))
        .collect();
    parts.get(1).or(parts.first()).map(|s| (*s).to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sway_and_i3_focus_events() {
        let sway = r#"{"change":"focus","container":{"app_id":"org.gnome.Nautilus","window_properties":null}}"#;
        let app = parse_i3_event(sway).unwrap();
        assert_eq!(app.app_id, "org.gnome.Nautilus");
        assert_eq!(app.wm_class, "org.gnome.Nautilus");

        let i3 = r#"{"change":"focus","container":{"window_properties":{"class":"firefox","instance":"Navigator"}}}"#;
        let app = parse_i3_event(i3).unwrap();
        assert_eq!(
            (app.app_id.as_str(), app.wm_class.as_str()),
            ("firefox", "firefox")
        );

        assert!(parse_i3_event(r#"{"change":"title","container":{}}"#).is_none());
    }

    #[test]
    fn parses_wm_class() {
        assert_eq!(
            parse_wm_class(r#"WM_CLASS(STRING) = "navigator", "firefox""#).as_deref(),
            Some("firefox")
        );
        assert_eq!(parse_wm_class("WM_CLASS:  not found."), None);
    }
}
