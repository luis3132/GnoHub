//! Configuración en TOML: macros reutilizables y asignaciones por dispositivo.
//!
//! El TOML se lee en estructuras "crudas" y luego se valida y resuelve, de modo
//! que el resto del programa nunca ve referencias a macros inexistentes ni
//! retardos por debajo del mínimo.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use evdev::KeyCode;

use rust_i18n::t;

use crate::action::{Action, Step, expand};
use crate::button::Button;
use crate::file::{BindingDef, ConfigFile, DeviceDef, MacroRef};

/// Retardo mínimo entre pasos de una macro, en milisegundos.
pub const MIN_DELAY_MS: u64 = 1;
/// Retardo entre pasos cuando la macro no lo indica.
pub const DEFAULT_STEP_DELAY_MS: u64 = 10;
/// Tiempo que hay que mantener un botón `tap_hold` para que cuente como "mantener".
pub const DEFAULT_HOLD_TIME_MS: u64 = 250;

/// Error de configuración. Su texto sale en el idioma activo (`set_locale`).
#[derive(Debug)]
pub enum ConfigError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse(toml::de::Error),
    Serialize(toml::ser::Error),
    EmptyMacro(String),
    DelayTooShort {
        context: String,
        field: &'static str,
    },
    ZeroRepeat {
        context: String,
    },
    EmptySequence {
        context: String,
    },
    EmptyRemap {
        context: String,
    },
    DuplicateButton {
        device: String,
        button: Button,
    },
    EmptyAppList {
        context: String,
    },
    UnknownMacro {
        context: String,
        name: String,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            ConfigError::Io { path, source } => {
                t!("config.read_failed", path = path.display(), error = source)
            }
            ConfigError::Parse(e) => t!("config.invalid_toml", error = e),
            ConfigError::Serialize(e) => t!("config.serialize_failed", error = e),
            ConfigError::EmptyMacro(name) => t!("config.empty_macro", name = name),
            ConfigError::DelayTooShort { context, field } => {
                t!(
                    "config.delay_too_short",
                    context = context,
                    field = field,
                    min = MIN_DELAY_MS
                )
            }
            ConfigError::ZeroRepeat { context } => t!("config.zero_repeat", context = context),
            ConfigError::EmptySequence { context } => {
                t!("config.empty_sequence", context = context)
            }
            ConfigError::EmptyRemap { context } => t!("config.empty_remap", context = context),
            ConfigError::DuplicateButton { device, button } => {
                t!("config.duplicate_button", device = device, button = button)
            }
            ConfigError::EmptyAppList { context } => t!("config.empty_app_list", context = context),
            ConfigError::UnknownMacro { context, name } => {
                t!("config.unknown_macro", context = context, name = name)
            }
        };
        f.write_str(&text)
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Io { source, .. } => Some(source),
            ConfigError::Parse(e) => Some(e),
            ConfigError::Serialize(e) => Some(e),
            _ => None,
        }
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(e: toml::de::Error) -> Self {
        ConfigError::Parse(e)
    }
}

impl From<toml::ser::Error> for ConfigError {
    fn from(e: toml::ser::Error) -> Self {
        ConfigError::Serialize(e)
    }
}

#[derive(Debug)]
pub struct Macro {
    pub name: String,
    pub step_delay: Duration,
    pub actions: Vec<Action>,
    pub steps: Vec<Step>,
}

impl Macro {
    fn new(name: String, actions: Vec<Action>, step_delay_ms: u64) -> Result<Self, ConfigError> {
        if actions.is_empty() {
            return Err(ConfigError::EmptyMacro(name));
        }
        check_delay(&name, "step_delay", step_delay_ms)?;
        if actions
            .iter()
            .any(|a| matches!(a, Action::Delay(ms) if *ms < MIN_DELAY_MS))
        {
            return Err(ConfigError::DelayTooShort {
                context: name,
                field: "delay",
            });
        }
        let step_delay = Duration::from_millis(step_delay_ms);
        let steps = expand(&actions, step_delay);
        Ok(Self {
            name,
            step_delay,
            actions,
            steps,
        })
    }
}

