//! Configuración del hardware (DPI, frecuencia de sondeo, LED y botones de la
//! memoria interna) a través de `ratbagd`, el servicio de libratbag que usa
//! Piper. Soporta ratones de Logitech, Razer, SteelSeries, Roccat y otros.
//!
//! API: <https://libratbag.github.io/dbus.html> (bus del sistema).

use std::collections::BTreeMap;
use std::path::PathBuf;

use evdev::KeyCode;
use gnohub_core::key_name;
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};
use zbus::{Connection, Proxy};

const SERVICE: &str = "org.freedesktop.ratbag1";
const MANAGER_PATH: &str = "/org/freedesktop/ratbag1";

/// Tipos de evento de las macros de libratbag (`RATBAG_MACRO_EVENT_*`).
const MACRO_KEY_PRESSED: u32 = 1;
const MACRO_KEY_RELEASED: u32 = 2;

/// Base de los códigos "especiales" de libratbag (`RATBAG_BUTTON_ACTION_SPECIAL_*`).
pub const SPECIAL_BASE: u32 = 1 << 30;

/// Teclas que GnoHub programa en los botones del firmware: nadie las usa y
/// los teclados USB las pueden enviar.
const FREE_KEYS: [KeyCode; 12] = [
    KeyCode::KEY_F13,
    KeyCode::KEY_F14,
    KeyCode::KEY_F15,
    KeyCode::KEY_F16,
    KeyCode::KEY_F17,
    KeyCode::KEY_F18,
    KeyCode::KEY_F19,
    KeyCode::KEY_F20,
    KeyCode::KEY_F21,
    KeyCode::KEY_F22,
    KeyCode::KEY_F23,
    KeyCode::KEY_F24,
];

/// Función de un botón en la memoria del ratón (`Mapping` de libratbag).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Mapping {
    None,
    /// Botón de ratón N (1 izquierdo, 2 derecho, 3 central, 4 atrás, 5 adelante…).
    Button {
        n: u32,
    },
    /// Función especial (`SPECIAL_BASE + n`): DPI, perfiles, rueda…
    Special {
        code: u32,
    },
    /// Tecla (código de `linux/input-event-codes.h`).
    Key {
        code: u32,
    },
    /// Macro guardada en el ratón: pares (tipo, tecla).
    Macro {
        events: Vec<(u32, u32)>,
    },
    Unknown {
        raw_kind: u32,
    },
}

impl Mapping {
    fn from_dbus(kind: u32, value: &Value<'_>) -> Self {
        match (kind, u32_of(value)) {
            (0, _) => Mapping::None,
            (1, Some(n)) => Mapping::Button { n },
            (2, Some(code)) => Mapping::Special { code },
            (3, Some(code)) => Mapping::Key { code },
            (4, _) => Mapping::Macro {
                events: macro_events(value).unwrap_or_default(),
            },
            _ => Mapping::Unknown { raw_kind: kind },
        }
    }

    fn to_dbus(&self) -> Option<Value<'static>> {
        let (kind, inner) = match self {
            Mapping::None => (0u32, Value::U32(0)),
            Mapping::Button { n } => (1, Value::U32(*n)),
            Mapping::Special { code } => (2, Value::U32(*code)),
            Mapping::Key { code } => (3, Value::U32(*code)),
            Mapping::Macro { events } => (4, Value::from(events.clone())),
            Mapping::Unknown { .. } => return None,
        };
        Some(Value::from((kind, inner)))
    }

    /// Tecla que envía, si envía una sola. Algunos ratones (los Logitech con
    /// HID++ 2.0) guardan "pulsar la tecla X" como una macro de dos eventos,
    /// pulsar X y soltar X, y así se lee de vuelta: se trata igual que la tecla.
    fn effective_key(&self) -> Option<u32> {
        match self {
            Mapping::Key { code } => Some(*code),
            Mapping::Macro { events } => match events.as_slice() {
                [(MACRO_KEY_PRESSED, a), (MACRO_KEY_RELEASED, b)] if a == b => Some(*a),
                _ => None,
            },
            _ => None,
        }
    }

    /// Evento que GnoHub recibirá de un botón con esta función, si es asignable.
    fn emits(&self) -> Option<String> {
        if let Some(code) = self.effective_key() {
            return key_name(KeyCode::new(code as u16));
        }
        match self {
            // Botón N: 1 izquierdo … 8 BTN_TASK; 9–16 son BTN_MOUSE_9…16.
            Mapping::Button { n } if (1..=16).contains(n) => {
                key_name(KeyCode::new(KeyCode::BTN_LEFT.code() + *n as u16 - 1))
            }
            Mapping::Special { code } => match code.wrapping_sub(SPECIAL_BASE) {
                2 => Some("WHEEL_LEFT".into()),
                3 => Some("WHEEL_RIGHT".into()),
                4 => Some("WHEEL_UP".into()),
                5 => Some("WHEEL_DOWN".into()),
                _ => None,
            },
            _ => None,
        }
    }

    /// ¿Lo reprogramó GnoHub (envía una de sus teclas libres)?
    fn is_gnohub_key(&self) -> bool {
        self.effective_key()
            .is_some_and(|code| FREE_KEYS.iter().any(|k| u32::from(k.code()) == code))
    }
}

