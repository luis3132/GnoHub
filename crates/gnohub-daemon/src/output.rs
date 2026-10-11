//! Dispositivos virtuales (uinput) por los que GnoHub emite eventos.

use std::io;
use std::sync::{Arc, Mutex, PoisonError};

use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, Device, InputEvent, KeyCode, RelativeAxisCode, UinputAbsSetup};

use gnohub_core::devices::VIRTUAL_PREFIX;

/// Límite de longitud de nombre de uinput (`UINPUT_MAX_NAME_SIZE` menos el NUL).
const MAX_NAME_LEN: usize = 79;

fn virtual_name(suffix: &str) -> String {
    let mut name = format!("{VIRTUAL_PREFIX} {suffix}");
    if name.len() > MAX_NAME_LEN {
        let mut end = MAX_NAME_LEN;
        while !name.is_char_boundary(end) {
            end -= 1;
        }
        name.truncate(end);
    }
    name
}

/// Clona las capacidades de un dispositivo físico para reenviar lo que no se remapea.
///
/// Se conserva el `input_id` original para que libinput y udev le apliquen los
/// mismos ajustes (hwdb, DPI, etc.) que al dispositivo real.
pub fn passthrough_for(device: &Device) -> io::Result<VirtualDevice> {
    let name = virtual_name(device.name().unwrap_or("dispositivo"));
    let mut builder = VirtualDevice::builder()?
        .name(&name)
        .input_id(device.input_id())
        .with_properties(device.properties())?;
    if let Some(keys) = device.supported_keys() {
        builder = builder.with_keys(keys)?;
    }
    if let Some(axes) = device.supported_relative_axes() {
        builder = builder.with_relative_axes(axes)?;
    }
    if let Some(misc) = device.misc_properties() {
        builder = builder.with_msc(misc)?;
    }
    for (axis, info) in device.get_absinfo()? {
        builder = builder.with_absolute_axis(&UinputAbsSetup::new(axis, info))?;
    }
    builder.build()
}

/// Dispositivo virtual compartido por el que salen las macros y los remapeos.
#[derive(Clone)]
pub struct Output(Arc<Mutex<VirtualDevice>>);

impl Output {
    /// Declara el teclado estándar y los botones de ratón, más cualquier tecla
    /// extra que use la configuración. No se declaran rangos de joystick o
    /// tableta para que udev no lo clasifique como tal.
    pub fn new(extra_keys: impl IntoIterator<Item = KeyCode>) -> io::Result<Self> {
        let mut keys = AttributeSet::<KeyCode>::new();
        for code in KeyCode::KEY_ESC.code()..=KeyCode::KEY_MICMUTE.code() {
            keys.insert(KeyCode::new(code));
        }
        for code in KeyCode::BTN_LEFT.code()..=KeyCode::BTN_TASK.code() {
            keys.insert(KeyCode::new(code));
        }
        for key in extra_keys {
            keys.insert(key);
        }

        let mut axes = AttributeSet::<RelativeAxisCode>::new();
        for axis in [
            RelativeAxisCode::REL_X,
            RelativeAxisCode::REL_Y,
            RelativeAxisCode::REL_WHEEL,
            RelativeAxisCode::REL_HWHEEL,
        ] {
            axes.insert(axis);
        }

        let device = VirtualDevice::builder()?
            .name(&virtual_name("virtual input"))
            .with_keys(&keys)?
            .with_relative_axes(&axes)?
            .build()?;
        Ok(Self(Arc::new(Mutex::new(device))))
    }

    pub fn emit(&self, events: &[InputEvent]) -> io::Result<()> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .emit(events)
    }
}