/// Qué hace un botón. Los modos imitan los de G Hub.
#[derive(Debug, Clone)]
pub enum Binding {
    /// El botón hace lo que hace normalmente (anula una asignación heredada).
    Native,
    /// El botón no hace nada: su pulsación se descarta.
    Disabled,
    /// El botón se comporta como otra tecla o combinación mientras está pulsado.
    Remap(Vec<KeyCode>),
    /// Al pulsar, ejecuta la macro `times` veces.
    Press {
        r#macro: Arc<Macro>,
        times: u32,
        gap: Duration,
    },
    /// Repite la macro mientras el botón siga pulsado.
    WhileHeld { r#macro: Arc<Macro>, gap: Duration },
    /// Una pulsación arranca la repetición y la siguiente la detiene.
    Toggle { r#macro: Arc<Macro>, gap: Duration },
    /// Secuencia de G Hub: una macro al pulsar, otra en bucle mientras se
    /// mantiene y otra al soltar.
    Sequence {
        on_press: Option<Arc<Macro>>,
        while_held: Option<Arc<Macro>>,
        on_release: Option<Arc<Macro>>,
        gap: Duration,
    },
    /// Pulsación corta ejecuta `tap`; mantener más de `hold_time` ejecuta `hold`
    /// (en bucle si `hold_repeat`).
    TapHold {
        tap: Arc<Macro>,
        hold: Arc<Macro>,
        hold_time: Duration,
        hold_repeat: bool,
        gap: Duration,
    },
}

impl Binding {
    /// Macros que puede ejecutar esta asignación.
    pub fn macros(&self) -> Vec<&Arc<Macro>> {
        match self {
            Binding::Native | Binding::Disabled | Binding::Remap(_) => Vec::new(),
            Binding::Press { r#macro, .. }
            | Binding::WhileHeld { r#macro, .. }
            | Binding::Toggle { r#macro, .. } => {
                vec![r#macro]
            }
            Binding::Sequence {
                on_press,
                while_held,
                on_release,
                ..
            } => [on_press, while_held, on_release]
                .into_iter()
                .flatten()
                .collect(),
            Binding::TapHold { tap, hold, .. } => vec![tap, hold],
        }
    }
}

/// La aplicación con el foco, tal como la informa el entorno de escritorio.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActiveApp {
    /// Identificador de aplicación (`org.mozilla.firefox`), si se conoce.
    pub app_id: String,
    /// Clase de ventana X11 / `app_id` de Wayland (`firefox`).
    pub wm_class: String,
}

#[derive(Debug, Clone)]
pub struct AppProfile {
    pub name: String,
    pub apps: Vec<String>,
    pub bindings: HashMap<Button, Binding>,
}

impl AppProfile {
    pub fn matches(&self, app: &ActiveApp) -> bool {
        self.apps.iter().any(|a| {
            !a.is_empty()
                && (a.eq_ignore_ascii_case(&app.app_id) || a.eq_ignore_ascii_case(&app.wm_class))
        })
    }
}

#[derive(Debug, Clone)]
pub struct DeviceConfig {
    pub name: Option<String>,
    pub vendor: u16,
    pub product: u16,
    pub bindings: HashMap<Button, Binding>,
    pub apps: Vec<AppProfile>,
}

impl DeviceConfig {
    pub fn label(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| format!("{:04x}:{:04x}", self.vendor, self.product))
    }

    pub fn matches(&self, vendor: u16, product: u16) -> bool {
        self.vendor == vendor && self.product == product
    }

    /// Asignación de `button` con `app` enfocada: la del primer perfil que
    /// coincida y lo asigne, si no la general. `None` si el botón debe
    /// funcionar como siempre.
    pub fn lookup(&self, button: Button, app: Option<&ActiveApp>) -> Option<&Binding> {
        let from_profile = app.and_then(|app| {
            self.apps
                .iter()
                .filter(|p| p.matches(app))
                .find_map(|p| p.bindings.get(&button))
        });
        from_profile
            .or_else(|| self.bindings.get(&button))
            .filter(|b| !matches!(b, Binding::Native))
    }

    /// Todos los botones con alguna asignación, en cualquier perfil.
    pub fn buttons(&self) -> impl Iterator<Item = Button> + '_ {
        self.bindings
            .keys()
            .chain(self.apps.iter().flat_map(|p| p.bindings.keys()))
            .copied()
    }

    fn all_bindings(&self) -> impl Iterator<Item = &Binding> {
        self.bindings
            .values()
            .chain(self.apps.iter().flat_map(|p| p.bindings.values()))
    }
}

#[derive(Debug, Default)]
pub struct Config {
    pub macros: BTreeMap<String, Arc<Macro>>,
    pub devices: Vec<DeviceConfig>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Io {
            path: path.to_owned(),
            source,
        })?;
        text.parse()
    }