fn macro_events(value: &Value<'_>) -> Option<Vec<(u32, u32)>> {
    match value {
        Value::Value(inner) => macro_events(inner),
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::Structure(s) => match s.fields() {
                    [a, b] => Some((u32_of(a)?, u32_of(b)?)),
                    _ => None,
                },
                _ => None,
            })
            .collect(),
        _ => None,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Hardware {
    pub name: String,
    pub report_rate: u32,
    pub report_rates: Vec<u32>,
    pub resolutions: Vec<Resolution>,
    /// DPI que admite el sensor.
    pub dpi_values: Vec<u32>,
    pub buttons: Vec<HwButton>,
    pub leds: Vec<Led>,
    /// Índice del perfil interno activo y cuántos hay.
    pub profile: u32,
    pub profiles: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Resolution {
    pub index: u32,
    pub dpi: u32,
    pub active: bool,
    pub default: bool,
    pub disabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Led {
    pub index: u32,
    /// 0 apagado, 1 fijo, 2 ciclo de colores, 3 respiración.
    pub mode: u32,
    pub modes: Vec<u32>,
    pub color: [u32; 3],
    /// 0 sin color (solo encendido/apagado), 1 RGB de 8 bits, 2 RGB de 1 bit.
    pub color_depth: u32,
    /// Duración de un ciclo del efecto, en ms.
    pub duration: u32,
    /// 0–255.
    pub brightness: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct HwButton {
    pub index: u32,
    /// Qué evento envía al sistema (`BTN_SIDE`, `KEY_F13`, `WHEEL_LEFT`…), si
    /// envía uno que GnoHub pueda asignar. `None` si lo gestiona el firmware.
    pub emits: Option<String>,
    /// Lo que hace ahora; la interfaz lo describe en el idioma elegido.
    pub mapping: Mapping,
    /// El botón lo gestiona el firmware (DPI, perfiles…) y se puede pasar a GnoHub.
    pub firmware: bool,
    /// GnoHub lo reprogramó con una de sus teclas libres.
    pub taken_over: bool,
    /// Hay copia de su función original para restaurarla.
    pub has_backup: bool,
}

async fn system() -> Result<Connection, String> {
    Connection::system()
        .await
        .map_err(|e| t!("ratbag.system_bus", error = e.to_string()).into_owned())
}

async fn proxy<'a>(
    conn: &Connection,
    path: &'a str,
    iface: &'static str,
) -> Result<Proxy<'a>, String> {
    Proxy::new(conn, SERVICE, path, iface)
        .await
        .map_err(|e| e.to_string())
}

async fn get<T>(p: &Proxy<'_>, name: &str) -> Result<T, String>
where
    T: TryFrom<OwnedValue>,
    T::Error: Into<zbus::Error>,
{
    p.get_property::<T>(name)
        .await
        .map_err(|e| format!("ratbagd ({name}): {e}"))
}

/// Ruta del dispositivo de ratbagd con este VID:PID, si lo soporta.
async fn find_device(
    conn: &Connection,
    vendor: u16,
    product: u16,
) -> Result<Option<String>, String> {
    let manager = match proxy(conn, MANAGER_PATH, "org.freedesktop.ratbag1.Manager").await {
        Ok(p) => p,
        Err(_) => return Ok(None),
    };
    let Ok(devices) = manager
        .get_property::<Vec<OwnedObjectPath>>("Devices")
        .await
    else {
        // ratbagd no está instalado o no arrancó.
        return Ok(None);
    };
    let suffix = format!(":{vendor:04x}:{product:04x}:");
    for path in devices {
        let device = proxy(conn, path.as_str(), "org.freedesktop.ratbag1.Device").await?;
        let model: String = get(&device, "Model").await?;
        if model.contains(&suffix) {
            return Ok(Some(path.to_string()));
        }
    }
    Ok(None)
}

async fn profiles(conn: &Connection, device: &str) -> Result<Vec<OwnedObjectPath>, String> {
    let device = proxy(conn, device, "org.freedesktop.ratbag1.Device").await?;
    get(&device, "Profiles").await
}

async fn active_profile(conn: &Connection, device: &str) -> Result<(u32, String, usize), String> {
    let all = profiles(conn, device).await?;
    for (i, path) in all.iter().enumerate() {
        let p = proxy(conn, path.as_str(), "org.freedesktop.ratbag1.Profile").await?;
        if get::<bool>(&p, "IsActive").await? {
            return Ok((i as u32, path.to_string(), all.len()));
        }
    }
    let first = all
        .first()
        .ok_or_else(|| t!("ratbag.no_profiles").into_owned())?;
    Ok((0, first.to_string(), all.len()))
}

/// Botones de un perfil: (ruta, índice, función).
async fn buttons_of(
    conn: &Connection,
    profile: &str,
) -> Result<Vec<(String, u32, Mapping)>, String> {
    let p = proxy(conn, profile, "org.freedesktop.ratbag1.Profile").await?;
    let paths: Vec<OwnedObjectPath> = get(&p, "Buttons").await?;
    let mut out = Vec::new();
    for path in paths {
        let b = proxy(conn, path.as_str(), "org.freedesktop.ratbag1.Button").await?;
        let index: u32 = get(&b, "Index").await?;
        let (kind, value): (u32, OwnedValue) = get(&b, "Mapping").await?;
        out.push((path.to_string(), index, Mapping::from_dbus(kind, &value)));
    }
    Ok(out)
}

pub async fn hardware(vendor: u16, product: u16) -> Result<Option<Hardware>, String> {
    let conn = system().await?;
    let Some(device_path) = find_device(&conn, vendor, product).await? else {
        return Ok(None);
    };
    let device = proxy(&conn, &device_path, "org.freedesktop.ratbag1.Device").await?;
    let name: String = get(&device, "Name").await?;
    let (profile_index, profile_path, profile_count) = active_profile(&conn, &device_path).await?;
    let profile = proxy(&conn, &profile_path, "org.freedesktop.ratbag1.Profile").await?;

    let mut resolutions = Vec::new();
    let mut dpi_values = Vec::new();
    let res_paths: Vec<OwnedObjectPath> = get(&profile, "Resolutions").await?;
    for path in res_paths {
        let r = proxy(&conn, path.as_str(), "org.freedesktop.ratbag1.Resolution").await?;
        let value: OwnedValue = get(&r, "Resolution").await?;
        if dpi_values.is_empty() {
            dpi_values = get(&r, "Resolutions").await.unwrap_or_default();
        }
        resolutions.push(Resolution {
            index: get(&r, "Index").await?,
            dpi: dpi_of(&value).unwrap_or(0),
            active: get(&r, "IsActive").await?,
            default: get(&r, "IsDefault").await?,
            disabled: get(&r, "IsDisabled").await.unwrap_or(false),
        });
    }

    let backups = load_backups();
    let device_backups = backups.get(&device_key(vendor, product));
    let mut buttons: Vec<HwButton> = buttons_of(&conn, &profile_path)
        .await?
        .into_iter()
        .map(|(_, index, mapping)| {
            let emits = mapping.emits();
            let has_backup = device_backups.is_some_and(|b| b.contains_key(&index));
            HwButton {
                index,
                firmware: emits.is_none(),
                // Reprogramado por GnoHub: tiene copia de su función original,
                // o envía una de las teclas libres (versiones anteriores).
                taken_over: has_backup || mapping.is_gnohub_key(),
                has_backup,
                emits,
                mapping,
            }
        })
        .collect();
    buttons.sort_by_key(|b| b.index);

    let mut leds = Vec::new();
    let led_paths: Vec<OwnedObjectPath> = get(&profile, "Leds").await.unwrap_or_default();
    for path in led_paths {
        let l = proxy(&conn, path.as_str(), "org.freedesktop.ratbag1.Led").await?;
        let (r, g, b): (u32, u32, u32) = get(&l, "Color").await?;
        leds.push(Led {
            index: get(&l, "Index").await?,
            mode: get(&l, "Mode").await?,
            modes: get(&l, "Modes").await.unwrap_or_default(),
            color: [r, g, b],
            color_depth: get(&l, "ColorDepth").await.unwrap_or(1),
            duration: get(&l, "EffectDuration").await.unwrap_or(0),
            brightness: get(&l, "Brightness").await.unwrap_or(255),
        });
    }
    leds.sort_by_key(|l| l.index);

    Ok(Some(Hardware {
        name,
        report_rate: get(&profile, "ReportRate").await?,
        report_rates: get(&profile, "ReportRates").await.unwrap_or_default(),
        resolutions,
        dpi_values,
        buttons,
        leds,
        profile: profile_index,
        profiles: profile_count,
    }))
}

/// Entero de una variante de D-Bus, que puede venir envuelta en otra.
fn u32_of(value: &Value<'_>) -> Option<u32> {
    match value {
        Value::Value(inner) => u32_of(inner),
        Value::U32(n) => Some(*n),
        _ => None,
    }
}

/// DPI de la propiedad `Resolution`: una variante con `u`, o `(uu)` en
/// sensores con DPI distinto por eje (se usa el de X).
fn dpi_of(value: &Value<'_>) -> Option<u32> {
    match value {
        Value::Value(inner) => dpi_of(inner),
        Value::U32(dpi) => Some(*dpi),
        Value::Structure(s) => s.fields().first().and_then(u32_of),
        _ => None,
    }
}

async fn commit(conn: &Connection, device: &str) -> Result<(), String> {
    let device = proxy(conn, device, "org.freedesktop.ratbag1.Device").await?;
    let code: u32 = device
        .call("Commit", &())
        .await
        .map_err(|e| e.to_string())?;
    if code == 0 {
        Ok(())
    } else {
        Err(t!("ratbag.commit_rejected", code = code).into_owned())
    }
}

async fn device_or_err(conn: &Connection, vendor: u16, product: u16) -> Result<String, String> {
    find_device(conn, vendor, product)
        .await?
        .ok_or_else(|| t!("ratbag.not_supported").into_owned())
}

pub async fn set_dpi(vendor: u16, product: u16, index: u32, dpi: u32) -> Result<(), String> {
    let conn = system().await?;
    let device = device_or_err(&conn, vendor, product).await?;
    let (_, profile, _) = active_profile(&conn, &device).await?;
    let p = proxy(&conn, &profile, "org.freedesktop.ratbag1.Profile").await?;
    let paths: Vec<OwnedObjectPath> = get(&p, "Resolutions").await?;
    let path = paths
        .get(index as usize)
        .ok_or_else(|| t!("ratbag.no_dpi_level").into_owned())?;
    let r = proxy(&conn, path.as_str(), "org.freedesktop.ratbag1.Resolution").await?;
    // `Resolution` es de tipo `v` (una variante con `u` o `(uu)` dentro), así
    // que el valor va envuelto en otra variante.
    r.set_property("Resolution", Value::Value(Box::new(Value::U32(dpi))))
        .await
        .map_err(|e| e.to_string())?;
    commit(&conn, &device).await
}

pub async fn set_report_rate(vendor: u16, product: u16, rate: u32) -> Result<(), String> {
    let conn = system().await?;
    let device = device_or_err(&conn, vendor, product).await?;
    let (_, profile, _) = active_profile(&conn, &device).await?;
    let p = proxy(&conn, &profile, "org.freedesktop.ratbag1.Profile").await?;
    p.set_property("ReportRate", rate)
        .await
        .map_err(|e| e.to_string())?;
    commit(&conn, &device).await
}

#[derive(Debug, Clone, Deserialize)]
pub struct LedSettings {
    pub mode: u32,
    pub color: [u32; 3],
    pub duration: u32,
    pub brightness: u32,
}

/// Cambia un LED en el perfil interno activo.
pub async fn set_led(
    vendor: u16,
    product: u16,
    index: u32,
    led: LedSettings,
) -> Result<(), String> {
    let conn = system().await?;
    let device = device_or_err(&conn, vendor, product).await?;
    let (_, profile, _) = active_profile(&conn, &device).await?;
    let p = proxy(&conn, &profile, "org.freedesktop.ratbag1.Profile").await?;
    let paths: Vec<OwnedObjectPath> = get(&p, "Leds").await?;
    let path = paths
        .get(index as usize)
        .ok_or_else(|| t!("ratbag.no_led").into_owned())?;
    let l = proxy(&conn, path.as_str(), "org.freedesktop.ratbag1.Led").await?;
    let set = |name: &'static str, value: Value<'static>| {
        let l = &l;
        async move {
            l.set_property(name, value).await.map_err(|e| {
                t!("ratbag.set_failed", what = name, error = e.to_string()).into_owned()
            })
        }
    };
    let [r, g, b] = led.color.map(|c| c.min(255));
    set("Mode", Value::U32(led.mode)).await?;
    set("Color", Value::from((r, g, b))).await?;
    if led.mode >= 2 {
        set(
            "EffectDuration",
            Value::U32(led.duration.clamp(100, 20_000)),
        )
        .await?;
    }
    set("Brightness", Value::U32(led.brightness.min(255))).await?;
    commit(&conn, &device).await
}

// ---------------------------------------------------------------------------
// Copia de la función original de los botones reprogramados.
//
// Se guarda en `~/.local/state/gnohub/firmware.json` antes de reprogramar un
// botón, para poder devolvérsela después. Por dispositivo → índice de botón →
// función en cada perfil interno.

type Backups = BTreeMap<String, BTreeMap<u32, Vec<(usize, Mapping)>>>;

fn device_key(vendor: u16, product: u16) -> String {
    format!("{vendor:04x}:{product:04x}")
}

fn backup_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "gnohub")
        .and_then(|d| d.state_dir().map(|s| s.join("firmware.json")))
}

