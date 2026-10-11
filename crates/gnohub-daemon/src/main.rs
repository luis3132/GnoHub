mod devices;
mod engine;
mod executor;
mod focus;
mod output;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use gnohub_core::Config;
use tracing_subscriber::EnvFilter;

/// Daemon de GnoHub: remapeo de botones y macros para cualquier ratón o teclado.
#[derive(Parser)]
#[command(name = "gnohubd", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Lista los dispositivos de entrada detectados.
    List,
    /// Muestra en vivo los botones que pulsas, sin capturar el dispositivo,
    /// para saber qué nombre poner en la configuración.
    Watch {
        /// VID:PID (p. ej. 046d:c08b) o ruta /dev/input/eventN.
        target: String,
    },
    /// Valida la configuración sin tocar ningún dispositivo.
    Check {
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
    /// Captura los dispositivos configurados y aplica las asignaciones. Recarga
    /// sola cuando cambia el archivo y captura los dispositivos que se conecten.
    Run {
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
}

fn default_config_path() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "gnohub")
        .context("no se encontró el directorio HOME")?;
    Ok(dirs.config_dir().join("config.toml"))
}

fn load_config(path: Option<PathBuf>) -> Result<(PathBuf, Config)> {
    let path = match path {
        Some(p) => p,
        None => default_config_path()?,
    };
    let config = Config::load(&path)?;
    Ok((path, config))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        // Sin colores cuando la salida va al journal de systemd.
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stderr()))
        .init();

    match Cli::parse().command {
        Command::List => devices::list(),
        Command::Watch { target } => devices::watch(&target).await?,
        Command::Check { config } => {
            let (path, config) = load_config(config)?;
            let bindings: usize = config.devices.iter().map(|d| d.bindings.len()).sum();
            println!(
                "{}: OK ({} macros, {} dispositivos, {bindings} asignaciones)",
                path.display(),
                config.macros.len(),
                config.devices.len(),
            );
        }
        Command::Run { config } => {
            let path = match config {
                Some(p) => p,
                None => default_config_path()?,
            };
            tracing::info!("configuración: {}", path.display());
            engine::run(path).await?;
        }
    }
    Ok(())
}
