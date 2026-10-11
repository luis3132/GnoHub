//! Concede acceso a dispositivos de entrada instalando reglas de udev con
//! `pkexec` (pide la contraseña con el diálogo gráfico del sistema).

use gnohub_core::devices::{self, LockedDevice};
use tokio::process::Command;

const RULES_FILE: &str = "/etc/udev/rules.d/70-gnohub.rules";
const UINPUT_RULE: &str =
    "KERNEL==\"uinput\", SUBSYSTEM==\"misc\", TAG+=\"uaccess\", OPTIONS+=\"static_node=uinput\"\n";

/// Script que se ejecuta como root: añade `$1` al archivo de reglas y las
/// aplica. El contenido llega como argumento, nunca interpolado en el script.
const SCRIPT: &str = r#"printf '%s' "$1" >> /etc/udev/rules.d/70-gnohub.rules && udevadm control --reload && udevadm trigger --subsystem-match=input --subsystem-match=misc --action=change && udevadm settle"#;

pub fn locked() -> Vec<LockedDevice> {
    devices::locked_devices()
}

pub async fn grant(vendor: u16, product: u16) -> Result<(), String> {
    let device = devices::locked_devices()
        .into_iter()
        .find(|d| d.vendor == vendor && d.product == product)
        .ok_or_else(|| rust_i18n::t!("access.already_granted").into_owned())?;

    let existing = std::fs::read_to_string(RULES_FILE).unwrap_or_default();
    let mut rules = devices::udev_rules_for(&device);
    let uinput_ok = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/uinput")
        .is_ok();
    if !uinput_ok && !existing.contains("uinput") {
        rules.push_str(UINPUT_RULE);
    }
    if existing.is_empty() {
        rules.insert_str(0, "# Reglas creadas por GnoHub: acceso del usuario con sesión activa a estos dispositivos.\n");
    }

    let out = Command::new("pkexec")
        .args(["sh", "-c", SCRIPT, "sh", &rules])
        .output()
        .await
        .map_err(|e| rust_i18n::t!("access.pkexec_failed", error = e).into_owned())?;
    match out.status.code() {
        Some(0) => Ok(()),
        // 126: el usuario canceló el diálogo; 127: no autorizado.
        Some(126 | 127) => Err(rust_i18n::t!("access.denied").into_owned()),
        _ => Err(String::from_utf8_lossy(&out.stderr).trim().to_owned()),
    }
}
