# GnoHub

Versión 0.0.1.

Alternativa a Logitech G Hub para Linux, escrita en Rust: macros, remapeo de
botones y modos de disparo para **cualquier** ratón o teclado, no solo Logitech.

Funciona a nivel de kernel (evdev + uinput), así que es independiente del
entorno: GNOME, KDE, Hyprland, Sway, i3, X11 o Wayland, e incluso una TTY.

## Estado

| Hecho | Pendiente |
|---|---|
| Captura de cualquier dispositivo evdev y reenvío del resto de eventos | Perfiles por aplicación en KDE Wayland |
| Modos `remap`, `press`, `while_held`, `toggle`, `sequence`, `tap_hold` y `native` | Macros guardadas en la memoria del ratón |
| Retardo entre pasos por macro (mínimo 1 ms), `delay` explícito, repeticiones | |
| Rueda e inclinación de rueda como botones (`WHEEL_*`) | |
| Perfiles por aplicación: GNOME (extensión), Hyprland, Sway, i3, X11 | |
| Hardware vía ratbagd: DPI, frecuencia, LED y reprogramar botones del firmware | |
| Definiciones de periféricos en JSON (`devices/`) y dibujos interactivos de Piper | |
| Recarga en caliente, reconexión de dispositivos, permisos desde la interfaz | |

## Instalación

GnoHub se instala como paquete; el paquete incluye la app, el servicio
`gnohubd` (activado para todos los usuarios), la extensión de GNOME, las
definiciones de periféricos y la regla de permisos de `uinput`.

```sh
cd crates/gnohub-gui
bun install
bun tauri build          # compila todo y genera .rpm y .deb en target/release/bundle/
sudo dnf install ../../target/release/bundle/rpm/GnoHub-*.rpm      # Fedora
sudo apt install ../../target/release/bundle/deb/GnoHub_*.deb      # Debian/Ubuntu
```

Después, abre GnoHub y pulsa **Dar acceso** en tu ratón o teclado. En GNOME,
cierra sesión y vuelve a entrar una vez para que se cargue la extensión.

Para desinstalar: `sudo dnf remove gno-hub` (también borra los permisos concedidos).

## Desarrollo

```sh
cd crates/gnohub-gui
bun tauri dev            # app con recarga en caliente
bun run dev              # solo la interfaz en el navegador, con datos de prueba
cargo test --workspace   # desde la raíz
```

## Uso

```sh
cargo build --release
./target/release/gnohubd list                 # dispositivos detectados
./target/release/gnohubd watch 046d:c08b      # ver qué nombre envía cada botón
cp examples/g502.toml ~/.config/gnohub/config.toml
./target/release/gnohubd check                # validar la configuración
./target/release/gnohubd run                  # aplicar (Ctrl+C para salir)
```

Al salir, o si el proceso muere, el kernel libera los dispositivos y el ratón
vuelve a su comportamiento normal. `RUST_LOG=debug` muestra más detalle.

## Permisos

GnoHub necesita leer `/dev/input/eventN` y escribir en `/dev/uinput`. El
paquete da acceso a `uinput`; para cada ratón o teclado, **Dar acceso** en la
app crea una regla de udev (pide la contraseña) que da acceso al usuario con
sesión activa solo a ese dispositivo. Evita añadir tu usuario al grupo `input`:
cualquier programa tuyo podría leer todo lo que tecleas.

## Configuración

Mira [`examples/g502.toml`](examples/g502.toml); está comentado con todas las
acciones y modos. Resumen:

```toml
[macros.copiar]
step_delay = 10            # ms entre pasos de esta macro (mínimo 1)
actions = [{ chord = ["KEY_LEFTCTRL", "KEY_C"] }]

[[devices]]
vendor = 0x046d
product = 0xc08b

[[devices.bindings]]
button = "BTN_SIDE"
mode = "tap_hold"          # corto → copiar, mantener → pegar
tap = "copiar"
hold = { step_delay = 10, actions = [{ chord = ["KEY_LEFTCTRL", "KEY_V"] }] }
```

## Estructura

```
crates/gnohub-core     modelo de macros, formato de configuración y dispositivos
crates/gnohub-daemon   binario `gnohubd`: evdev, uinput y ejecución de macros
crates/gnohub-gui      interfaz Tauri (Svelte en src/, Rust en src-tauri/)
devices/               definiciones de periféricos (JSON) — ver devices/README.md
examples/              configuraciones de ejemplo
packaging/             archivos del paquete: servicio, regla udev, extensión de GNOME, scripts
```
