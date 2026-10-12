// Espejo de los tipos de `gnohub-core` tal como viajan en JSON.

export type KeyName = string;

export type Action =
  | { tap: KeyName }
  | { down: KeyName }
  | { up: KeyName }
  | { chord: KeyName[] }
  | { delay: number }
  | { move: [number, number] }
  | { scroll: number }
  | { command: string };

export interface MacroDef {
  /** Milisegundos entre cada paso (mínimo 1). */
  step_delay: number;
  actions: Action[];
}

/** Nombre de una macro guardada o macro propia del botón. */
export type MacroRef = string | MacroDef;

export type Mode = "native" | "disabled" | "remap" | "press" | "while_held" | "toggle" | "sequence" | "tap_hold";

export type BindingDef =
  | { mode: "native"; button: KeyName }
  | { mode: "disabled"; button: KeyName }
  | { mode: "remap"; button: KeyName; to: KeyName[] }
  | { mode: "press"; button: KeyName; macro: MacroRef; repeat: number; repeat_delay?: number }
  | { mode: "while_held"; button: KeyName; macro: MacroRef; repeat_delay?: number }
  | { mode: "toggle"; button: KeyName; macro: MacroRef; repeat_delay?: number }
  | {
      mode: "sequence";
      button: KeyName;
      on_press?: MacroRef;
      while_held?: MacroRef;
      on_release?: MacroRef;
      repeat_delay?: number;
    }
  | {
      mode: "tap_hold";
      button: KeyName;
      tap: MacroRef;
      hold: MacroRef;
      hold_time: number;
      hold_repeat: boolean;
      repeat_delay?: number;
    };

export interface AppProfileDef {
  name: string;
  /** Identificadores de aplicación o clases de ventana. */
  apps: string[];
  bindings: BindingDef[];
}

export interface DeviceDef {
  name?: string;
  vendor: number;
  product: number;
  bindings: BindingDef[];
  apps?: AppProfileDef[];
}

export interface InstalledApp {
  id: string;
  name: string;
  wm_class: string | null;
}

/** Juego instalado (Steam, Lutris, Heroic) tal como se pone en un perfil. */
export interface Game {
  /** `steam:730` o `exe:speed.exe`. */
  id: string;
  name: string;
  source: "steam" | "lutris" | "heroic";
}

export interface FocusStatus {
  desktop: "gnome" | "hyprland" | "sway" | "i3" | "x11" | "otro";
  extension_installed: boolean;
  extension_active: boolean;
}

export interface ConfigFile {
  macros: Record<string, MacroDef>;
  devices: DeviceDef[];
}

export type DeviceKind = "mouse" | "keyboard" | "other";

export interface DeviceSummary {
  name: string;
  vendor: number;
  product: number;
  kind: DeviceKind;
  nodes: string[];
  buttons: KeyName[];
}

/** Texto de una definición: igual en todos los idiomas, o `{ en, es, … }`. */
export type Localized = string | Record<string, string>;

export interface DefButton {
  id: string;
  label: Localized;
  emits?: string | null;
  hw_index?: number | null;
  x?: number | null;
  y?: number | null;
}

export interface Definition {
  name: string;
  kind: string;
  matches: string[];
  /** `data:` URL de una imagen PNG/JPG/WebP. */
  image?: string | null;
  /** Marcado de una imagen SVG (interactiva si sigue la convención de Piper). */
  svg?: string | null;
  buttons: DefButton[];
  notes?: Localized | null;
  source: string;
}

export interface Resolution {
  index: number;
  dpi: number;
  active: boolean;
  default: boolean;
  disabled: boolean;
}

/** Función de un botón en la memoria del ratón (libratbag). */
export type Mapping =
  | { kind: "none" }
  | { kind: "button"; n: number }
  | { kind: "special"; code: number }
  | { kind: "key"; code: number }
  | { kind: "macro"; events: [number, number][] }
  | { kind: "unknown"; raw_kind: number };

export interface HwButton {
  index: number;
  emits: string | null;
  mapping: Mapping;
  firmware: boolean;
  /** GnoHub lo reprogramó con una tecla libre (F13–F24). */
  taken_over: boolean;
  /** Hay copia de su función original para restaurarla. */
  has_backup: boolean;
}

export interface Led {
  index: number;
  /** 0 apagado, 1 fijo, 2 ciclo de colores, 3 respiración. */
  mode: number;
  modes: number[];
  color: [number, number, number];
  color_depth: number;
  duration: number;
  brightness: number;
}

export interface Hardware {
  name: string;
  report_rate: number;
  report_rates: number[];
  resolutions: Resolution[];
  dpi_values: number[];
  buttons: HwButton[];
  leds: Led[];
  profile: number;
  profiles: number;
}

export interface LockedDevice {
  name: string;
  vendor: number;
  product: number;
  kind: DeviceKind;
  node_names: string[];
}

export interface ServiceStatus {
  installed: boolean;
  active: boolean;
  enabled: boolean;
  /** Lo ejecuta la propia app porque el servicio no está instalado. */
  embedded: boolean;
  daemon_path: string | null;
}

export interface SystemStatus {
  unreadable: number;
  uinput_writable: boolean;
  service: ServiceStatus;
}

export interface LoadedConfig {
  path: string;
  exists: boolean;
  file: ConfigFile;
}

export type ServiceAction = "install" | "start" | "stop" | "restart";
