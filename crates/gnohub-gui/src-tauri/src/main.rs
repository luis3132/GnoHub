//! Interfaz gráfica de GnoHub.
//!
//! La interfaz no ejecuta macros: edita `config.toml` y el daemon (`gnohubd`)
//! lo recarga solo al detectar el cambio. Desde aquí también se instala y
//! controla el daemon como servicio de usuario de systemd.

// Traducciones de los mensajes del backend (inglés y español), en `locales/`.
rust_i18n::i18n!("locales", fallback = "en");

mod access;
mod apps;
mod definitions;
mod detect;
mod piper;
mod ratbag;
mod service;

use std::path::PathBuf;

use gnohub_core::devices::{self, DeviceSummary};
use gnohub_core::{Config, ConfigFile};
use rust_i18n::t;
use serde::Serialize;

fn config_path() -> Result<PathBuf, String> {
    directories::ProjectDirs::from("", "", "gnohub")
        .map(|d| d.config_dir().join("config.toml"))
        .ok_or_else(|| t!("common.no_home").into_owned())
}

#[derive(Serialize)]
struct LoadedConfig {
    path: String,
    exists: bool,
    file: ConfigFile,
}

#[tauri::command]
fn load_config() -> Result<LoadedConfig, String> {
    let path = config_path()?;
    let file = ConfigFile::load_or_default(&path).map_err(|e| e.to_string())?;
    Ok(LoadedConfig {
        path: path.display().to_string(),
        exists: path.exists(),
        file,
    })
}

