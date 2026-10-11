//! Formato del archivo de configuración tal como se escribe en TOML.
//!
//! Estas estructuras son las que lee y escribe la interfaz gráfica; no están
//! validadas. `Config::resolve` las convierte en la configuración que usa el
//! daemon, comprobando referencias y retardos mínimos.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use evdev::KeyCode;
use serde::{Deserialize, Serialize};
use toml_edit::{Array, DocumentMut, Item, Table, Value};

use crate::action::Action;
use crate::button::Button;
use crate::config::{ConfigError, DEFAULT_HOLD_TIME_MS, DEFAULT_STEP_DELAY_MS};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigFile {
    #[serde(default)]
    pub macros: BTreeMap<String, MacroDef>,
    #[serde(default)]
    pub devices: Vec<DeviceDef>,
}

fn default_step_delay() -> u64 {
    DEFAULT_STEP_DELAY_MS
}

fn default_hold_time() -> u64 {
    DEFAULT_HOLD_TIME_MS
}

fn one() -> u32 {
    1
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MacroDef {
    /// Milisegundos entre cada paso de la macro (mínimo 1).
    #[serde(default = "default_step_delay")]
    pub step_delay: u64,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub vendor: u16,
    pub product: u16,
    #[serde(default)]
    pub bindings: Vec<BindingDef>,
    /// Perfiles por aplicación; los botones que no cambian heredan `bindings`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub apps: Vec<AppProfileDef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppProfileDef {
    /// Nombre que se muestra en la interfaz.
    pub name: String,
    /// Identificadores de aplicación (`org.mozilla.firefox`) o clases de
    /// ventana (`firefox`) en los que se activa, sin distinguir mayúsculas.
    pub apps: Vec<String>,
    #[serde(default)]
    pub bindings: Vec<BindingDef>,
}

impl Default for MacroDef {
    fn default() -> Self {
        Self {
            step_delay: DEFAULT_STEP_DELAY_MS,
            actions: Vec::new(),
        }
    }
}

/// Una macro con nombre (`"copiar"`) o una macro propia del botón.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MacroRef {
    Named(String),
    Inline(MacroDef),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum BindingDef {
    /// El botón hace lo que hace normalmente. Sirve en un perfil de aplicación
    /// para anular una asignación general.
    Native {
        button: Button,
    },
    /// El botón no hace nada (el sistema no recibe su pulsación).
    Disabled {
        button: Button,
    },
    Remap {
        button: Button,
        to: Vec<KeyCode>,
    },
    Press {
        button: Button,
        r#macro: MacroRef,
        #[serde(default = "one")]
        repeat: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        repeat_delay: Option<u64>,
    },
    WhileHeld {
        button: Button,
        r#macro: MacroRef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        repeat_delay: Option<u64>,
    },
    Toggle {
        button: Button,
        r#macro: MacroRef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        repeat_delay: Option<u64>,
    },
    Sequence {
        button: Button,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on_press: Option<MacroRef>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        while_held: Option<MacroRef>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on_release: Option<MacroRef>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        repeat_delay: Option<u64>,
    },
    TapHold {
        button: Button,
        tap: MacroRef,
        hold: MacroRef,
        #[serde(default = "default_hold_time")]
        hold_time: u64,
        #[serde(default)]
        hold_repeat: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        repeat_delay: Option<u64>,
    },
}

impl BindingDef {
    pub fn button(&self) -> Button {
        match self {
            BindingDef::Native { button }
            | BindingDef::Disabled { button }
            | BindingDef::Remap { button, .. }
            | BindingDef::Press { button, .. }
            | BindingDef::WhileHeld { button, .. }
            | BindingDef::Toggle { button, .. }
            | BindingDef::Sequence { button, .. }
            | BindingDef::TapHold { button, .. } => *button,
        }
    }
}

impl ConfigFile {
    pub fn from_toml(text: &str) -> Result<Self, ConfigError> {
        match migrate(text) {
            Some(migrated) => Ok(toml::from_str(&migrated)?),
            None => Ok(toml::from_str(text)?),
        }
    }

    /// Serializa a TOML legible: acciones en listas compactas (una por línea)
    /// y VID/PID en hexadecimal, como se escribiría a mano.
    pub fn to_toml(&self) -> Result<String, ConfigError> {
        let text = toml::to_string(self)?;
        let mut doc: DocumentMut = text.parse().expect("toml genera TOML válido");
        tidy(&mut doc);
        Ok(doc.to_string())
    }

    /// Lee el archivo; si no existe devuelve una configuración vacía.
    pub fn load_or_default(path: &Path) -> Result<Self, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::from_toml(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(ConfigError::Io {
                path: path.to_owned(),
                source,
            }),
        }
    }

    /// Escribe de forma atómica (archivo temporal + `rename`), para que el
    /// daemon, que recarga al detectar cambios, nunca lea un archivo a medias.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let io_err = |source| ConfigError::Io {
            path: path.to_owned(),
            source,
        };
        let text = self.to_toml()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(io_err)?;
        }
        let tmp = path.with_extension("toml.tmp");
        let mut file = std::fs::File::create(&tmp).map_err(io_err)?;
        file.write_all(text.as_bytes()).map_err(io_err)?;
        file.sync_all().map_err(io_err)?;
        std::fs::rename(&tmp, path).map_err(io_err)
    }
}

