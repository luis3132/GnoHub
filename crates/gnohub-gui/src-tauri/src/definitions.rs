//! Definiciones de periféricos: archivos JSON que describen un modelo (sus
//! botones, qué envía cada uno, dónde está en la imagen…). Así añadir un
//! dispositivo nuevo no requiere tocar código.
//!
//! Se buscan, por orden de prioridad (la primera que coincida gana):
//! 1. `~/.config/gnohub/devices/`  — las tuyas
//! 2. `~/.local/share/gnohub/devices/` y `$XDG_DATA_DIRS/gnohub/devices/` — las instaladas
//! 3. `devices/` del proyecto (solo en desarrollo)
//!
//! El formato está documentado en `devices/README.md` y `devices/schema.json`.

use std::path::{Path, PathBuf};

use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    #[serde(rename = "$schema", default, skip_serializing)]
    _schema: Option<String>,
    pub name: String,
    /// "mouse", "keyboard", "gamepad" u "other".
    #[serde(default = "default_kind")]
    pub kind: String,
    /// Modelos a los que se aplica, como "046d:c08b". En el JSON se llama
    /// `match`; hacia la interfaz se envía como `matches`.
    #[serde(rename(deserialize = "match"))]
    pub matches: Vec<String>,
    /// Imagen relativa al JSON. Un PNG/JPG/WebP se entrega como `data:` URL
    /// en `image`; un SVG, como marcado en `svg` para que sea interactivo.
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default, skip_deserializing)]
    pub svg: Option<String>,
    #[serde(default)]
    pub buttons: Vec<DefButton>,
    /// Texto de ayuda que se muestra en la página del dispositivo.
    #[serde(default)]
    pub notes: Option<Text>,
    /// Archivo del que salió (para mostrarlo y depurar).
    #[serde(default, skip_deserializing)]
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefButton {
    /// Nombre corto impreso en el periférico o en el manual ("G4").
    pub id: String,
    /// Descripción ("Side back" / "Lateral atrás").
    pub label: Text,
    /// Evento que envía de fábrica (`BTN_SIDE`, `KEY_F13`, `WHEEL_LEFT`…).
    /// Si lo gestiona el firmware (DPI, perfiles), se omite.
    #[serde(default)]
    pub emits: Option<String>,
    /// Índice del botón en ratbagd/libratbag, para leer y reprogramar su
    /// función en la memoria interna.
    #[serde(default)]
    pub hw_index: Option<u32>,
    /// Posición en la imagen, de 0 a 1 (esquina superior izquierda = 0, 0).
    #[serde(default)]
    pub x: Option<f32>,
    #[serde(default)]
    pub y: Option<f32>,
}

/// Texto igual en todos los idiomas, o uno por idioma: `{ "en": …, "es": … }`.
/// La interfaz elige el del idioma activo.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Text {
    Plain(String),
    Translated(std::collections::BTreeMap<String, String>),
}

fn default_kind() -> String {
    "mouse".into()
}

fn search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(base) = directories::BaseDirs::new() {
        dirs.push(base.config_dir().join("gnohub/devices"));
        dirs.push(base.data_local_dir().join("gnohub/devices"));
    }
    let system =
        std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    dirs.extend(std::env::split_paths(&system).map(|d| d.join("gnohub/devices")));
    if cfg!(debug_assertions) {
        dirs.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../devices"));
    }
    // Ejecutando el binario sin instalar, desde `target/<perfil>/` del proyecto.
    if let Some(project) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent()?.parent()?.parent().map(Path::to_path_buf))
    {
        dirs.push(project.join("devices"));
    }
    dirs
}

fn normalize_id(id: &str) -> Option<String> {
    let (v, p) = id.trim().split_once(':')?;
    let v = u16::from_str_radix(v, 16).ok()?;
    let p = u16::from_str_radix(p, 16).ok()?;
    Some(format!("{v:04x}:{p:04x}"))
}

fn image_data_url(json: &Path, image: &str) -> Option<String> {
    let path = json.parent()?.join(image);
    let mime = match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => return None,
    };
    let bytes = std::fs::read(&path).ok()?;
    Some(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

/// Carga una definición; los errores se devuelven con el archivo para que se
/// vean en la interfaz en vez de ignorarse en silencio.
fn load(path: &Path) -> Result<Definition, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut def: Definition = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    def.matches = def
        .matches
        .iter()
        .map(|m| {
            normalize_id(m).ok_or_else(|| rust_i18n::t!("definitions.bad_id", id = m).into_owned())
        })
        .collect::<Result<_, _>>()?;
    if let Some(image) = def.image.take() {
        let missing = || rust_i18n::t!("definitions.bad_image", image = image).into_owned();
        if image.to_ascii_lowercase().ends_with(".svg") {
            let file = path.parent().ok_or_else(missing)?.join(&image);
            def.svg = Some(std::fs::read_to_string(file).map_err(|_| missing())?);
        } else {
            def.image = Some(image_data_url(path, &image).ok_or_else(missing)?);
        }
    }
    def.source = path.display().to_string();
    Ok(def)
}

#[derive(Debug, Clone, Serialize)]
pub struct Loaded {
    pub definitions: Vec<Definition>,
    /// Archivos con errores, para avisar en Ajustes.
    pub errors: Vec<String>,
}

pub fn load_all() -> Loaded {
    let mut definitions: Vec<Definition> = Vec::new();
    let mut errors = Vec::new();
    for dir in search_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut paths: Vec<_> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.extension().is_some_and(|e| e == "json")
                    && p.file_name().is_some_and(|n| n != "schema.json")
            })
            .collect();
        paths.sort();
        for path in paths {
            match load(&path) {
                // Un modelo ya definido en un directorio de más prioridad se ignora.
                Ok(def)
                    if !def
                        .matches
                        .iter()
                        .any(|m| definitions.iter().any(|d| d.matches.contains(m))) =>
                {
                    definitions.push(def)
                }
                Ok(_) => {}
                Err(e) => errors.push(format!("{}: {e}", path.display())),
            }
        }
    }
    Loaded {
        definitions,
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_definitions_are_valid() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../devices");
        let mut count = 0;
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") && !path.ends_with("schema.json") {
                let def = load(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                for b in &def.buttons {
                    if let Some(name) = &b.emits {
                        assert!(
                            name.parse::<gnohub_core::Button>().is_ok(),
                            "{}: {name}",
                            path.display()
                        );
                    }
                    for v in [b.x, b.y].into_iter().flatten() {
                        assert!(
                            (0.0..=1.0).contains(&v),
                            "{}: {} fuera de la imagen",
                            path.display(),
                            b.id
                        );
                    }
                }
                count += 1;
            }
        }
        assert!(count > 0);
    }

    #[test]
    fn normalizes_ids() {
        assert_eq!(normalize_id("046D:C08B").as_deref(), Some("046d:c08b"));
        assert_eq!(normalize_id("46d:c08b").as_deref(), Some("046d:c08b"));
        assert_eq!(normalize_id("nope"), None);
    }
}