fn load_backups() -> Backups {
    backup_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save_backups(backups: &Backups) -> Result<(), String> {
    let path = backup_path().ok_or_else(|| t!("common.no_home").into_owned())?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(backups).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| e.to_string())
}

/// Escribe `mapping` (o, si es `None`, la copia guardada de cada perfil) en el
/// botón `index` de todos los perfiles internos.
async fn write_button(
    conn: &Connection,
    device: &str,
    index: u32,
    mapping: impl Fn(usize) -> Option<Mapping>,
) -> Result<(), String> {
    for (profile_index, profile) in profiles(conn, device).await?.iter().enumerate() {
        let Some(target) = mapping(profile_index) else {
            continue;
        };
        let Some((path, _, _)) = buttons_of(conn, profile.as_str())
            .await?
            .into_iter()
            .find(|(_, i, _)| *i == index)
        else {
            continue;
        };
        let value = target
            .to_dbus()
            .ok_or_else(|| t!("ratbag.unknown_mapping").into_owned())?;
        let b = proxy(conn, &path, "org.freedesktop.ratbag1.Button").await?;
        b.set_property("Mapping", value)
            .await
            .map_err(|e| t!("ratbag.reprogram_failed", error = e.to_string()).into_owned())?;
    }
    Ok(())
}