/// Adapta archivos del formato anterior, que tenía un `[settings] step_delay`
/// global y macros de botón escritas como lista de acciones sin retardo.
/// Devuelve `None` si no hay nada que cambiar (o si el TOML no se puede leer:
/// entonces el error lo da el parser normal).
fn migrate(text: &str) -> Option<String> {
    let mut doc: DocumentMut = text.parse().ok()?;
    let mut changed = false;

    let legacy_step = doc
        .remove("settings")
        .and_then(|settings| {
            changed = true;
            settings.get("step_delay").and_then(Item::as_integer)
        })
        .unwrap_or(DEFAULT_STEP_DELAY_MS as i64);

    if let Some(macros) = doc.get_mut("macros").and_then(Item::as_table_like_mut) {
        for (_, m) in macros.iter_mut() {
            if let Some(m) = m.as_table_like_mut()
                && !m.contains_key("step_delay")
            {
                m.insert("step_delay", toml_edit::value(legacy_step));
                changed = true;
            }
        }
    }

    if let Some(devices) = doc
        .get_mut("devices")
        .and_then(Item::as_array_of_tables_mut)
    {
        for device in devices.iter_mut() {
            let Some(bindings) = device.get_mut("bindings") else {
                continue;
            };
            let mut fix = |binding: &mut dyn toml_edit::TableLike| {
                for key in &ACTION_KEYS[1..] {
                    let Some(Item::Value(Value::Array(actions))) = binding.get(key) else {
                        continue;
                    };
                    let mut own = toml_edit::InlineTable::new();
                    own.insert("step_delay", legacy_step.into());
                    own.insert("actions", Value::Array(actions.clone()));
                    binding.insert(key, Item::Value(Value::InlineTable(own)));
                    changed = true;
                }
            };
            if let Some(list) = bindings.as_array_of_tables_mut() {
                list.iter_mut().for_each(|b| fix(b));
            } else if let Some(list) = bindings.as_array_mut() {
                for b in list.iter_mut() {
                    if let Value::InlineTable(t) = b {
                        fix(t);
                    }
                }
            }
        }
    }

    changed.then(|| doc.to_string())
}

/// Claves cuyo valor puede ser una lista de acciones.
const ACTION_KEYS: &[&str] = &[
    "actions",
    "macro",
    "tap",
    "hold",
    "on_press",
    "while_held",
    "on_release",
];

fn tidy(doc: &mut DocumentMut) {
    if let Some(macros) = doc.get_mut("macros").and_then(Item::as_table_mut) {
        for (_, m) in macros.iter_mut() {
            if let Some(table) = m.as_table_mut() {
                table.set_implicit(false);
                inline_actions(table);
            }
        }
    }
    if let Some(devices) = doc
        .get_mut("devices")
        .and_then(Item::as_array_of_tables_mut)
    {
        for device in devices.iter_mut() {
            for key in ["vendor", "product"] {
                if let Some(n) = device.get(key).and_then(Item::as_integer) {
                    let mut hex: Value = format!("0x{n:04x}")
                        .parse()
                        .expect("entero hexadecimal válido");
                    hex.decor_mut().set_prefix(" ");
                    device.insert(key, Item::Value(hex));
                }
            }
            if let Some(bindings) = device
                .get_mut("bindings")
                .and_then(Item::as_array_of_tables_mut)
            {
                bindings.iter_mut().for_each(inline_actions);
            }
            if let Some(profiles) = device
                .get_mut("apps")
                .and_then(Item::as_array_of_tables_mut)
            {
                for profile in profiles.iter_mut() {
                    if let Some(bindings) = profile
                        .get_mut("bindings")
                        .and_then(Item::as_array_of_tables_mut)
                    {
                        bindings.iter_mut().for_each(inline_actions);
                    }
                }
            }
        }
    }
}

