// Backend simulado para abrir la interfaz en un navegador normal (`npm run dev`)
// y revisar el diseño sin Tauri. Nunca se usa dentro de la app.

import type { ConfigFile, DeviceSummary, ServiceStatus } from "./types";

let config: ConfigFile = {
  macros: {
    copiar: { step_delay: 10, actions: [{ chord: ["KEY_LEFTCTRL", "KEY_C"] }] },
    rafaga: { step_delay: 1, actions: [{ tap: "BTN_LEFT" }, { delay: 30 }] },
  },
  devices: [
    {
      name: "G502 HERO",
      vendor: 0x046d,
      product: 0xc08b,
      bindings: [
        { mode: "tap_hold", button: "BTN_SIDE", tap: "copiar", hold: { step_delay: 10, actions: [{ chord: ["KEY_LEFTCTRL", "KEY_V"] }] }, hold_time: 300, hold_repeat: false },
        { mode: "while_held", button: "BTN_EXTRA", macro: "rafaga", repeat_delay: 30 },
      ],
    },
  ],
};

const devices: DeviceSummary[] = [
  {
    name: "Logitech G502 HERO Gaming Mouse",
    vendor: 0x046d,
    product: 0xc08b,
    kind: "mouse",
    nodes: ["/dev/input/event11", "/dev/input/event13"],
    buttons: ["BTN_LEFT", "BTN_RIGHT", "BTN_MIDDLE", "BTN_SIDE", "BTN_EXTRA", "BTN_FORWARD", "BTN_BACK", "BTN_TASK"],
  },
  { name: "Microsoft X-Box One S pad", vendor: 0x045e, product: 0x02ea, kind: "other", nodes: ["/dev/input/event29"], buttons: ["BTN_SOUTH", "BTN_EAST", "BTN_NORTH", "BTN_WEST", "BTN_TL", "BTN_TR"] },
];

let service: ServiceStatus = { installed: true, active: true, enabled: true, embedded: false, daemon_path: "/home/usuario/GnoHub/target/debug/gnohubd" };

const keys = [
  ...Array.from({ length: 26 }, (_, i) => `KEY_${String.fromCharCode(65 + i)}`),
  "KEY_LEFTCTRL", "KEY_LEFTSHIFT", "KEY_LEFTALT", "KEY_LEFTMETA", "KEY_ENTER", "KEY_SPACE", "KEY_ESC",
  "BTN_LEFT", "BTN_RIGHT", "BTN_MIDDLE", "BTN_SIDE", "BTN_EXTRA", "BTN_FORWARD", "BTN_BACK", "BTN_TASK",
];

const DEFINITION = {"name": "Logitech G502 HERO", "kind": "mouse", "matches": ["046d:c08b"], "image": null, "svg": null, "buttons": [{"id": "G1", "label": {"en": "Left click", "es": "Clic izquierdo"}, "emits": "BTN_LEFT", "hw_index": 0}, {"id": "G2", "label": {"en": "Right click", "es": "Clic derecho"}, "emits": "BTN_RIGHT", "hw_index": 1}, {"id": "G3", "label": {"en": "Wheel click", "es": "Clic de rueda"}, "emits": "BTN_MIDDLE", "hw_index": 2}, {"id": "G4", "label": {"en": "Side back", "es": "Lateral atrás"}, "emits": "BTN_SIDE", "hw_index": 3}, {"id": "G5", "label": {"en": "Side forward", "es": "Lateral adelante"}, "emits": "BTN_EXTRA", "hw_index": 4}, {"id": "G6", "label": {"en": "Precision", "es": "Precisión"}, "hw_index": 5}, {"id": "G7", "label": {"en": "DPI down", "es": "Bajar DPI"}, "hw_index": 6}, {"id": "G8", "label": {"en": "DPI up", "es": "Subir DPI"}, "hw_index": 7}, {"id": "G9", "label": {"en": "Change profile", "es": "Cambiar perfil"}, "hw_index": 8}, {"id": "←", "label": {"en": "Wheel left", "es": "Rueda a la izquierda"}, "emits": "WHEEL_LEFT", "hw_index": 10}, {"id": "→", "label": {"en": "Wheel right", "es": "Rueda a la derecha"}, "emits": "WHEEL_RIGHT", "hw_index": 9}], "notes": {"en": "G6 to G9 are handled by the mouse's internal memory (DPI and profiles). Click one and press “Use in GnoHub” to make it send a free key, so you can assign it anything.", "es": "G6 a G9 los gestiona la memoria interna del ratón (DPI y perfiles). Pulsa «Usar en GnoHub» en uno de ellos para que envíe una tecla libre y poder asignarle lo que quieras."}, "source": "devices/logitech-g502-hero.json"};
const HARDWARE = {"name": "Logitech G502 HERO Gaming Mouse", "report_rate": 1000, "report_rates": [125, 250, 500, 1000], "resolutions": [{"index": 0, "dpi": 1200, "active": true, "default": true, "disabled": false}, {"index": 1, "dpi": 2400, "active": false, "default": false, "disabled": false}], "dpi_values": [400, 800, 1200, 1600, 2400, 3200, 6400], "buttons": [{"index": 0, "emits": "BTN_LEFT", "firmware": false, "mapping": {"kind": "button", "n": 1}, "taken_over": false, "has_backup": false}, {"index": 1, "emits": "BTN_RIGHT", "firmware": false, "mapping": {"kind": "button", "n": 2}, "taken_over": false, "has_backup": false}, {"index": 2, "emits": "BTN_MIDDLE", "firmware": false, "mapping": {"kind": "button", "n": 3}, "taken_over": false, "has_backup": false}, {"index": 3, "emits": "BTN_SIDE", "firmware": false, "mapping": {"kind": "button", "n": 4}, "taken_over": false, "has_backup": false}, {"index": 4, "emits": "BTN_EXTRA", "firmware": false, "mapping": {"kind": "button", "n": 5}, "taken_over": false, "has_backup": false}, {"index": 5, "emits": null, "firmware": true, "mapping": {"kind": "special", "code": 1073741835}, "taken_over": false, "has_backup": false}, {"index": 6, "emits": null, "firmware": true, "mapping": {"kind": "special", "code": 1073741834}, "taken_over": false, "has_backup": false}, {"index": 7, "emits": "KEY_F13", "firmware": false, "mapping": {"kind": "key", "code": 183}, "taken_over": true, "has_backup": false}, {"index": 8, "emits": null, "firmware": true, "mapping": {"kind": "special", "code": 1073741837}, "taken_over": false, "has_backup": false}, {"index": 9, "emits": "WHEEL_RIGHT", "firmware": false, "mapping": {"kind": "special", "code": 1073741827}, "taken_over": false, "has_backup": false}, {"index": 10, "emits": "WHEEL_LEFT", "firmware": false, "mapping": {"kind": "special", "code": 1073741826}, "taken_over": false, "has_backup": false}], "leds": [{"index": 0, "mode": 1, "modes": [0, 1, 2, 3], "color": [128, 0, 255], "color_depth": 1, "duration": 0, "brightness": 255}, {"index": 1, "mode": 3, "modes": [0, 1, 2, 3], "color": [240, 166, 58], "color_depth": 1, "duration": 4000, "brightness": 200}], "profile": 4, "profiles": 5};

