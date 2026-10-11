//! Ejecuta las asignaciones de un dispositivo según los eventos de sus botones.
//!
//! Cada pulsación de un botón con macro lanza una tarea que recibe un
//! `watch::Receiver<bool>` con el estado "activo" (botón mantenido, o toggle
//! encendido). Así todos los modos se expresan como una tarea que reacciona a
//! ese estado, y al soltar o desactivar solo hay que enviar `false`.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use evdev::{EventType, KeyCode};
use gnohub_core::action::key_event;
use gnohub_core::{Binding, Button, Macro, Step};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::{debug, warn};

use crate::output::Output;

type Active = watch::Receiver<bool>;

struct Running {
    active: watch::Sender<bool>,
    task: JoinHandle<()>,
}

impl Running {
    fn is_done(&self) -> bool {
        self.task.is_finished()
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        // Al desconectar el dispositivo se detienen sus bucles; la tarea suelta
        // las teclas que tuviera pulsadas antes de terminar.
        let _ = self.active.send(false);
    }
}

pub struct Runner {
    output: Output,
    running: HashMap<Button, Running>,
}

impl Runner {
    pub fn new(output: Output) -> Self {
        Self {
            output,
            running: HashMap::new(),
        }
    }

    /// Procesa un evento del botón `button` (`value`: 1 pulsado, 0 soltado, 2 autorepetición).
    pub fn handle(&mut self, button: Button, binding: &Binding, value: i32) {
        match value {
            1 => self.press(button, binding),
            0 => self.release(button, binding),
            _ => {}
        }
    }

    fn press(&mut self, button: Button, binding: &Binding) {
        if let Binding::Native | Binding::Disabled = binding {
            return;
        }
        if let Binding::Remap(keys) = binding {
            for key in keys {
                self.emit_key(*key, true);
            }
            return;
        }

        let busy = self.running.get(&button).is_some_and(|r| !r.is_done());
        if let Binding::Toggle { .. } = binding
            && busy
        {
            debug!(?button, "toggle desactivado");
            self.running.remove(&button);
            return;
        }
        if busy {
            // La ejecución anterior (p. ej. la parte `on_release`) aún no ha
            // terminado; mezclar dos ejecuciones dejaría teclas en mal estado.
            debug!(?button, "macro en curso, pulsación ignorada");
            return;
        }

        let (active, rx) = watch::channel(true);
        let task = tokio::spawn(run_binding(
            Player::new(self.output.clone()),
            binding.clone(),
            rx,
        ));
        self.running.insert(button, Running { active, task });
    }

    fn release(&mut self, button: Button, binding: &Binding) {
        match binding {
            Binding::Remap(keys) => {
                for key in keys.iter().rev() {
                    self.emit_key(*key, false);
                }
            }
            // El toggle ignora las sueltas: solo cambia con la siguiente pulsación.
            Binding::Native | Binding::Disabled | Binding::Toggle { .. } => {}
            _ => {
                if let Some(r) = self.running.get(&button) {
                    let _ = r.active.send(false);
                }
            }
        }
    }

    fn emit_key(&self, key: KeyCode, pressed: bool) {
        if let Err(e) = self.output.emit(&[key_event(key, pressed)]) {
            warn!("no se pudo emitir {key:?}: {e}");
        }
    }
}

