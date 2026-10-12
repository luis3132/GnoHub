//! Juegos instalados (Steam, Lutris y Heroic), para elegirlos en un perfil de
//! aplicación. Cada juego se identifica como lo entiende el daemon:
//! `steam:<id>` o `exe:<ejecutable>`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Game {
    /// `steam:730` o `exe:speed.exe`.
    pub id: String,
    pub name: String,
    /// "steam", "lutris" o "heroic".
    pub source: &'static str,
}

pub fn installed() -> Vec<Game> {
    let home = directories::BaseDirs::new()
        .map(|d| d.home_dir().to_path_buf())
        .unwrap_or_default();
    let mut games = steam(&home);
    games.extend(lutris(&home));
    games.extend(heroic(&home));
    let mut seen = BTreeSet::new();
    games.retain(|g| seen.insert(g.id.clone()));
    games.sort_by_key(|g| g.name.to_lowercase());
    games
}

// ------------------------------------------------------------------ Steam ----

/// Valores de una clave en un archivo VDF/ACF de Steam (`"clave"  "valor"`).
fn vdf_values<'a>(text: &'a str, key: &str) -> Vec<&'a str> {
    let quoted: Vec<&str> = text.split('"').skip(1).step_by(2).collect();
    quoted
        .windows(2)
        .filter(|w| w[0].eq_ignore_ascii_case(key))
        .map(|w| w[1])
        .collect()
}

/// Herramientas que Steam instala como "juegos" y no lo son.
fn is_steam_tool(name: &str) -> bool {
    name.starts_with("Proton")
        || name.starts_with("Steam Linux Runtime")
        || name.starts_with("Steamworks Common")
        || name.contains("EasyAntiCheat Runtime")
        || name.contains("BattlEye Runtime")
}

fn steam(home: &Path) -> Vec<Game> {
    let roots = [
        home.join(".local/share/Steam"),
        home.join(".steam/steam"),
        home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
    ];
    let mut libraries: BTreeSet<PathBuf> = BTreeSet::new();
    for root in roots {
        let Ok(text) = std::fs::read_to_string(root.join("steamapps/libraryfolders.vdf")) else {
            continue;
        };
        for path in vdf_values(&text, "path") {
            let lib = PathBuf::from(path.replace("\\\\", "\\"));
            libraries.insert(lib.canonicalize().unwrap_or(lib));
        }
    }
    let mut games = Vec::new();
    for lib in libraries {
        let Ok(entries) = std::fs::read_dir(lib.join("steamapps")) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let is_manifest = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("appmanifest_") && n.ends_with(".acf"));
            if !is_manifest {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let (Some(id), Some(name)) = (
                vdf_values(&text, "appid").first().copied(),
                vdf_values(&text, "name").first().copied(),
            ) else {
                continue;
            };
            if !is_steam_tool(name) {
                games.push(Game {
                    id: format!("steam:{id}"),
                    name: name.to_owned(),
                    source: "steam",
                });
            }
        }
    }
    games
}

// ----------------------------------------------------------------- Lutris ----

/// `exe:` de la sección `game:` de la configuración YAML de un juego de Lutris.
fn lutris_exe(yaml: &str) -> Option<String> {
    let mut in_game = false;
    for line in yaml.lines() {
        if !line.starts_with(' ') && !line.is_empty() {
            in_game = line.trim_end() == "game:";
            continue;
        }
        if in_game && let Some(exe) = line.trim().strip_prefix("exe:") {
            let exe = exe.trim().trim_matches(['"', '\'']);
            return (!exe.is_empty()).then(|| exe.to_owned());
        }
    }
    None
}

