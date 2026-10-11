//! Captura los dispositivos configurados y reparte sus eventos entre el
//! reenvío normal y las asignaciones.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use anyhow::{Context, Result};
use evdev::{Device, EventSummary, SynchronizationCode};
use gnohub_core::{Binding, Button, Config, DeviceConfig, Wheel};
use tokio::signal::unix::{SignalKind, signal};
use tokio::task::JoinSet;
use tracing::{debug, error, info, warn};

use crate::devices;
use crate::executor::Runner;
use crate::focus::{self, Focus};
use crate::output::{self, Output};

/// Cuánto esperar a que se suelten los botones antes de capturar un dispositivo.
const RELEASE_WAIT: Duration = Duration::from_secs(3);
/// Cada cuánto se comprueba si cambió la configuración o se conectó algo.
const POLL: Duration = Duration::from_secs(1);

/// Identifica una versión del archivo de configuración (o su ausencia).
type Stamp = Option<(SystemTime, u64)>;

fn stamp(path: &Path) -> Stamp {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

/// Carga la configuración; si no existe se arranca sin asignaciones y si es
/// inválida se informa y se sigue con la anterior.
fn load(path: &Path) -> Option<Config> {
    if !path.exists() {
        warn!("{} no existe; esperando a que se cree", path.display());
        return Some(Config::default());
    }
    match Config::load(path) {
        Ok(config) => Some(config),
        Err(e) => {
            error!("configuración inválida, se ignora: {e}");
            None
        }
    }
}

/// Bucle principal: aplica la configuración, la recarga cuando el archivo
/// cambia y captura los dispositivos que se conecten después de arrancar.
pub async fn run(path: PathBuf) -> Result<()> {
    let mut last_stamp = stamp(&path);
    let focus = focus::spawn();
    let mut session = Session::new(load(&path).unwrap_or_default(), focus.clone())?;
    session.attach_new();
    let mut last_nodes = devices::event_nodes();

    let mut sigterm = signal(SignalKind::terminate())?;
    let mut tick = tokio::time::interval(POLL);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            _ = sigterm.recv() => break,
            Some(res) = session.nodes.join_next(), if !session.nodes.is_empty() => {
                session.finished(res);
            }
            _ = tick.tick() => {
                let current = stamp(&path);
                if current != last_stamp {
                    last_stamp = current;
                    if let Some(config) = load(&path) {
                        info!("configuración cambiada; recargando");
                        session.shutdown().await;
                        session = Session::new(config, focus.clone())?;
                        session.attach_new();
                    }
                }
                let nodes = devices::event_nodes();
                if nodes != last_nodes {
                    last_nodes = nodes;
                    session.attach_new();
                }
            }
        }
    }
    info!("saliendo; se liberan los dispositivos");
    session.shutdown().await;
    Ok(())
}

/// Una configuración aplicada: su dispositivo de salida y los nodos capturados.
struct Session {
    devices: Vec<Arc<DeviceConfig>>,
    output: Output,
    focus: Focus,
    nodes: JoinSet<(PathBuf, Result<()>)>,
    active: HashSet<PathBuf>,
    warned_empty: bool,
}

impl Session {
    fn new(config: Config, focus: Focus) -> Result<Self> {
        let output = Output::new(config.output_keys())
            .context("no se pudo crear el dispositivo virtual (¿acceso a /dev/uinput?)")?;
        Ok(Self {
            devices: config.devices.into_iter().map(Arc::new).collect(),
            output,
            focus,
            nodes: JoinSet::new(),
            active: HashSet::new(),
            warned_empty: false,
        })
    }

    /// Captura los nodos configurados que aún no estén capturados.
    fn attach_new(&mut self) {
        if self.devices.is_empty() {
            return;
        }
        for (path, device) in devices::physical_devices() {
            if self.active.contains(&path) {
                continue;
            }
            let id = device.input_id();
            let Some(config) = self
                .devices
                .iter()
                .find(|d| d.matches(id.vendor(), id.product()))
            else {
                continue;
            };
            // Solo se captura el nodo que realmente emite alguno de los botones
            // asignados; el resto del dispositivo sigue funcionando sin pasar por aquí.
            let handles_any = config.buttons().any(|b| match b {
                Button::Key(k) => device.supported_keys().is_some_and(|keys| keys.contains(k)),
                Button::Wheel(w) => device
                    .supported_relative_axes()
                    .is_some_and(|axes| axes.contains(w.axes()[0])),
            });
            if !handles_any {
                debug!("{}: ningún botón asignado en este nodo", path.display());
                continue;
            }
            let (config, output, focus) =
                (Arc::clone(config), self.output.clone(), self.focus.clone());
            self.active.insert(path.clone());
            self.nodes.spawn(async move {
                let res = handle_node(&path, device, config, output, focus).await;
                (path, res)
            });
        }
        let any_binding = self.devices.iter().any(|d| d.buttons().next().is_some());
        if self.active.is_empty() && any_binding && !self.warned_empty {
            self.warned_empty = true;
            let hint = devices::permission_hint();
            warn!("ningún dispositivo configurado está conectado o accesible. {hint}");
        }
    }

