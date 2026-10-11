//! Descubrimiento de dispositivos de entrada físicos, compartido por el daemon
//! y la interfaz gráfica.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use evdev::{Device, KeyCode, RelativeAxisCode};
use serde::Serialize;

use crate::button::Wheel;
use crate::file::key_name;

/// Prefijo de los dispositivos virtuales que crea GnoHub. Nunca se capturan,
/// para no entrar en un bucle leyendo nuestra propia salida.
pub const VIRTUAL_PREFIX: &str = "GnoHub";

/// Rango de códigos de "botones" (BTN_0 … BTN_THUMBR): ratón, joystick y mando.
/// Los teclados declaran cientos de teclas, así que esas no se listan.
const BUTTON_RANGE: std::ops::RangeInclusive<u16> = 0x100..=0x13e;

pub fn is_ours(device: &Device) -> bool {
    device.name().is_some_and(|n| n.starts_with(VIRTUAL_PREFIX))
}

/// Dispositivos físicos que se pueden abrir, ordenados por ruta.
pub fn physical_devices() -> Vec<(PathBuf, Device)> {
    let mut devices: Vec<_> = evdev::enumerate().filter(|(_, d)| !is_ours(d)).collect();
    devices.sort_by(|a, b| a.0.cmp(&b.0));
    devices
}

/// Nombres de los nodos `eventN` presentes. Leer el directorio es barato, así
/// que sirve para detectar conexiones sin abrir cada dispositivo.
pub fn event_nodes() -> BTreeSet<String> {
    std::fs::read_dir("/dev/input")
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("event"))
        .collect()
}

/// Nodos `eventN` que existen pero no se pueden abrir (normalmente por permisos).
pub fn unreadable_count() -> usize {
    event_nodes()
        .iter()
        .filter(|n| std::fs::File::open(format!("/dev/input/{n}")).is_err())
        .count()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    Mouse,
    Keyboard,
    Other,
}

pub fn kind_of(device: &Device) -> DeviceKind {
    if device
        .supported_relative_axes()
        .is_some_and(|r| r.contains(RelativeAxisCode::REL_X))
    {
        DeviceKind::Mouse
    } else if device
        .supported_keys()
        .is_some_and(|k| k.contains(KeyCode::KEY_A))
    {
        DeviceKind::Keyboard
    } else {
        DeviceKind::Other
    }
}

/// Un dispositivo físico, agrupando todos sus nodos (un G502 expone uno de
/// ratón y otro de teclado para los botones G).
#[derive(Debug, Clone, Serialize)]
pub struct DeviceSummary {
    pub name: String,
    pub vendor: u16,
    pub product: u16,
    pub kind: DeviceKind,
    pub nodes: Vec<PathBuf>,
    /// Botones (no teclas) que declara alguno de sus nodos, por nombre.
    pub buttons: Vec<String>,
}

pub fn summaries() -> Vec<DeviceSummary> {
    let mut groups: BTreeMap<(u16, u16), DeviceSummary> = BTreeMap::new();
    for (path, device) in physical_devices() {
        let id = device.input_id();
        let name = device
            .name()
            .unwrap_or("Dispositivo desconocido")
            .to_owned();
        let kind = kind_of(&device);
        let entry = groups
            .entry((id.vendor(), id.product()))
            .or_insert_with(|| DeviceSummary {
                name: name.clone(),
                vendor: id.vendor(),
                product: id.product(),
                kind,
                nodes: Vec::new(),
                buttons: Vec::new(),
            });
        // El nombre más corto suele ser el principal ("… Mouse" frente a "… Mouse Keyboard").
        if name.len() < entry.name.len() {
            entry.name = name;
        }
        if kind == DeviceKind::Mouse
            || (kind == DeviceKind::Keyboard && entry.kind == DeviceKind::Other)
        {
            entry.kind = kind;
        }
        entry.nodes.push(path);
        if let Some(axes) = device.supported_relative_axes() {
            for wheel in Wheel::ALL {
                if axes.contains(wheel.axes()[0]) {
                    entry.buttons.push(wheel.name().to_owned());
                }
            }
        }
        if let Some(keys) = device.supported_keys() {
            entry.buttons.extend(
                keys.iter()
                    .filter(|k| BUTTON_RANGE.contains(&k.code()))
                    .filter_map(key_name),
            );
        }
    }
    groups
        .into_values()
        .map(|mut d| {
            // Sin repetidos (varios nodos declaran los mismos botones o ejes);
            // primero los botones por código y al final las direcciones de rueda.
            let mut seen = std::collections::HashSet::new();
            d.buttons.retain(|name| seen.insert(name.clone()));
            d.buttons
                .sort_by_key(|name| name.parse::<KeyCode>().map_or(u16::MAX, |k| k.code()));
            d
        })
        .collect()
}

/// Un dispositivo cuyos nodos no se pueden abrir. Sus datos salen de `/sys`,
/// que es legible sin permisos, para poder ofrecer darle acceso.
#[derive(Debug, Clone, Serialize)]
pub struct LockedDevice {
    pub name: String,
    pub vendor: u16,
    pub product: u16,
    pub kind: DeviceKind,
    /// Nombres de todos sus nodos `eventN` (para las reglas de udev).
    pub node_names: Vec<String>,
}