/// "need-for-speed-most-wanted-1791691555" → "Need For Speed Most Wanted".
fn lutris_name(slug: &str) -> String {
    let base = slug
        .rsplit_once('-')
        .filter(|(_, n)| n.chars().all(|c| c.is_ascii_digit()))
        .map_or(slug, |(b, _)| b);
    base.split('-')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            chars
                .next()
                .map(|c| c.to_uppercase().chain(chars).collect::<String>())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn lutris(home: &Path) -> Vec<Game> {
    let dirs = [
        home.join(".local/share/lutris/games"),
        home.join(".config/lutris/games"),
        home.join(".var/app/net.lutris.Lutris/data/lutris/games"),
    ];
    let mut games = Vec::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "yml") {
                continue;
            }
            let Some(exe) = std::fs::read_to_string(&path)
                .ok()
                .as_deref()
                .and_then(lutris_exe)
            else {
                continue;
            };
            let slug = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            games.push(Game {
                id: format!("exe:{}", gnohub_core::process::basename(&exe)),
                name: lutris_name(slug),
                source: "lutris",
            });
        }
    }
    games
}

// ----------------------------------------------------------------- Heroic ----

fn heroic(home: &Path) -> Vec<Game> {
    let roots = [
        home.join(".config/heroic"),
        home.join(".var/app/com.heroicgameslauncher.hgl/config/heroic"),
    ];
    let read_json = |path: PathBuf| -> Option<serde_json::Value> {
        serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
    };
    let mut games = Vec::new();
    let mut push = |title: Option<&str>, exe: Option<&str>| {
        if let (Some(title), Some(exe)) = (title, exe.filter(|e| !e.is_empty())) {
            games.push(Game {
                id: format!("exe:{}", gnohub_core::process::basename(exe)),
                name: title.to_owned(),
                source: "heroic",
            });
        }
    };
    for root in roots {
        // Epic (legendary): { app_name: { title, executable, … } }
        if let Some(serde_json::Value::Object(apps)) =
            read_json(root.join("legendaryConfig/legendary/installed.json"))
        {
            for app in apps.values() {
                push(app["title"].as_str(), app["executable"].as_str());
            }
        }
        // Juegos añadidos a mano: { games: [ { title, install: { executable } } ] }
        if let Some(lib) = read_json(root.join("sideload_apps/library.json")) {
            for app in lib["games"].as_array().into_iter().flatten() {
                let exe = app["install"]["executable"].as_str();
                // Heroic deja "Title" como título si no se cambió; mejor el nombre del ejecutable.
                let title = app["title"].as_str().filter(|t| *t != "Title");
                let fallback = exe.map(gnohub_core::process::basename);
                push(title.or(fallback.as_deref()), exe);
            }
        }
    }
    games
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_steam_vdf() {
        let vdf = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"/home/u/.local/share/Steam\"\n\t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"/mnt/SteamLibrary\"\n\t}\n}";
        assert_eq!(
            vdf_values(vdf, "path"),
            vec!["/home/u/.local/share/Steam", "/mnt/SteamLibrary"]
        );
        let acf = "\"AppState\"\n{\n\t\"appid\"\t\t\"730\"\n\t\"name\"\t\t\"Counter-Strike 2\"\n}";
        assert_eq!(vdf_values(acf, "appid"), vec!["730"]);
        assert_eq!(vdf_values(acf, "name"), vec!["Counter-Strike 2"]);
        assert!(is_steam_tool("Proton Experimental"));
        assert!(is_steam_tool("Steam Linux Runtime 3.0 (sniper)"));
        assert!(!is_steam_tool("Counter-Strike 2"));
    }

    #[test]
    fn reads_lutris_games() {
        let yaml = "game:\n  exe: /mnt/juegos/NFS/speed.exe\n  working_dir: /mnt/juegos/NFS\nwine:\n  version: Proton 11.0\n";
        assert_eq!(
            lutris_exe(yaml).as_deref(),
            Some("/mnt/juegos/NFS/speed.exe")
        );
        assert_eq!(lutris_exe("system:\n  exe: nope\n"), None);
        assert_eq!(
            lutris_name("need-for-speed-most-wanted-1791691555"),
            "Need For Speed Most Wanted"
        );
    }
}