/// Compacta las acciones: `[[x.actions]]` pasa a `actions = [ … ]` con una
/// acción por línea, y las macros propias de un botón (`[x.macro]`) pasan a
/// una tabla en línea `{ step_delay = …, actions = [ … ] }`.
fn inline_actions(table: &mut Table) {
    for key in ACTION_KEYS {
        let Some(item) = table.get_mut(key) else {
            continue;
        };
        let mut value = if let Some(tables) = item.as_array_of_tables() {
            let mut array: Array = tables.clone().into_array();
            for value in array.iter_mut() {
                format_value(value);
                value.decor_mut().set_prefix("\n    ");
            }
            array.set_trailing_comma(true);
            array.set_trailing("\n");
            Value::Array(array)
        } else if let Some(t) = item.as_table() {
            let mut value = Value::InlineTable(t.clone().into_inline_table());
            format_value(&mut value);
            value
        } else {
            continue;
        };
        value.decor_mut().set_prefix(" ");
        *item = Item::Value(value);
        if let Some(mut k) = table.key_mut(key) {
            k.leaf_decor_mut().set_suffix(" ");
        }
    }
}

/// Formato compacto de una línea para tablas y listas anidadas.
fn format_value(value: &mut Value) {
    match value {
        Value::InlineTable(t) => {
            t.fmt();
            for (_, v) in t.iter_mut() {
                format_value(v);
            }
        }
        Value::Array(a) => {
            a.fmt();
            for v in a.iter_mut() {
                format_value(v);
            }
        }
        _ => {}
    }
}

/// `KEY_CNT` de `linux/input-event-codes.h` (evdev no lo expone).
const KEY_CNT: u16 = 0x300;

/// Nombre de una tecla (`KEY_A`, `BTN_SIDE`…), o `None` si evdev no le da nombre.
///
/// Hay que usarla en vez de serializar `KeyCode` directamente: su serializador
/// hace `panic` con los códigos sin nombre, y algunos dispositivos los declaran.
pub fn key_name(key: KeyCode) -> Option<String> {
    if let Some(n) = extra_mouse_button(key) {
        return Some(format!("BTN_MOUSE_{n}"));
    }
    let name = format!("{key:?}");
    (!name.starts_with("unknown")).then_some(name)
}

/// Los botones de ratón 9 a 16 (códigos 0x118–0x11f) no tienen nombre en
/// Linux; GnoHub los llama `BTN_MOUSE_9` … `BTN_MOUSE_16`. Los ratones con
/// muchos botones los usan, y GnoHub los asigna al reprogramar botones.
fn extra_mouse_button(key: KeyCode) -> Option<u16> {
    let first = KeyCode::BTN_LEFT.code();
    (first + 8..=first + 15)
        .contains(&key.code())
        .then(|| key.code() - first + 1)
}

/// Inversa de `key_name`: admite los nombres de evdev y `BTN_MOUSE_9`…`BTN_MOUSE_16`.
pub fn parse_key(name: &str) -> Option<KeyCode> {
    let upper = name.trim().to_ascii_uppercase();
    if let Some(n) = upper
        .strip_prefix("BTN_MOUSE_")
        .and_then(|n| n.parse::<u16>().ok())
        && (9..=16).contains(&n)
    {
        return Some(KeyCode::new(KeyCode::BTN_LEFT.code() + n - 1));
    }
    upper.parse().ok()
}

