//! Acciones que componen una macro y su traducción a pasos de bajo nivel.

use std::time::Duration;

use evdev::{EventType, InputEvent, KeyCode, RelativeAxisCode};
use serde::{Deserialize, Serialize};

/// Una acción individual dentro de una macro.
///
/// En TOML se escribe como tabla de una sola clave, p. ej. `{ tap = "KEY_A" }`
/// o `{ delay = 50 }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Mantiene pulsada una tecla o botón.
    Down(KeyCode),
    /// Suelta una tecla o botón.
    Up(KeyCode),
    /// Pulsa y suelta.
    Tap(KeyCode),
    /// Combinación: pulsa en orden y suelta en orden inverso (p. ej. Ctrl+Shift+T).
    Chord(Vec<KeyCode>),
    /// Espera en milisegundos.
    Delay(u64),
    /// Movimiento relativo del puntero `[dx, dy]`.
    Move([i32; 2]),
    /// Rueda vertical (positivo = arriba).
    Scroll(i32),
    /// Ejecuta un comando con `sh -c`, sin esperar a que termine.
    Command(String),
}

impl Action {
    /// Teclas que esta acción puede emitir, para declararlas en el dispositivo virtual.
    pub fn keys(&self) -> &[KeyCode] {
        match self {
            Action::Down(k) | Action::Up(k) | Action::Tap(k) => std::slice::from_ref(k),
            Action::Chord(keys) => keys,
            _ => &[],
        }
    }
}

/// Paso ya resuelto, listo para que el ejecutor lo procese.
#[derive(Debug, Clone)]
pub enum Step {
    /// Un frame de eventos; quien lo emite añade el `SYN_REPORT`.
    Emit(Vec<InputEvent>),
    Sleep(Duration),
    Spawn(String),
}

pub fn key_event(key: KeyCode, pressed: bool) -> InputEvent {
    InputEvent::new(EventType::KEY.0, key.code(), i32::from(pressed))
}

fn rel_event(axis: RelativeAxisCode, value: i32) -> InputEvent {
    InputEvent::new(EventType::RELATIVE.0, axis.0, value)
}

/// Convierte una lista de acciones en pasos ejecutables.
///
/// Entre dos pasos consecutivos se inserta `step_delay`, salvo que el usuario
/// haya puesto un `delay` explícito en ese punto. Cada pulsación va en su
/// propio frame para que las aplicaciones vean los modificadores antes que la
/// tecla que acompañan.
pub fn expand(actions: &[Action], step_delay: Duration) -> Vec<Step> {
    let mut atoms = Vec::new();
    for action in actions {
        match action {
            Action::Down(k) => atoms.push(Step::Emit(vec![key_event(*k, true)])),
            Action::Up(k) => atoms.push(Step::Emit(vec![key_event(*k, false)])),
            Action::Tap(k) => {
                atoms.push(Step::Emit(vec![key_event(*k, true)]));
                atoms.push(Step::Emit(vec![key_event(*k, false)]));
            }
            Action::Chord(keys) => {
                atoms.extend(keys.iter().map(|k| Step::Emit(vec![key_event(*k, true)])));
                atoms.extend(
                    keys.iter()
                        .rev()
                        .map(|k| Step::Emit(vec![key_event(*k, false)])),
                );
            }
            Action::Delay(ms) => atoms.push(Step::Sleep(Duration::from_millis(*ms))),
            Action::Move([dx, dy]) => {
                let frame: Vec<_> = [
                    (RelativeAxisCode::REL_X, *dx),
                    (RelativeAxisCode::REL_Y, *dy),
                ]
                .into_iter()
                .filter(|(_, v)| *v != 0)
                .map(|(axis, v)| rel_event(axis, v))
                .collect();
                if !frame.is_empty() {
                    atoms.push(Step::Emit(frame));
                }
            }
            Action::Scroll(n) => {
                atoms.push(Step::Emit(vec![rel_event(RelativeAxisCode::REL_WHEEL, *n)]))
            }
            Action::Command(cmd) => atoms.push(Step::Spawn(cmd.clone())),
        }
    }

    let mut steps = Vec::with_capacity(atoms.len() * 2);
    let mut atoms = atoms.into_iter().peekable();
    while let Some(atom) = atoms.next() {
        let is_sleep = matches!(atom, Step::Sleep(_));
        steps.push(atom);
        if !is_sleep
            && atoms
                .peek()
                .is_some_and(|next| !matches!(next, Step::Sleep(_)))
        {
            steps.push(Step::Sleep(step_delay));
        }
    }
    steps
}

#[cfg(test)]
mod tests {
    use super::*;

    fn emitted(steps: &[Step]) -> Vec<(u16, u16, i32)> {
        steps
            .iter()
            .filter_map(|s| match s {
                Step::Emit(evs) => {
                    Some(evs.iter().map(|e| (e.event_type().0, e.code(), e.value())))
                }
                _ => None,
            })
            .flatten()
            .collect()
    }

    const STEP: Duration = Duration::from_millis(5);

    fn sleeps(steps: &[Step]) -> Vec<u64> {
        steps
            .iter()
            .filter_map(|s| match s {
                Step::Sleep(d) => Some(d.as_millis() as u64),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn chord_releases_in_reverse_order() {
        let steps = expand(
            &[Action::Chord(vec![KeyCode::KEY_LEFTCTRL, KeyCode::KEY_C])],
            STEP,
        );
        let key = EventType::KEY.0;
        assert_eq!(
            emitted(&steps),
            vec![
                (key, KeyCode::KEY_LEFTCTRL.code(), 1),
                (key, KeyCode::KEY_C.code(), 1),
                (key, KeyCode::KEY_C.code(), 0),
                (key, KeyCode::KEY_LEFTCTRL.code(), 0),
            ]
        );
    }

    #[test]
    fn move_skips_zero_axes() {
        let steps = expand(&[Action::Move([0, 5])], STEP);
        assert_eq!(
            emitted(&steps),
            vec![(EventType::RELATIVE.0, RelativeAxisCode::REL_Y.0, 5)]
        );
    }

    #[test]
    fn step_delay_between_every_step_but_not_at_the_end() {
        let steps = expand(
            &[Action::Tap(KeyCode::KEY_A), Action::Tap(KeyCode::KEY_B)],
            STEP,
        );
        // down a, up a, down b, up b → 3 huecos
        assert_eq!(sleeps(&steps), vec![5, 5, 5]);
        assert!(matches!(steps.last(), Some(Step::Emit(_))));
    }

    #[test]
    fn explicit_delay_replaces_step_delay() {
        let steps = expand(
            &[
                Action::Tap(KeyCode::KEY_A),
                Action::Delay(100),
                Action::Command("true".into()),
            ],
            STEP,
        );
        assert_eq!(sleeps(&steps), vec![5, 100]);
        assert!(matches!(steps.last(), Some(Step::Spawn(c)) if c == "true"));
    }
}