async fn run_binding(mut player: Player, binding: Binding, mut active: Active) {
    match binding {
        Binding::Native | Binding::Disabled | Binding::Remap(_) => {
            unreachable!("no generan tareas")
        }
        Binding::Press {
            r#macro,
            times,
            gap,
        } => {
            for i in 0..times {
                if i > 0 {
                    tokio::time::sleep(gap).await;
                }
                player.play(&r#macro, None).await;
            }
        }
        Binding::WhileHeld { r#macro, gap } | Binding::Toggle { r#macro, gap } => {
            player.repeat(&r#macro, gap, &mut active).await;
        }
        Binding::Sequence {
            on_press,
            while_held,
            on_release,
            gap,
        } => {
            if let Some(m) = on_press {
                player.play(&m, None).await;
            }
            if let Some(m) = while_held {
                player.repeat(&m, gap, &mut active).await;
            }
            until_inactive(&mut active).await;
            if let Some(m) = on_release {
                player.play(&m, None).await;
            }
        }
        Binding::TapHold {
            tap,
            hold,
            hold_time,
            hold_repeat,
            gap,
        } => {
            tokio::select! {
                () = until_inactive(&mut active) => {
                    player.play(&tap, None).await;
                }
                () = tokio::time::sleep(hold_time) => {
                    if hold_repeat {
                        player.repeat(&hold, gap, &mut active).await;
                    } else {
                        player.play(&hold, None).await;
                    }
                }
            }
        }
    }
    player.release_all();
}

fn is_active(active: &Active) -> bool {
    *active.borrow() && active.has_changed().is_ok()
}

/// Termina cuando el estado pasa a inactivo o el emisor desaparece.
async fn until_inactive(active: &mut Active) {
    let _ = active.wait_for(|a| !*a).await;
}

/// Reproduce pasos de macro y recuerda qué teclas deja pulsadas, para soltarlas
/// si la macro se interrumpe a mitad.
struct Player {
    output: Output,
    held: HashSet<u16>,
}

impl Player {
    fn new(output: Output) -> Self {
        Self {
            output,
            held: HashSet::new(),
        }
    }

    /// Ejecuta la macro una vez. Con `stop`, se interrumpe en cuanto pasa a inactivo.
    /// Devuelve `false` si se interrumpió.
    async fn play(&mut self, r#macro: &Macro, mut stop: Option<&mut Active>) -> bool {
        for step in &r#macro.steps {
            if stop.as_deref().is_some_and(|s| !is_active(s)) {
                return false;
            }
            match step {
                Step::Emit(events) => {
                    for ev in events.iter().filter(|e| e.event_type() == EventType::KEY) {
                        match ev.value() {
                            0 => self.held.remove(&ev.code()),
                            _ => self.held.insert(ev.code()),
                        };
                    }
                    if let Err(e) = self.output.emit(events) {
                        warn!(r#macro = %r#macro.name, "error emitiendo eventos: {e}");
                        return false;
                    }
                }
                Step::Sleep(d) => {
                    if !self.sleep(*d, stop.as_deref_mut()).await {
                        return false;
                    }
                }
                Step::Spawn(cmd) => spawn_command(cmd),
            }
        }
        true
    }

    /// Repite la macro, con `gap` entre repeticiones, mientras siga activa.
    async fn repeat(&mut self, r#macro: &Macro, gap: Duration, active: &mut Active) {
        let held_before = self.held.clone();
        while is_active(active) {
            if !self.play(r#macro, Some(active)).await {
                break;
            }
            if !self.sleep(gap, Some(active)).await {
                break;
            }
        }
        // Una iteración interrumpida puede dejar teclas pulsadas (p. ej. un `down`
        // sin su `up`). Se sueltan, pero no las que dejó pulsadas `on_press`:
        // esas le corresponden a `on_release`.
        let stray: Vec<u16> = self.held.difference(&held_before).copied().collect();
        for code in stray {
            self.release(code);
        }
    }

    /// Duerme `d`; devuelve `false` si `stop` pasó a inactivo antes.
    async fn sleep(&mut self, d: Duration, stop: Option<&mut Active>) -> bool {
        match stop {
            None => {
                tokio::time::sleep(d).await;
                true
            }
            Some(stop) => tokio::select! {
                () = tokio::time::sleep(d) => true,
                () = until_inactive(stop) => false,
            },
        }
    }

    fn release(&mut self, code: u16) {
        self.held.remove(&code);
        if let Err(e) = self.output.emit(&[key_event(KeyCode::new(code), false)]) {
            warn!("no se pudo soltar la tecla {code}: {e}");
        }
    }

    fn release_all(&mut self) {
        let held: Vec<u16> = self.held.iter().copied().collect();
        for code in held {
            self.release(code);
        }
    }
}

fn spawn_command(cmd: &str) {
    let child = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdin(std::process::Stdio::null())
        .spawn();
    match child {
        // Se espera en segundo plano para no dejar procesos zombi.
        Ok(mut child) => {
            tokio::spawn(async move {
                let _ = child.wait().await;
            });
        }
        Err(e) => warn!("no se pudo ejecutar `{cmd}`: {e}"),
    }
}
