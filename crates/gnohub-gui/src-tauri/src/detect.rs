//! Detección del botón que pulsa el usuario, para asignarlo sin tener que
//! conocer su nombre. También detecta la inclinación de la rueda.

use std::time::Duration;

use evdev::EventSummary;
use gnohub_core::{Button, Wheel, devices};
use tokio::sync::mpsc;

/// Escucha (sin capturar) todos los nodos del dispositivo y devuelve el primer
/// botón o tecla pulsado, o `None` si se agota el tiempo.
pub async fn first_press(
    vendor: u16,
    product: u16,
    timeout: Duration,
) -> Result<Option<Button>, String> {
    let nodes: Vec<_> = devices::physical_devices()
        .into_iter()
        .filter(|(_, d)| d.input_id().vendor() == vendor && d.input_id().product() == product)
        .collect();
    if nodes.is_empty() {
        return Err(rust_i18n::t!(
            "detect.unreadable",
            id = format!("{vendor:04x}:{product:04x}")
        )
        .into_owned());
    }

    let (tx, mut rx) = mpsc::channel(1);
    let mut tasks = Vec::new();
    for (path, device) in nodes {
        let tx = tx.clone();
        let mut stream = device
            .into_event_stream()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        tasks.push(tokio::spawn(async move {
            while let Ok(ev) = stream.next_event().await {
                let button = match ev.destructure() {
                    EventSummary::Key(_, key, 1) => Button::Key(key),
                    // Solo la inclinación: el giro normal se dispara sin querer al mover el ratón.
                    EventSummary::RelativeAxis(_, axis, value) => {
                        match Wheel::from_event(axis, value) {
                            Some(w @ (Wheel::Left | Wheel::Right)) => Button::Wheel(w),
                            _ => continue,
                        }
                    }
                    _ => continue,
                };
                let _ = tx.send(button).await;
                return;
            }
        }));
    }
    drop(tx);

    let result = tokio::time::timeout(timeout, rx.recv())
        .await
        .ok()
        .flatten();
    for task in tasks {
        task.abort();
    }
    Ok(result)
}