    fn finished(&mut self, res: Result<(PathBuf, Result<()>), tokio::task::JoinError>) {
        match res {
            Ok((path, result)) => {
                self.active.remove(&path);
                if let Err(e) = result {
                    warn!("{e:#}");
                }
            }
            Err(e) => error!("tarea de dispositivo abortada: {e}"),
        }
    }

    /// Detiene todas las capturas y espera a que se liberen, para que una
    /// sesión nueva pueda volver a capturar los mismos dispositivos.
    async fn shutdown(&mut self) {
        // Al soltar los `Device` el kernel libera la captura, y al destruir el
        // dispositivo virtual suelta cualquier tecla que siguiera pulsada.
        self.nodes.shutdown().await;
        self.active.clear();
    }
}

/// Espera a que no haya teclas pulsadas, para no capturar el dispositivo con un
/// botón a medias: su "soltar" iría a nuestro reenvío y la pulsación original
/// quedaría atascada en el sistema.
async fn wait_for_release(device: &Device) -> Result<()> {
    let start = Instant::now();
    while device.get_key_state()?.iter().next().is_some() {
        if start.elapsed() > RELEASE_WAIT {
            warn!("hay botones pulsados; se captura igualmente");
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    Ok(())
}

async fn handle_node(
    path: &Path,
    mut device: Device,
    config: Arc<DeviceConfig>,
    output: Output,
    focus: Focus,
) -> Result<()> {
    let label = config.label();
    let node = path.display().to_string();
    let mut passthrough = output::passthrough_for(&device)
        .with_context(|| format!("{node}: no se pudo crear el reenvío"))?;
    wait_for_release(&device).await?;
    device
        .grab()
        .with_context(|| format!("{node}: no se pudo capturar (¿otro programa lo tiene?)"))?;
    info!(
        "{label}: capturado {node} ({} perfiles de aplicación)",
        config.apps.len()
    );

    let mut stream = device.into_event_stream()?;
    let mut runner = Runner::new(output);
    let mut frame = Vec::new();
    // Asignación con la que se pulsó cada botón: al soltarlo se usa la misma
    // aunque entretanto haya cambiado la ventana activa.
    let mut pressed: HashMap<Button, Binding> = HashMap::new();
    loop {
        let ev = stream
            .next_event()
            .await
            .with_context(|| format!("{label}: {node} desconectado"))?;
        match ev.destructure() {
            EventSummary::Synchronization(_, SynchronizationCode::SYN_REPORT, _) => {
                if !frame.is_empty() {
                    passthrough.emit(&frame)?;
                    frame.clear();
                }
            }
            EventSummary::Key(_, key, value) => {
                let button = Button::Key(key);
                let binding = match value {
                    1 => {
                        let found = config.lookup(button, focus.borrow().as_ref()).cloned();
                        match &found {
                            Some(b) => pressed.insert(button, b.clone()),
                            None => pressed.remove(&button),
                        };
                        found
                    }
                    0 => pressed.remove(&button),
                    _ => pressed.get(&button).cloned(),
                };
                match binding {
                    Some(b) => runner.handle(button, &b, value),
                    None => frame.push(ev),
                }
            }
            EventSummary::RelativeAxis(_, axis, value) => {
                let wheel = Wheel::from_event(axis, value).map(Button::Wheel);
                let binding =
                    wheel.and_then(|w| config.lookup(w, focus.borrow().as_ref()).cloned());
                match (wheel, binding) {
                    // La rueda no tiene "soltar": cada clic de rueda cuenta como
                    // pulsar y soltar. Los eventos de alta resolución del mismo
                    // giro se descartan para que no se cuele el desplazamiento.
                    (Some(button), Some(b)) => {
                        if Wheel::is_notch_axis(axis) {
                            for _ in 0..value.unsigned_abs() {
                                runner.handle(button, &b, 1);
                                runner.handle(button, &b, 0);
                            }
                        }
                    }
                    _ => frame.push(ev),
                }
            }
            _ => frame.push(ev),
        }
    }
}