/// ¿Está activo el bit `bit` en un mapa de capacidades de `/sys`? Son palabras
/// hexadecimales separadas por espacios, la más significativa primero.
fn has_capability(map: &str, bit: usize) -> bool {
    let words: Vec<&str> = map.split_whitespace().rev().collect();
    let word_bits = usize::BITS as usize;
    words
        .get(bit / word_bits)
        .and_then(|w| u64::from_str_radix(w, 16).ok())
        .is_some_and(|w| w >> (bit % word_bits) & 1 == 1)
}

fn sys_kind(node: &str) -> DeviceKind {
    let caps = |name| sys_attr(node, &format!("capabilities/{name}")).unwrap_or_default();
    if has_capability(&caps("rel"), RelativeAxisCode::REL_X.0 as usize) {
        DeviceKind::Mouse
    } else if has_capability(&caps("key"), KeyCode::KEY_A.code() as usize) {
        DeviceKind::Keyboard
    } else {
        DeviceKind::Other
    }
}

fn sys_attr(node: &str, attr: &str) -> Option<String> {
    std::fs::read_to_string(format!("/sys/class/input/{node}/device/{attr}"))
        .ok()
        .map(|s| s.trim().to_owned())
}

/// Dispositivos de entrada que existen pero no se pueden leer, agrupados por
/// VID:PID. Excluye los virtuales (los de GnoHub y los de bus 0, como uinput).
pub fn locked_devices() -> Vec<LockedDevice> {
    let mut groups: BTreeMap<(u16, u16), LockedDevice> = BTreeMap::new();
    for node in event_nodes() {
        if std::fs::File::open(format!("/dev/input/{node}")).is_ok() {
            continue;
        }
        let Some(name) = sys_attr(&node, "name") else {
            continue;
        };
        let hex = |attr| sys_attr(&node, attr).and_then(|v| u16::from_str_radix(&v, 16).ok());
        let (Some(vendor), Some(product), Some(bus)) =
            (hex("id/vendor"), hex("id/product"), hex("id/bustype"))
        else {
            continue;
        };
        if bus == 0 || name.starts_with(VIRTUAL_PREFIX) || (vendor == 0 && product == 0) {
            continue;
        }
        let kind = sys_kind(&node);
        let entry = groups
            .entry((vendor, product))
            .or_insert_with(|| LockedDevice {
                name: name.clone(),
                vendor,
                product,
                kind,
                node_names: Vec::new(),
            });
        if kind == DeviceKind::Mouse
            || (kind == DeviceKind::Keyboard && entry.kind == DeviceKind::Other)
        {
            entry.kind = kind;
        }
        if name.len() < entry.name.len() {
            entry.name = name.clone();
        }
        if !entry.node_names.contains(&name) {
            entry.node_names.push(name);
        }
    }
    groups.into_values().collect()
}

/// Reglas de udev que dan acceso al usuario con sesión activa a un
/// dispositivo, identificado por el nombre de cada uno de sus nodos (funciona
/// igual para USB y Bluetooth).
pub fn udev_rules_for(device: &LockedDevice) -> String {
    let mut rules = format!(
        "# {} ({:04x}:{:04x})\n",
        device.name, device.vendor, device.product
    );
    for name in &device.node_names {
        // Comillas y comodines de udev se sustituyen por `?` para que la regla
        // siga siendo válida y no pueda coincidir con más de lo debido.
        let safe: String = name
            .chars()
            .map(|c| {
                if matches!(c, '"' | '\\' | '*' | '[' | ']' | '|') || c.is_control() {
                    '?'
                } else {
                    c
                }
            })
            .collect();
        rules.push_str(&format!(
            "SUBSYSTEM==\"input\", KERNEL==\"event*\", ATTRS{{name}}==\"{safe}\", TAG+=\"uaccess\"\n"
        ));
    }
    rules
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_capability_bitmaps() {
        // REL_X es el bit 0; KEY_A, el 30 de la última palabra.
        assert!(has_capability("1943", 0));
        assert!(!has_capability("1942", 0));
        assert!(has_capability("ffff0000 0 0 0 40000000", 30));
        assert!(!has_capability("ffff0000 0 0 0 0", 30));
        assert!(has_capability("1 0", 64));
    }

    #[test]
    fn udev_rules_escape_quotes_and_globs() {
        let device = LockedDevice {
            name: "Raro \"*\" Mouse".into(),
            vendor: 0x1234,
            product: 0xabcd,
            kind: DeviceKind::Mouse,
            node_names: vec!["Raro \"*\" Mouse".into(), "Raro Keyboard".into()],
        };
        let rules = udev_rules_for(&device);
        assert!(
            rules.contains(r#"ATTRS{name}=="Raro ??? Mouse""#),
            "{rules}"
        );
        assert!(
            rules.contains(r#"ATTRS{name}=="Raro Keyboard", TAG+="uaccess""#),
            "{rules}"
        );
        assert_eq!(rules.lines().count(), 3);
    }
}
