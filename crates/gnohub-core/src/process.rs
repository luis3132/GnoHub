//! Qué programa hay detrás de una ventana, a partir de su PID: para los perfiles
//! que se activan con un ejecutable (`exe:juego.exe`) o un juego de Steam
//! (`steam:730`).

/// Datos del proceso de la ventana enfocada.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProcessInfo {
    /// Nombres de ejecutable en minúsculas: el del binario y los que aparecen
    /// en sus argumentos (en Wine/Proton el `.exe` del juego va como argumento).
    pub executables: Vec<String>,
    /// ID de Steam del juego (`SteamAppId`), si lo lanzó Steam.
    pub steam_app: Option<String>,
}

/// Lee `/proc/<pid>`. Solo funciona con procesos del mismo usuario, que es el
/// caso de las ventanas de su sesión.
pub fn info(pid: u32) -> ProcessInfo {
    let read = |file: &str| std::fs::read(format!("/proc/{pid}/{file}")).unwrap_or_default();
    let exe = std::fs::read_link(format!("/proc/{pid}/exe"))
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    ProcessInfo {
        executables: executables(&exe, &split_nul(&read("cmdline"))),
        steam_app: steam_app(&split_nul(&read("environ"))),
    }
}

fn split_nul(bytes: &[u8]) -> Vec<String> {
    bytes
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect()
}

/// Último componente de una ruta, con `/` o `\` (rutas de Windows en Wine).
pub fn basename(path: &str) -> String {
    path.rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
        .trim()
        .to_lowercase()
}

fn executables(exe: &str, cmdline: &[String]) -> Vec<String> {
    let mut names = Vec::new();
    let mut add = |name: String| {
        if !name.is_empty() && !names.contains(&name) {
            names.push(name);
        }
    };
    if !exe.is_empty() {
        add(basename(exe));
    }
    for (i, arg) in cmdline.iter().enumerate() {
        // El programa (argumento 0) y cualquier `.exe` de los argumentos.
        if i == 0 || arg.to_lowercase().ends_with(".exe") {
            add(basename(arg));
        }
    }
    names
}

fn steam_app(environ: &[String]) -> Option<String> {
    ["SteamAppId=", "SteamGameId="].iter().find_map(|key| {
        environ
            .iter()
            .find_map(|var| var.strip_prefix(key))
            .filter(|id| !id.is_empty() && *id != "0")
            .map(str::to_owned)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_windows_executables_in_wine_arguments() {
        let cmdline = vec![
            "Z:\\mnt\\1TBNso\\programas\\NFS\\speed.exe".to_owned(),
            "-windowed".to_owned(),
        ];
        let names = executables(
            "/home/u/.local/share/Steam/proton/files/bin/wine64-preloader",
            &cmdline,
        );
        assert_eq!(names, vec!["wine64-preloader", "speed.exe"]);
    }

    #[test]
    fn native_executable() {
        assert_eq!(
            executables("/opt/game/Game.x86_64", &["./Game.x86_64".into()]),
            vec!["game.x86_64"]
        );
    }

    #[test]
    fn reads_steam_app_id() {
        let env = vec!["HOME=/home/u".to_owned(), "SteamAppId=730".to_owned()];
        assert_eq!(steam_app(&env).as_deref(), Some("730"));
        assert_eq!(steam_app(&["SteamAppId=0".to_owned()]), None);
        assert_eq!(
            steam_app(&["SteamGameId=2483190".to_owned()]).as_deref(),
            Some("2483190")
        );
    }

    #[test]
    fn reads_own_process() {
        let me = info(std::process::id());
        assert!(!me.executables.is_empty());
    }
}
