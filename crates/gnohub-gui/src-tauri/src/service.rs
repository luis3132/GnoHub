//! Control del daemon.
//!
//! Con el paquete instalado, `gnohubd` es un servicio de usuario de systemd
//! (`gnohubd.service`) y sigue funcionando con la ventana cerrada. Sin
//! instalar (p. ej. ejecutando la app desde `target/`), la app lo arranca ella
//! misma como proceso hijo mientras está abierta: así las asignaciones
//! funcionan igualmente.

use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tokio::process::Command;

const UNIT: &str = "gnohubd.service";

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Lo habilita al iniciar sesión y lo arranca.
    Install,
    Start,
    Stop,
    Restart,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceStatus {
    /// El servicio de systemd está instalado (viene con el paquete).
    pub installed: bool,
    /// El daemon está en marcha (como servicio o, sin instalar, dentro de la app).
    pub active: bool,
    pub enabled: bool,
    /// Lo ejecuta la propia app porque el servicio no está instalado.
    pub embedded: bool,
    /// Ruta del ejecutable `gnohubd` que se usaría al instalar.
    pub daemon_path: Option<String>,
}

async fn systemctl(args: &[&str]) -> Result<String, String> {
    let out = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .await
        .map_err(|e| rust_i18n::t!("service.systemctl_failed", error = e).into_owned())?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    if out.status.success() {
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_owned();
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}

/// Ruta del daemon: junto al ejecutable de la app (instalado en `/usr/bin` o
/// compilado en `target/`), o en el `PATH`.
fn daemon_path() -> Option<PathBuf> {
    let sibling = std::env::current_exe()
        .ok()
        .map(|exe| exe.with_file_name("gnohubd"));
    sibling.filter(|p| p.is_file()).or_else(|| {
        std::env::var_os("PATH").and_then(|paths| {
            std::env::split_paths(&paths)
                .map(|dir| dir.join("gnohubd"))
                .find(|p| p.is_file())
        })
    })
}

/// Daemon arrancado por la app cuando el servicio no está instalado.
static EMBEDDED: Mutex<Option<Child>> = Mutex::new(None);

fn embedded_running() -> bool {
    let mut guard = EMBEDDED.lock().unwrap_or_else(|e| e.into_inner());
    match guard.as_mut().map(Child::try_wait) {
        Some(Ok(None)) => true,
        Some(_) => {
            *guard = None;
            false
        }
        None => false,
    }
}

fn start_embedded() -> Result<(), String> {
    if embedded_running() {
        return Ok(());
    }
    let path = daemon_path().ok_or_else(|| rust_i18n::t!("service.daemon_missing").into_owned())?;
    let child = std::process::Command::new(path)
        .arg("run")
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| rust_i18n::t!("service.embedded_failed", error = e).into_owned())?;
    *EMBEDDED.lock().unwrap_or_else(|e| e.into_inner()) = Some(child);
    Ok(())
}

/// Detiene el daemon arrancado por la app (al pararlo o al cerrar la app).
/// Con SIGTERM el daemon suelta los dispositivos de forma ordenada.
pub fn stop_embedded() {
    let child = EMBEDDED.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some(mut child) = child {
        let _ = std::process::Command::new("kill")
            .arg(child.id().to_string())
            .status();
        for _ in 0..30 {
            if matches!(child.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let _ = child.kill();
        let _ = child.wait();
    }
}

pub async fn status() -> ServiceStatus {
    let load = systemctl(&["show", UNIT, "--property=LoadState", "--value"])
        .await
        .unwrap_or_default();
    // `is-active` / `is-enabled` salen con error cuando la respuesta es "no",
    // pero igualmente imprimen el estado.
    let state = |args: [&'static str; 2]| async move {
        match systemctl(&args).await {
            Ok(s) | Err(s) => s,
        }
    };
    let installed = load == "loaded";
    let embedded = !installed && embedded_running();
    ServiceStatus {
        installed,
        active: embedded || (installed && state(["is-active", UNIT]).await == "active"),
        enabled: installed && state(["is-enabled", UNIT]).await == "enabled",
        embedded,
        daemon_path: daemon_path().map(|p| p.display().to_string()),
    }
}

pub async fn run(action: Action) -> Result<(), String> {
    if !status().await.installed {
        // Sin servicio instalado, la app gestiona su propio daemon.
        match action {
            Action::Install | Action::Start => start_embedded()?,
            Action::Restart => {
                stop_embedded();
                start_embedded()?;
            }
            Action::Stop => stop_embedded(),
        }
        return Ok(());
    }
    match action {
        Action::Install => {
            // El paquete instala la unidad; aquí solo se activa para este usuario.
            systemctl(&["enable", "--now", UNIT]).await?;
        }
        Action::Start => {
            systemctl(&["start", UNIT]).await?;
        }
        Action::Stop => {
            systemctl(&["stop", UNIT]).await?;
        }
        Action::Restart => {
            systemctl(&["restart", UNIT]).await?;
        }
    }
    Ok(())
}
