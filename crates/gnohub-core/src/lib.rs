//! Núcleo de GnoHub: modelo de macros, formato de configuración y
//! descubrimiento de dispositivos. Lo comparten el daemon y la interfaz.

// Traducciones de los mensajes (inglés y español), en `locales/`.
rust_i18n::i18n!("locales", fallback = "en");

pub mod action;
pub mod button;
pub mod config;
pub mod devices;
pub mod file;
pub mod process;

pub use action::{Action, Step};
pub use button::{Button, Wheel};
pub use config::{ActiveApp, AppProfile, Binding, Config, ConfigError, DeviceConfig, Macro};
pub use file::{
    AppProfileDef, BindingDef, ConfigFile, DeviceDef, MacroDef, MacroRef, key_name, known_keys,
    parse_key,
};

/// Idioma de los mensajes ("en", "es"…). Es global: afecta a todo el proceso.
pub fn set_locale(locale: &str) {
    rust_i18n::set_locale(locale);
}

/// Idioma del sistema reducido a uno de los disponibles (inglés por defecto).
pub fn system_locale() -> &'static str {
    match sys_locale::get_locale() {
        Some(l) if l.to_ascii_lowercase().starts_with("es") => "es",
        _ => "en",
    }
}