    /// Todas las teclas que alguna asignación puede emitir.
    pub fn output_keys(&self) -> Vec<KeyCode> {
        let mut keys = Vec::new();
        for m in self.macros.values() {
            keys.extend(m.actions.iter().flat_map(Action::keys));
        }
        for binding in self.devices.iter().flat_map(|d| d.all_bindings()) {
            if let Binding::Remap(k) = binding {
                keys.extend(k);
            }
            for m in binding.macros() {
                keys.extend(m.actions.iter().flat_map(Action::keys));
            }
        }
        keys
    }
}

impl std::str::FromStr for Config {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::resolve(ConfigFile::from_toml(s)?)
    }
}

impl Config {
    /// Valida un `ConfigFile` y lo convierte en la configuración ejecutable.
    pub fn resolve(file: ConfigFile) -> Result<Self, ConfigError> {
        let macros = file
            .macros
            .into_iter()
            .map(|(name, m)| {
                Ok((
                    name.clone(),
                    Arc::new(Macro::new(name, m.actions, m.step_delay)?),
                ))
            })
            .collect::<Result<BTreeMap<_, _>, ConfigError>>()?;

        let resolver = Resolver { macros: &macros };
        let devices = file
            .devices
            .into_iter()
            .map(|d| resolver.device(d))
            .collect::<Result<_, _>>()?;

        Ok(Self { macros, devices })
    }
}

fn check_delay(context: &str, field: &'static str, ms: u64) -> Result<(), ConfigError> {
    if ms < MIN_DELAY_MS {
        return Err(ConfigError::DelayTooShort {
            context: context.to_owned(),
            field,
        });
    }
    Ok(())
}

struct Resolver<'a> {
    macros: &'a BTreeMap<String, Arc<Macro>>,
}