/// Comprueba que la configuración sea ejecutable (macros existentes, retardos ≥ 1 ms…).
#[tauri::command]
fn validate_config(file: ConfigFile) -> Result<(), String> {
    Config::resolve(file).map(drop).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_config(file: ConfigFile) -> Result<(), String> {
    Config::resolve(file.clone()).map_err(|e| e.to_string())?;
    file.save(&config_path()?).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_devices() -> Vec<DeviceSummary> {
    devices::summaries()
}

/// Dispositivos que existen pero que GnoHub no puede leer.
#[tauri::command]
fn locked_devices() -> Vec<gnohub_core::devices::LockedDevice> {
    access::locked()
}

#[tauri::command]
async fn grant_access(vendor: u16, product: u16) -> Result<(), String> {
    access::grant(vendor, product).await
}

#[tauri::command]
fn installed_apps() -> Vec<apps::InstalledApp> {
    apps::installed()
}

#[tauri::command]
async fn focus_status() -> apps::FocusStatus {
    apps::focus_status().await
}

#[tauri::command]
async fn enable_gnome_extension() -> Result<apps::FocusStatus, String> {
    apps::enable_extension().await
}

/// DPI, frecuencia y botones del firmware, si ratbagd soporta el ratón.
#[tauri::command]
async fn hardware(vendor: u16, product: u16) -> Result<Option<ratbag::Hardware>, String> {
    ratbag::hardware(vendor, product).await
}

#[tauri::command]
async fn set_dpi(vendor: u16, product: u16, index: u32, dpi: u32) -> Result<(), String> {
    ratbag::set_dpi(vendor, product, index, dpi).await
}

#[tauri::command]
async fn set_report_rate(vendor: u16, product: u16, rate: u32) -> Result<(), String> {
    ratbag::set_report_rate(vendor, product, rate).await
}

#[tauri::command]
async fn take_over_button(vendor: u16, product: u16, index: u32) -> Result<String, String> {
    ratbag::take_over_button(vendor, product, index).await
}

/// Definiciones de periféricos (JSON) y los archivos que tenían errores.
#[tauri::command]
fn device_definitions() -> definitions::Loaded {
    definitions::load_all()
}

/// Devuelve al botón su función original (o `mapping`, si se indica).
#[tauri::command]
async fn restore_button(
    vendor: u16,
    product: u16,
    index: u32,
    mapping: Option<ratbag::Mapping>,
) -> Result<(), String> {
    ratbag::restore_button(vendor, product, index, mapping).await
}

#[tauri::command]
async fn set_led(
    vendor: u16,
    product: u16,
    index: u32,
    led: ratbag::LedSettings,
) -> Result<(), String> {
    ratbag::set_led(vendor, product, index, led).await
}

/// Dibujo interactivo del dispositivo según Piper, si está instalado.
#[tauri::command]
async fn piper_svg(vendor: u16, product: u16) -> Option<String> {
    piper::svg_for(vendor, product).await
}

#[tauri::command]
fn log_frontend_error(message: String) {
    eprintln!("[interfaz] {message}");
}

/// Idioma del sistema reducido a uno de los disponibles.
#[tauri::command]
fn system_locale() -> &'static str {
    gnohub_core::system_locale()
}

/// Idioma de los mensajes del backend; la interfaz lo fija al arrancar y al cambiarlo.
#[tauri::command]
fn set_locale(locale: String) {
    gnohub_core::set_locale(&locale);
}

#[tauri::command]
fn key_names() -> Vec<String> {
    gnohub_core::known_keys()
}

#[derive(Serialize)]
struct SystemStatus {
    /// Nodos de entrada que no se pueden leer (faltan permisos).
    unreadable: usize,
    uinput_writable: bool,
    service: service::ServiceStatus,
}

#[tauri::command]
async fn system_status() -> SystemStatus {
    SystemStatus {
        unreadable: devices::unreadable_count(),
        uinput_writable: std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/uinput")
            .is_ok(),
        service: service::status().await,
    }
}

#[tauri::command]
async fn service_action(action: service::Action) -> Result<service::ServiceStatus, String> {
    service::run(action).await?;
    Ok(service::status().await)
}

/// Espera a que se pulse un botón del dispositivo y devuelve su nombre.
///
/// El daemon captura los dispositivos en exclusiva, así que si el servicio
/// está activo se detiene durante la detección y se vuelve a arrancar después.
#[tauri::command]
async fn detect_button(
    vendor: u16,
    product: u16,
    timeout_ms: u64,
) -> Result<Option<String>, String> {
    let was_active = service::status().await.active;
    if was_active {
        service::run(service::Action::Stop).await?;
    }
    let result = detect::first_press(
        vendor,
        product,
        std::time::Duration::from_millis(timeout_ms),
    )
    .await;
    if was_active {
        service::run(service::Action::Start).await?;
    }
    match result? {
        None => Ok(None),
        Some(button) => button
            .name()
            .map(Some)
            .ok_or_else(|| t!("detect.unnamed", button = button).into_owned()),
    }
}

fn main() {
    gnohub_core::set_locale(gnohub_core::system_locale());
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_config,
            validate_config,
            save_config,
            list_devices,
            locked_devices,
            grant_access,
            installed_apps,
            hardware,
            device_definitions,
            set_dpi,
            set_report_rate,
            take_over_button,
            restore_button,
            set_led,
            piper_svg,
            focus_status,
            enable_gnome_extension,
            log_frontend_error,
            system_locale,
            set_locale,
            key_names,
            system_status,
            service_action,
            detect_button,
        ])
        .build(tauri::generate_context!())
        .expect("error al iniciar GnoHub")
        .run(|_, event| {
            // El daemon que arrancó la app (sin servicio instalado) se va con ella.
            if let tauri::RunEvent::Exit = event {
                service::stop_embedded();
            }
        });
}

#[cfg(test)]
mod dump {
    /// Vuelca lo que devuelve el backend real a `dev-assets/real.json`, para
    /// reproducir en la vista previa del navegador lo que ve la app:
    /// `cargo test -p gnohub-gui -- --ignored dump_real_backend`.
    #[tokio::test]
    #[ignore]
    async fn dump_real_backend() {
        let devices = super::list_devices();
        let mut hardware = serde_json::Map::new();
        let mut piper = serde_json::Map::new();
        for d in &devices {
            let key = format!("{:04x}:{:04x}", d.vendor, d.product);
            let hw = super::ratbag::hardware(d.vendor, d.product)
                .await
                .ok()
                .flatten();
            hardware.insert(key.clone(), serde_json::to_value(hw).unwrap());
            piper.insert(
                key,
                serde_json::to_value(super::piper::svg_for(d.vendor, d.product).await).unwrap(),
            );
        }
        let all = serde_json::json!({
            "load_config": super::load_config().unwrap(),
            "list_devices": devices,
            "locked_devices": super::locked_devices(),
            "device_definitions": super::device_definitions(),
            "installed_apps": super::installed_apps(),
            "focus_status": super::focus_status().await,
            "system_status": super::system_status().await,
            "hardware": hardware,
            "piper_svg": piper,
        });
        let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../dev-assets/real.json");
        std::fs::write(&out, serde_json::to_string_pretty(&all).unwrap()).unwrap();
        println!("{}", out.display());
    }
}