/// Reprograma el botón `index` para que envíe una tecla libre (F13…F24) en
/// todos los perfiles internos, guardando antes su función original.
/// Devuelve el nombre de la tecla, que es lo que GnoHub asignará.
pub async fn take_over_button(vendor: u16, product: u16, index: u32) -> Result<String, String> {
    let conn = system().await?;
    let device = device_or_err(&conn, vendor, product).await?;
    // Lo que ya envía cualquier otro botón, en cualquier perfil interno: dos
    // botones que envíen lo mismo serían indistinguibles para GnoHub.
    let mut used_buttons: Vec<u32> = Vec::new();
    let mut used_keys: Vec<u32> = Vec::new();
    for profile in profiles(&conn, &device).await? {
        for (_, i, m) in buttons_of(&conn, profile.as_str()).await? {
            if i == index {
                continue;
            }
            match m {
                Mapping::Button { n } => used_buttons.push(n),
                other => used_keys.extend(other.effective_key()),
            }
        }
    }
    let target = free_target(vendor, product, &used_buttons, &used_keys)
        .ok_or_else(|| t!("ratbag.no_free_keys").into_owned())?;

    // Copia de la función original de cada perfil (solo la primera vez: si ya
    // estaba reprogramado, la copia buena es la que ya hay).
    let mut backups = load_backups();
    let already_saved = backups
        .get(&device_key(vendor, product))
        .is_some_and(|b| b.contains_key(&index));
    if !already_saved {
        let mut originals = Vec::new();
        for (i, profile) in profiles(&conn, &device).await?.iter().enumerate() {
            if let Some((_, _, m)) = buttons_of(&conn, profile.as_str())
                .await?
                .into_iter()
                .find(|(_, b, _)| *b == index)
                && !m.is_gnohub_key()
            {
                originals.push((i, m));
            }
        }
        if !originals.is_empty() {
            backups
                .entry(device_key(vendor, product))
                .or_default()
                .insert(index, originals);
            save_backups(&backups)?;
        }
    }

    let name = target.emits().expect("los destinos libres tienen nombre");
    write_button(&conn, &device, index, |_| Some(target.clone())).await?;
    commit(&conn, &device).await?;
    Ok(name)
}