impl Resolver<'_> {
    fn device(&self, raw: DeviceDef) -> Result<DeviceConfig, ConfigError> {
        let mut device = DeviceConfig {
            name: raw.name,
            vendor: raw.vendor,
            product: raw.product,
            bindings: HashMap::new(),
            apps: Vec::new(),
        };
        let label = device.label();
        device.bindings = self.bindings(raw.bindings, &label)?;
        for profile in raw.apps {
            let label = t!(
                "config.context.profile",
                device = label,
                name = profile.name
            )
            .into_owned();
            let apps: Vec<String> = profile
                .apps
                .into_iter()
                .map(|a| a.trim().to_owned())
                .filter(|a| !a.is_empty())
                .collect();
            if apps.is_empty() {
                return Err(ConfigError::EmptyAppList { context: label });
            }
            device.apps.push(AppProfile {
                bindings: self.bindings(profile.bindings, &label)?,
                name: profile.name,
                apps,
            });
        }
        Ok(device)
    }

    fn bindings(
        &self,
        raw: Vec<BindingDef>,
        label: &str,
    ) -> Result<HashMap<Button, Binding>, ConfigError> {
        let mut bindings = HashMap::new();
        for b in raw {
            let button = b.button();
            let context = t!("config.context.button", device = label, button = button).into_owned();
            let binding = self.binding(b, &context)?;
            if bindings.insert(button, binding).is_some() {
                return Err(ConfigError::DuplicateButton {
                    device: label.to_owned(),
                    button,
                });
            }
        }
        Ok(bindings)
    }

    fn macro_ref(&self, r: MacroRef, context: &str, slot: &str) -> Result<Arc<Macro>, ConfigError> {
        match r {
            MacroRef::Named(name) => {
                self.macros
                    .get(&name)
                    .cloned()
                    .ok_or_else(|| ConfigError::UnknownMacro {
                        context: context.to_owned(),
                        name,
                    })
            }
            MacroRef::Inline(m) => {
                Macro::new(format!("{context} ({slot})"), m.actions, m.step_delay).map(Arc::new)
            }
        }
    }

    /// Pausa entre repeticiones; por defecto, el mismo retardo entre pasos de la macro.
    fn gap(
        &self,
        repeat_delay: Option<u64>,
        r#macro: Option<&Macro>,
        context: &str,
    ) -> Result<Duration, ConfigError> {
        match repeat_delay {
            Some(ms) => {
                check_delay(context, "repeat_delay", ms)?;
                Ok(Duration::from_millis(ms))
            }
            None => Ok(
                r#macro.map_or(Duration::from_millis(DEFAULT_STEP_DELAY_MS), |m| {
                    m.step_delay
                }),
            ),
        }
    }

    fn binding(&self, raw: BindingDef, ctx: &str) -> Result<Binding, ConfigError> {
        Ok(match raw {
            BindingDef::Native { .. } => Binding::Native,
            BindingDef::Disabled { .. } => Binding::Disabled,
            BindingDef::Remap { to, .. } => {
                if to.is_empty() {
                    return Err(ConfigError::EmptyRemap {
                        context: ctx.to_owned(),
                    });
                }
                Binding::Remap(to)
            }
            BindingDef::Press {
                r#macro,
                repeat,
                repeat_delay,
                ..
            } => {
                if repeat == 0 {
                    return Err(ConfigError::ZeroRepeat {
                        context: ctx.to_owned(),
                    });
                }
                let r#macro = self.macro_ref(r#macro, ctx, "macro")?;
                Binding::Press {
                    gap: self.gap(repeat_delay, Some(&r#macro), ctx)?,
                    r#macro,
                    times: repeat,
                }
            }
            BindingDef::WhileHeld {
                r#macro,
                repeat_delay,
                ..
            } => {
                let r#macro = self.macro_ref(r#macro, ctx, "macro")?;
                Binding::WhileHeld {
                    gap: self.gap(repeat_delay, Some(&r#macro), ctx)?,
                    r#macro,
                }
            }
            BindingDef::Toggle {
                r#macro,
                repeat_delay,
                ..
            } => {
                let r#macro = self.macro_ref(r#macro, ctx, "macro")?;
                Binding::Toggle {
                    gap: self.gap(repeat_delay, Some(&r#macro), ctx)?,
                    r#macro,
                }
            }
            BindingDef::Sequence {
                on_press,
                while_held,
                on_release,
                repeat_delay,
                ..
            } => {
                if on_press.is_none() && while_held.is_none() && on_release.is_none() {
                    return Err(ConfigError::EmptySequence {
                        context: ctx.to_owned(),
                    });
                }
                let while_held = while_held
                    .map(|r| self.macro_ref(r, ctx, "while_held"))
                    .transpose()?;
                Binding::Sequence {
                    on_press: on_press
                        .map(|r| self.macro_ref(r, ctx, "on_press"))
                        .transpose()?,
                    gap: self.gap(repeat_delay, while_held.as_deref(), ctx)?,
                    while_held,
                    on_release: on_release
                        .map(|r| self.macro_ref(r, ctx, "on_release"))
                        .transpose()?,
                }
            }
            BindingDef::TapHold {
                tap,
                hold,
                hold_time,
                hold_repeat,
                repeat_delay,
                ..
            } => {
                check_delay(ctx, "hold_time", hold_time)?;
                let hold = self.macro_ref(hold, ctx, "hold")?;
                Binding::TapHold {
                    tap: self.macro_ref(tap, ctx, "tap")?,
                    gap: self.gap(repeat_delay, Some(&hold), ctx)?,
                    hold,
                    hold_time: Duration::from_millis(hold_time),
                    hold_repeat,
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
        [macros.copiar]
        step_delay = 3
        actions = [{ chord = ["KEY_LEFTCTRL", "KEY_C"] }]

        [[devices]]
        name = "G502"
        vendor = 0x046d
        product = 0xc08b

        [[devices.bindings]]
        button = "BTN_SIDE"
        mode = "press"
        macro = "copiar"
        repeat = 3
        repeat_delay = 100

        [[devices.bindings]]
        button = "btn_extra"
        mode = "remap"
        to = ["KEY_LEFTMETA"]

        [[devices.bindings]]
        button = "BTN_FORWARD"
        mode = "while_held"
        macro = { step_delay = 20, actions = [{ tap = "BTN_LEFT" }, { delay = 50 }] }

        [[devices.bindings]]
        button = "BTN_BACK"
        mode = "sequence"
        on_press = { actions = [{ down = "KEY_LEFTSHIFT" }] }
        while_held = { actions = [{ tap = "KEY_W" }] }
        on_release = { actions = [{ up = "KEY_LEFTSHIFT" }] }

        [[devices.bindings]]
        button = "wheel_left"
        mode = "press"
        macro = "copiar"

        [[devices.bindings]]
        button = "BTN_TASK"
        mode = "tap_hold"
        tap = "copiar"
        hold = { actions = [{ tap = "KEY_V" }] }
        hold_time = 400
        hold_repeat = true
    "#;

    fn sleeps(m: &Macro) -> Vec<u64> {
        m.steps
            .iter()
            .filter_map(|s| match s {
                Step::Sleep(d) => Some(d.as_millis() as u64),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn parses_all_modes() {
        let cfg: Config = SAMPLE.parse().unwrap();
        let dev = &cfg.devices[0];
        assert!(dev.matches(0x046d, 0xc08b));

        match &dev.bindings[&Button::Key(KeyCode::BTN_SIDE)] {
            Binding::Press {
                r#macro,
                times,
                gap,
            } => {
                assert_eq!(r#macro.name, "copiar");
                assert_eq!(*times, 3);
                assert_eq!(*gap, Duration::from_millis(100));
                // step_delay propio de la macro: 4 pasos → 3 huecos de 3 ms
                assert_eq!(sleeps(r#macro), vec![3, 3, 3]);
            }
            other => panic!("{other:?}"),
        }
        assert!(
            matches!(&dev.bindings[&Button::Key(KeyCode::BTN_EXTRA)], Binding::Remap(k) if k == &[KeyCode::KEY_LEFTMETA])
        );
        match &dev.bindings[&Button::Key(KeyCode::BTN_FORWARD)] {
            // macro propia del botón con step_delay = 20; el delay explícito lo sustituye
            // y la pausa entre repeticiones usa el step_delay de la macro
            Binding::WhileHeld { r#macro, gap } => {
                assert_eq!(sleeps(r#macro), vec![20, 50]);
                assert_eq!(*gap, Duration::from_millis(20));
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            &dev.bindings[&Button::Key(KeyCode::BTN_BACK)],
            Binding::Sequence {
                on_press: Some(_),
                while_held: Some(_),
                on_release: Some(_),
                ..
            }
        ));
        assert!(matches!(
            &dev.bindings[&Button::Wheel(crate::Wheel::Left)],
            Binding::Press { .. }
        ));
        assert!(matches!(
            &dev.bindings[&Button::Key(KeyCode::BTN_TASK)],
            Binding::TapHold { hold_time, hold_repeat: true, .. } if *hold_time == Duration::from_millis(400)
        ));
    }

    #[test]
    fn app_profiles_override_and_inherit() {
        let cfg: Config = r#"
            [[devices]]
            vendor = 1
            product = 2
            bindings = [
                { button = "BTN_SIDE", mode = "remap", to = ["KEY_A"] },
                { button = "BTN_EXTRA", mode = "remap", to = ["KEY_B"] },
            ]

            [[devices.apps]]
            name = "Navegador"
            apps = ["org.mozilla.firefox", "firefox"]
            bindings = [
                { button = "BTN_SIDE", mode = "remap", to = ["KEY_C"] },
                { button = "BTN_EXTRA", mode = "native" },
            ]
        "#
        .parse()
        .unwrap();
        let dev = &cfg.devices[0];
        let side = Button::Key(KeyCode::BTN_SIDE);
        let extra = Button::Key(KeyCode::BTN_EXTRA);
        let firefox = ActiveApp {
            app_id: String::new(),
            wm_class: "Firefox".into(),
        };
        let other = ActiveApp {
            app_id: "org.gnome.Nautilus".into(),
            wm_class: "nautilus".into(),
        };
        let remap_to = |b: Option<&Binding>| match b {
            Some(Binding::Remap(k)) => Some(k[0]),
            _ => None,
        };

        assert_eq!(
            remap_to(dev.lookup(side, Some(&firefox))),
            Some(KeyCode::KEY_C)
        );
        assert!(
            dev.lookup(extra, Some(&firefox)).is_none(),
            "native anula la general"
        );
        assert_eq!(
            remap_to(dev.lookup(side, Some(&other))),
            Some(KeyCode::KEY_A)
        );
        assert_eq!(remap_to(dev.lookup(extra, None)), Some(KeyCode::KEY_B));
        assert_eq!(dev.buttons().count(), 4);
    }

    #[test]
    fn disabled_swallows_but_native_passes_through() {
        let cfg: Config = r#"
            [[devices]]
            vendor = 1
            product = 2
            bindings = [
                { button = "KEY_F13", mode = "disabled" },
                { button = "BTN_SIDE", mode = "native" },
            ]
        "#
        .parse()
        .unwrap();
        let dev = &cfg.devices[0];
        // `Disabled` llega al ejecutor (que no hace nada) y la tecla no pasa al sistema.
        assert!(matches!(
            dev.lookup(Button::Key(KeyCode::KEY_F13), None),
            Some(Binding::Disabled)
        ));
        // `Native` devuelve None: el evento se reenvía tal cual.
        assert!(dev.lookup(Button::Key(KeyCode::BTN_SIDE), None).is_none());
    }

    #[test]
    fn errors_follow_the_active_locale() {
        let err = ConfigError::EmptyMacro("x".into());
        crate::set_locale("es");
        let es = err.to_string();
        crate::set_locale("en");
        let en = err.to_string();
        assert_eq!(es, "la macro «x» no tiene pasos");
        assert_eq!(en, "the macro “x” has no steps");
    }

    #[test]
    fn rejects_profile_without_apps() {
        let err = "[[devices]]\nvendor = 1\nproduct = 2\n[[devices.apps]]\nname = \"x\"\napps = [\" \"]\n"
            .parse::<Config>()
            .unwrap_err();
        assert!(matches!(err, ConfigError::EmptyAppList { .. }), "{err}");
    }

    #[test]
    fn output_keys_include_everything() {
        let cfg: Config = SAMPLE.parse().unwrap();
        let keys = cfg.output_keys();
        for k in [
            KeyCode::KEY_LEFTCTRL,
            KeyCode::KEY_C,
            KeyCode::KEY_LEFTMETA,
            KeyCode::BTN_LEFT,
            KeyCode::KEY_W,
            KeyCode::KEY_V,
        ] {
            assert!(keys.contains(&k), "{k:?}");
        }
    }

    fn device_with(binding: &str) -> String {
        format!("[[devices]]\nvendor = 1\nproduct = 2\nbindings = [{binding}]\n")
    }

    #[test]
    fn rejects_unknown_macro() {
        let err = device_with(r#"{ button = "BTN_SIDE", mode = "press", macro = "nope" }"#)
            .parse::<Config>()
            .unwrap_err();
        assert!(matches!(err, ConfigError::UnknownMacro { .. }));
    }

    #[test]
    fn rejects_delays_below_one_ms() {
        for bad in [
            r#"{ button = "BTN_SIDE", mode = "press", macro = { actions = [{ delay = 0 }] } }"#,
            r#"{ button = "BTN_SIDE", mode = "toggle", macro = { actions = [{ tap = "KEY_A" }] }, repeat_delay = 0 }"#,
        ] {
            let err = device_with(bad).parse::<Config>().unwrap_err();
            assert!(
                matches!(err, ConfigError::DelayTooShort { .. }),
                "{bad}: {err}"
            );
        }
        let err = "[macros.x]\nstep_delay = 0\nactions = [{ tap = \"KEY_A\" }]\n"
            .parse::<Config>()
            .unwrap_err();
        assert!(matches!(err, ConfigError::DelayTooShort { .. }));
    }

    #[test]
    fn rejects_duplicate_button() {
        let err = device_with(
            r#"{ button = "BTN_SIDE", mode = "remap", to = ["KEY_A"] },
               { button = "BTN_SIDE", mode = "remap", to = ["KEY_B"] }"#,
        )
        .parse::<Config>()
        .unwrap_err();
        assert!(matches!(err, ConfigError::DuplicateButton { .. }));
    }

    #[test]
    fn rejects_empty_sequence_and_zero_repeat() {
        let err = device_with(r#"{ button = "BTN_SIDE", mode = "sequence" }"#)
            .parse::<Config>()
            .unwrap_err();
        assert!(matches!(err, ConfigError::EmptySequence { .. }));
        let err = device_with(
            r#"{ button = "BTN_SIDE", mode = "press", macro = { actions = [{ tap = "KEY_A" }] }, repeat = 0 }"#,
        )
        .parse::<Config>()
        .unwrap_err();
        assert!(matches!(err, ConfigError::ZeroRepeat { .. }));
    }

    #[test]
    fn rejects_typos() {
        // nombre de tecla inexistente, modo inexistente y campo desconocido
        for bad in [
            r#"{ button = "BTN_NOPE", mode = "remap", to = ["KEY_A"] }"#,
            r#"{ button = "BTN_SIDE", mode = "turbo", macro = "x" }"#,
            r#"{ button = "BTN_SIDE", mode = "remap", to = ["KEY_A"], repat = 2 }"#,
        ] {
            assert!(device_with(bad).parse::<Config>().is_err(), "{bad}");
        }
    }
}