// Dibujos de Piper para la vista previa: se copian a mano a `dev-assets/`
// (ignorada por git); sin ellos, la vista previa no muestra dibujo.
const DEV_SVGS: Record<string, () => Promise<unknown>> = import.meta.env.DEV
  ? import.meta.glob("/dev-assets/*.svg", { query: "?raw", import: "default" })
  : {};

// Datos reales volcados del backend (ver `dump_real_backend` en main.rs);
// si existen, la vista previa los usa en vez de los inventados.
const REAL = import.meta.env.DEV ? import.meta.glob("/dev-assets/real.json", { import: "default" }) : {};

async function realData(): Promise<Record<string, any> | null> {
  const load = REAL["/dev-assets/real.json"];
  return load ? ((await load()) as Record<string, any>) : null;
}

export async function mockInvoke(cmd: string, args: Record<string, unknown> = {}): Promise<unknown> {
  await new Promise((r) => setTimeout(r, 60));
  const real = await realData();
  if (real) {
    const key = `${(args.vendor as number)?.toString(16).padStart(4, "0")}:${(args.product as number)?.toString(16).padStart(4, "0")}`;
    if (cmd === "hardware") return real.hardware[key] ?? null;
    if (cmd === "piper_svg") return real.piper_svg[key] ?? null;
    if (cmd in real) return structuredClone(real[cmd]);
  }
  switch (cmd) {
    case "load_config":
      return { path: "~/.config/gnohub/config.toml", exists: true, file: structuredClone(config) };
    case "validate_config":
      return null;
    case "save_config":
      config = args.file as ConfigFile;
      return null;
    case "list_devices":
      return devices;
    case "locked_devices":
      return [
        { name: "AT Translated Set 2 keyboard", vendor: 1, product: 1, kind: "keyboard", node_names: ["AT Translated Set 2 keyboard"] },
        { name: "Razer DeathAdder V2", vendor: 0x1532, product: 0x0084, kind: "mouse", node_names: ["Razer DeathAdder V2"] },
      ];
    case "grant_access":
      return null;
    case "installed_games":
      return [
        { id: "steam:730", name: "Counter-Strike 2", source: "steam" },
        { id: "steam:526870", name: "Satisfactory", source: "steam" },
        { id: "exe:speed.exe", name: "Need For Speed Most Wanted", source: "lutris" },
      ];
    case "installed_apps":
      return [
        { id: "org.mozilla.firefox", name: "Firefox", wm_class: "firefox" },
        { id: "steam", name: "Steam", wm_class: "steam" },
        { id: "org.gnome.Nautilus", name: "Archivos", wm_class: null },
      ];
    case "focus_status":
    case "enable_gnome_extension":
      return { desktop: "gnome", extension_installed: true, extension_active: cmd === "enable_gnome_extension" };
    case "device_definitions":
      return { definitions: [DEFINITION], errors: [] };
    case "hardware":
      return args.vendor === 0x046d ? structuredClone(HARDWARE) : null;
    case "piper_svg": {
      const load = DEV_SVGS["/dev-assets/piper-046d-c08b.svg"];
      return args.vendor === 0x046d && load ? await load() : null;
    }
    case "set_dpi":
    case "set_report_rate":
    case "set_led":
      return null;
    case "take_over_button":
      return "KEY_F13";
    case "restore_button":
    case "set_locale":
      return null;
    case "system_locale":
      return navigator.language.startsWith("es") ? "es" : "en";
    case "log_frontend_error":
      console.error("[GnoHub]", args.message);
      return null;
    case "key_names":
      return keys;
    case "system_status":
      return { unreadable: 27, uinput_writable: true, service };
    case "service_action":
      service = { ...service, active: args.action !== "stop" };
      return service;
    case "detect_button":
      await new Promise((r) => setTimeout(r, 1200));
      return "BTN_FORWARD";
    default:
      throw new Error(`comando desconocido: ${cmd}`);
  }
}