/// Qué programar en un botón que pasa a GnoHub. Primero un botón de ratón
/// extra (6–16) que el dispositivo declare: el ratón informa de cuándo se
/// pulsa y cuándo se suelta, así que funcionan los modos de mantener. Si no
/// queda ninguno, una tecla libre (F13–F24); algunos ratones las envían como
/// pulsar-y-soltar instantáneo, y entonces solo sirven para "Al pulsar".
fn free_target(
    vendor: u16,
    product: u16,
    used_buttons: &[u32],
    used_keys: &[u32],
) -> Option<Mapping> {
    let declared: Vec<u16> = gnohub_core::devices::physical_devices()
        .into_iter()
        .filter(|(_, d)| d.input_id().vendor() == vendor && d.input_id().product() == product)
        .flat_map(|(_, d)| {
            d.supported_keys()
                .map(|k| k.iter().map(|k| k.code()).collect::<Vec<_>>())
                .unwrap_or_default()
        })
        .collect();
    let button = (6..=16u32).find(|n| {
        let code = KeyCode::BTN_LEFT.code() + *n as u16 - 1;
        declared.contains(&code) && !used_buttons.contains(n)
    });
    if let Some(n) = button {
        return Some(Mapping::Button { n });
    }
    FREE_KEYS
        .into_iter()
        .map(|k| u32::from(k.code()))
        .find(|code| !used_keys.contains(code))
        .map(|code| Mapping::Key { code })
}

