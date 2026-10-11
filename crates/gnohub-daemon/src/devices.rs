//! Comandos `list` y `watch`, y utilidades de dispositivos para el daemon.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use evdev::{Device, EventSummary, RelativeAxisCode};
pub use gnohub_core::devices::{
    DeviceKind, event_nodes, kind_of, physical_devices, unreadable_count,
};

pub fn permission_hint() -> String {
    match unreadable_count() {
        0 => String::new(),
        n => format!(
            "{n} dispositivos sin permiso de lectura; consulta la sección «Permisos» del README"
        ),
    }
}

pub fn list() {
    for (path, device) in physical_devices() {
        let id = device.input_id();
        let buttons = device.supported_keys().map_or(0, |k| k.iter().count());
        println!(
            "{:<20} {:04x}:{:04x}  {:<45} [{}, {} teclas/botones]",
            path.display(),
            id.vendor(),
            id.product(),
            device.name().unwrap_or("?"),
            match kind_of(&device) {
                DeviceKind::Mouse => "ratón",
                DeviceKind::Keyboard => "teclado",
                DeviceKind::Other => "otro",
            },
            buttons,
        );
    }
    let hint = permission_hint();
    if !hint.is_empty() {
        eprintln!("\naviso: {hint}");
    }
}

/// Resuelve `VID:PID` (hex) o una ruta `/dev/input/eventN` a los nodos correspondientes.
fn resolve_target(target: &str) -> Result<Vec<(PathBuf, Device)>> {
    if target.starts_with('/') {
        let device = Device::open(target).with_context(|| format!("no se pudo abrir {target}"))?;
        return Ok(vec![(PathBuf::from(target), device)]);
    }
    let (vid, pid) = target
        .split_once(':')
        .context("formato esperado VID:PID, p. ej. 046d:c08b")?;
    let vid = u16::from_str_radix(vid, 16).context("VID no es hexadecimal")?;
    let pid = u16::from_str_radix(pid, 16).context("PID no es hexadecimal")?;
    let nodes: Vec<_> = physical_devices()
        .into_iter()
        .filter(|(_, d)| d.input_id().vendor() == vid && d.input_id().product() == pid)
        .collect();
    if nodes.is_empty() {
        bail!(
            "no hay dispositivos {vid:04x}:{pid:04x} accesibles. {}",
            permission_hint()
        );
    }
    Ok(nodes)
}

fn node_label(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// Imprime los eventos de botones y ruedas sin capturar el dispositivo, para
/// descubrir qué nombre usar en la configuración.
pub async fn watch(target: &str) -> Result<()> {
    let nodes = resolve_target(target)?;
    for (path, device) in nodes {
        let label = node_label(&path);
        println!("escuchando {label}: {}", device.name().unwrap_or("?"));
        let mut stream = device.into_event_stream()?;
        tokio::spawn(async move {
            loop {
                let ev = match stream.next_event().await {
                    Ok(ev) => ev,
                    Err(e) => {
                        eprintln!("[{label}] desconectado: {e}");
                        return;
                    }
                };
                match ev.destructure() {
                    EventSummary::Key(_, key, value) => {
                        let state = match value {
                            0 => "soltado",
                            1 => "pulsado",
                            _ => "repetición",
                        };
                        println!("[{label}] {key:?} {state}");
                    }
                    EventSummary::RelativeAxis(_, axis, value)
                        if !matches!(axis, RelativeAxisCode::REL_X | RelativeAxisCode::REL_Y) =>
                    {
                        println!("[{label}] {axis:?} {value}");
                    }
                    _ => {}
                }
            }
        });
    }
    println!("pulsa los botones del dispositivo; Ctrl+C para salir\n");
    tokio::signal::ctrl_c().await?;
    Ok(())
}