/// Nombres de todas las teclas y botones que entiende la configuración.
pub fn known_keys() -> Vec<String> {
    (0..KEY_CNT)
        .filter_map(|code| key_name(KeyCode::new(code)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;

    const SAMPLE: &str = include_str!("../../../examples/g502.toml");

    #[test]
    fn toml_round_trip_keeps_meaning() {
        let file = ConfigFile::from_toml(SAMPLE).unwrap();
        let written = file.to_toml().unwrap();
        assert_eq!(ConfigFile::from_toml(&written).unwrap(), file, "{written}");
        assert!(Config::resolve(file).is_ok());
    }

    #[test]
    fn json_round_trip_for_the_gui() {
        let file = ConfigFile::from_toml(SAMPLE).unwrap();
        let json = serde_json::to_string(&file).unwrap();
        assert!(json.contains(r#""mode":"tap_hold""#), "{json}");
        assert!(json.contains(r#""tap":"copiar""#), "{json}");
        assert_eq!(serde_json::from_str::<ConfigFile>(&json).unwrap(), file);
    }

    #[test]
    fn button_macros_round_trip_as_inline_tables() {
        let text = r#"
            [[devices]]
            vendor = 1
            product = 2

            [[devices.bindings]]
            button = "BTN_SIDE"
            mode = "sequence"
            on_press = { step_delay = 5, actions = [{ chord = ["KEY_LEFTCTRL", "KEY_C"] }, { delay = 20 }] }
            on_release = "otra"
        "#;
        let file = ConfigFile::from_toml(text).unwrap();
        let written = file.to_toml().unwrap();
        assert!(
            written.contains(r#"on_press = { step_delay = 5, actions = [{ chord = ["KEY_LEFTCTRL", "KEY_C"] }, { delay = 20 }] }"#),
            "{written}"
        );
        assert_eq!(ConfigFile::from_toml(&written).unwrap(), file);
    }

    #[test]
    fn migrates_global_settings_and_action_lists() {
        let old = r#"
            [settings]
            step_delay = 4

            [macros.a]
            actions = [{ tap = "KEY_A" }]

            [macros.b]
            step_delay = 7
            actions = [{ tap = "KEY_B" }]

            [[devices]]
            vendor = 1
            product = 2

            [[devices.bindings]]
            button = "BTN_SIDE"
            mode = "press"
            macro = [{ tap = "KEY_C" }]

            [[devices.bindings]]
            button = "BTN_EXTRA"
            mode = "tap_hold"
            tap = "a"
            hold = [{ tap = "KEY_D" }]
        "#;
        let file = ConfigFile::from_toml(old).unwrap();
        assert_eq!(file.macros["a"].step_delay, 4);
        assert_eq!(file.macros["b"].step_delay, 7);
        let bindings = &file.devices[0].bindings;
        assert!(
            matches!(&bindings[0], BindingDef::Press { r#macro: MacroRef::Inline(m), .. } if m.step_delay == 4)
        );
        assert!(
            matches!(&bindings[1], BindingDef::TapHold { hold: MacroRef::Inline(m), tap: MacroRef::Named(_), .. } if m.step_delay == 4)
        );
        assert!(Config::resolve(file).is_ok());
    }

    #[test]
    fn app_profiles_round_trip() {
        let text = r#"
            [[devices]]
            vendor = 1
            product = 2

            [[devices.apps]]
            name = "Juego"
            apps = ["steam_app_730"]

            [[devices.apps.bindings]]
            button = "BTN_SIDE"
            mode = "while_held"
            macro = { step_delay = 1, actions = [{ tap = "BTN_LEFT" }] }
        "#;
        let file = ConfigFile::from_toml(text).unwrap();
        let written = file.to_toml().unwrap();
        assert!(
            written.contains("macro = { step_delay = 1, actions = [{ tap = \"BTN_LEFT\" }] }"),
            "{written}"
        );
        assert_eq!(ConfigFile::from_toml(&written).unwrap(), file);
    }

    #[test]
    fn key_name_never_panics_on_unnamed_codes() {
        assert_eq!(key_name(KeyCode::BTN_SIDE).as_deref(), Some("BTN_SIDE"));
        assert_eq!(key_name(KeyCode::new(0x2ff)), None);
    }

    #[test]
    fn extra_mouse_buttons_have_names() {
        assert_eq!(
            key_name(KeyCode::new(0x118)).as_deref(),
            Some("BTN_MOUSE_9")
        );
        assert_eq!(
            key_name(KeyCode::new(0x11f)).as_deref(),
            Some("BTN_MOUSE_16")
        );
        assert_eq!(parse_key("btn_mouse_9"), Some(KeyCode::new(0x118)));
        assert_eq!(parse_key("BTN_MOUSE_17"), None);
        assert_eq!(parse_key("BTN_SIDE"), Some(KeyCode::BTN_SIDE));
        assert!("BTN_MOUSE_12".parse::<crate::Button>().is_ok());
    }

    #[test]
    fn known_keys_has_mouse_and_keyboard() {
        let keys = known_keys();
        for k in ["KEY_A", "KEY_LEFTCTRL", "BTN_LEFT", "BTN_SIDE"] {
            assert!(keys.iter().any(|n| n == k), "{k}");
        }
    }
}