/// Devuelve al botón su función original (la copia guardada) o, si se indica,
/// la función `mapping`, en todos los perfiles internos.
pub async fn restore_button(
    vendor: u16,
    product: u16,
    index: u32,
    mapping: Option<Mapping>,
) -> Result<(), String> {
    let conn = system().await?;
    let device = device_or_err(&conn, vendor, product).await?;
    let mut backups = load_backups();
    let key = device_key(vendor, product);
    match mapping {
        Some(m) => write_button(&conn, &device, index, |_| Some(m.clone())).await?,
        None => {
            let saved = backups
                .get(&key)
                .and_then(|b| b.get(&index))
                .cloned()
                .ok_or_else(|| t!("ratbag.no_backup").into_owned())?;
            // Un perfil sin copia propia (p. ej. ya estaba reprogramado al
            // guardarla) recibe la función guardada de otro perfil.
            let fallback = saved.first().map(|(_, m)| m.clone());
            write_button(&conn, &device, index, |profile| {
                saved
                    .iter()
                    .find(|(p, _)| *p == profile)
                    .map(|(_, m)| m.clone())
                    .or_else(|| fallback.clone())
            })
            .await?;
        }
    }
    commit(&conn, &device).await?;
    if let Some(device_backups) = backups.get_mut(&key) {
        device_backups.remove(&index);
        save_backups(&backups)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mappings_round_trip_through_dbus_values() {
        for m in [
            Mapping::Button { n: 4 },
            Mapping::Special {
                code: SPECIAL_BASE + 11,
            },
            Mapping::Key { code: 183 },
            Mapping::Macro {
                events: vec![(1, 6), (2, 6)],
            },
            Mapping::None,
        ] {
            let value = m.to_dbus().unwrap();
            let Value::Structure(s) = &value else {
                panic!("{value:?}")
            };
            let [kind, inner] = s.fields() else { panic!() };
            assert_eq!(Mapping::from_dbus(u32_of(kind).unwrap(), inner), m);
        }
    }

    #[test]
    fn describes_what_each_button_emits() {
        assert_eq!(
            Mapping::Button { n: 4 }.emits().as_deref(),
            Some("BTN_SIDE")
        );
        assert_eq!(
            Mapping::Special {
                code: SPECIAL_BASE + 11
            }
            .emits(),
            None
        );
        assert_eq!(
            Mapping::Special {
                code: SPECIAL_BASE + 2
            }
            .emits()
            .as_deref(),
            Some("WHEEL_LEFT")
        );
        assert_eq!(
            Mapping::Key { code: 183 }.emits().as_deref(),
            Some("KEY_F13")
        );
        assert!(Mapping::Key { code: 183 }.is_gnohub_key());
        assert!(!Mapping::Key { code: 30 }.is_gnohub_key());
        // Así devuelve el G502 una tecla programada: macro de pulsar y soltar.
        let f13_macro = Mapping::Macro {
            events: vec![(1, 183), (2, 183)],
        };
        assert_eq!(f13_macro.emits().as_deref(), Some("KEY_F13"));
        assert!(f13_macro.is_gnohub_key());
        assert_eq!(
            Mapping::Macro {
                events: vec![(1, 6), (2, 6)]
            }
            .emits()
            .as_deref(),
            Some("KEY_5")
        );
        // Una macro de verdad (varias teclas) sigue siendo del firmware.
        assert_eq!(
            Mapping::Macro {
                events: vec![(1, 29), (1, 46), (2, 46), (2, 29)]
            }
            .emits(),
            None
        );
    }
}

#[cfg(test)]
mod live {
    /// Reescribe el DPI actual del nivel activo (no cambia nada): comprueba que
    /// ratbagd acepta el formato. `cargo test -p gnohub-gui -- --ignored live_set_same_dpi`.
    #[tokio::test]
    #[ignore]
    async fn live_set_same_dpi() {
        let hw = super::hardware(0x046d, 0xc08b).await.unwrap().unwrap();
        let level = hw.resolutions.iter().find(|r| r.active).unwrap();
        super::set_dpi(0x046d, 0xc08b, level.index, level.dpi).await.unwrap();
        let after = super::hardware(0x046d, 0xc08b).await.unwrap().unwrap();
        assert_eq!(after.resolutions[level.index as usize].dpi, level.dpi);
        println!("DPI {} reescrito sin errores", level.dpi);
    }

    /// Lee el ratón real; se ejecuta a mano: `cargo test -p gnohub-gui -- --ignored --nocapture live`.
    #[tokio::test]
    #[ignore]
    async fn live_read_g502() {
        let hw = super::hardware(0x046d, 0xc08b).await.unwrap();
        println!("{}", serde_json::to_string_pretty(&hw).unwrap());
    }
}
