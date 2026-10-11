//! Lo que se puede asignar en un dispositivo: una tecla o botón, o un
//! movimiento de rueda (incluida la inclinación lateral).

use std::fmt;
use std::str::FromStr;

use evdev::{KeyCode, RelativeAxisCode};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::file::{key_name, parse_key};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Wheel {
    Up,
    Down,
    /// Inclinar la rueda a la izquierda (o rueda horizontal hacia la izquierda).
    Left,
    Right,
}

impl Wheel {
    pub const ALL: [Wheel; 4] = [Wheel::Up, Wheel::Down, Wheel::Left, Wheel::Right];

    pub fn name(self) -> &'static str {
        match self {
            Wheel::Up => "WHEEL_UP",
            Wheel::Down => "WHEEL_DOWN",
            Wheel::Left => "WHEEL_LEFT",
            Wheel::Right => "WHEEL_RIGHT",
        }
    }

    /// Eje del que sale esta dirección, en resolución normal y alta.
    pub fn axes(self) -> [RelativeAxisCode; 2] {
        match self {
            Wheel::Up | Wheel::Down => [
                RelativeAxisCode::REL_WHEEL,
                RelativeAxisCode::REL_WHEEL_HI_RES,
            ],
            Wheel::Left | Wheel::Right => [
                RelativeAxisCode::REL_HWHEEL,
                RelativeAxisCode::REL_HWHEEL_HI_RES,
            ],
        }
    }

    /// Dirección de un evento de rueda (de cualquier resolución), si lo es.
    pub fn from_event(axis: RelativeAxisCode, value: i32) -> Option<Wheel> {
        let vertical = matches!(
            axis,
            RelativeAxisCode::REL_WHEEL | RelativeAxisCode::REL_WHEEL_HI_RES
        );
        let horizontal = matches!(
            axis,
            RelativeAxisCode::REL_HWHEEL | RelativeAxisCode::REL_HWHEEL_HI_RES
        );
        match (vertical, horizontal, value.signum()) {
            (true, _, 1) => Some(Wheel::Up),
            (true, _, -1) => Some(Wheel::Down),
            (_, true, 1) => Some(Wheel::Right),
            (_, true, -1) => Some(Wheel::Left),
            _ => None,
        }
    }

    /// ¿Es este eje el de resolución normal (uno por "clic" de rueda)?
    pub fn is_notch_axis(axis: RelativeAxisCode) -> bool {
        matches!(
            axis,
            RelativeAxisCode::REL_WHEEL | RelativeAxisCode::REL_HWHEEL
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Button {
    Key(KeyCode),
    Wheel(Wheel),
}

impl Button {
    pub fn name(self) -> Option<String> {
        match self {
            Button::Key(k) => key_name(k),
            Button::Wheel(w) => Some(w.name().to_owned()),
        }
    }
}

impl From<KeyCode> for Button {
    fn from(key: KeyCode) -> Self {
        Button::Key(key)
    }
}

impl fmt::Display for Button {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Button::Key(k) => match key_name(*k) {
                Some(name) => f.write_str(&name),
                None => write!(f, "código {}", k.code()),
            },
            Button::Wheel(w) => f.write_str(w.name()),
        }
    }
}

#[derive(Debug)]
pub struct UnknownButton(String);

impl fmt::Display for UnknownButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&rust_i18n::t!("button.unknown", name = self.0))
    }
}

impl std::error::Error for UnknownButton {}

impl FromStr for Button {
    type Err = UnknownButton;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let upper = s.trim().to_ascii_uppercase();
        if let Some(w) = Wheel::ALL.into_iter().find(|w| w.name() == upper) {
            return Ok(Button::Wheel(w));
        }
        parse_key(&upper)
            .map(Button::Key)
            .ok_or_else(|| UnknownButton(s.to_owned()))
    }
}

impl Serialize for Button {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.name() {
            Some(name) => serializer.serialize_str(&name),
            None => Err(serde::ser::Error::custom(format!("{self} no tiene nombre"))),
        }
    }
}

impl<'de> Deserialize<'de> for Button {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_keys_and_wheel_case_insensitive() {
        assert_eq!(
            "btn_side".parse::<Button>().unwrap(),
            Button::Key(KeyCode::BTN_SIDE)
        );
        assert_eq!(
            "Wheel_Left".parse::<Button>().unwrap(),
            Button::Wheel(Wheel::Left)
        );
        assert!("BTN_NOPE".parse::<Button>().is_err());
    }

    #[test]
    fn wheel_direction_from_events() {
        use RelativeAxisCode as R;
        assert_eq!(Wheel::from_event(R::REL_HWHEEL, -1), Some(Wheel::Left));
        assert_eq!(
            Wheel::from_event(R::REL_HWHEEL_HI_RES, 120),
            Some(Wheel::Right)
        );
        assert_eq!(Wheel::from_event(R::REL_WHEEL, 2), Some(Wheel::Up));
        assert_eq!(Wheel::from_event(R::REL_X, 5), None);
        assert_eq!(Wheel::from_event(R::REL_WHEEL, 0), None);
    }
}
