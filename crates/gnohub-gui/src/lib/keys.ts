// Nombres legibles de teclas y botones, y traducción de eventos del navegador
// a los nombres de evdev que usa la configuración.

import { has, t } from "./i18n/index.svelte";
import type { KeyName } from "./types";

// Nombres que no cambian con el idioma; el resto está en los diccionarios (`keys.*`).
const SYMBOLS: Record<string, string> = {
  KEY_LEFTCTRL: "Ctrl",
  KEY_LEFTALT: "Alt",
  KEY_LEFTMETA: "Super",
  KEY_ESC: "Esc",
  KEY_TAB: "Tab",
  KEY_UP: "↑",
  KEY_DOWN: "↓",
  KEY_LEFT: "←",
  KEY_RIGHT: "→",
  KEY_INSERT: "Insert",
  KEY_MINUS: "-",
  KEY_EQUAL: "=",
  KEY_LEFTBRACE: "[",
  KEY_RIGHTBRACE: "]",
  KEY_SEMICOLON: ";",
  KEY_APOSTROPHE: "'",
  KEY_GRAVE: "`",
  KEY_BACKSLASH: "\\",
  KEY_COMMA: ",",
  KEY_DOT: ".",
  KEY_SLASH: "/",
  KEY_102ND: "< >",
};

export function deviceKey(vendor: number, product: number): string {
  const hex = (n: number) => n.toString(16).padStart(4, "0");
  return `${hex(vendor)}:${hex(product)}`;
}

export function keyLabel(name: KeyName): string {
  const up = name.toUpperCase();
  if (SYMBOLS[up]) return SYMBOLS[up];
  if (has(`keys.${up}`)) return t(`keys.${up}`);
  const bare = up.replace(/^(KEY|BTN)_/, "");
  if (/^KP/.test(bare)) return t("keys.numpad", { key: bare.slice(2).toLowerCase() });
  if (bare.length <= 3) return bare;
  return bare.charAt(0) + bare.slice(1).toLowerCase();
}

const CODES: Record<string, string> = {
  Enter: "KEY_ENTER",
  Escape: "KEY_ESC",
  Backspace: "KEY_BACKSPACE",
  Tab: "KEY_TAB",
  Space: "KEY_SPACE",
  Minus: "KEY_MINUS",
  Equal: "KEY_EQUAL",
  BracketLeft: "KEY_LEFTBRACE",
  BracketRight: "KEY_RIGHTBRACE",
  Backslash: "KEY_BACKSLASH",
  Semicolon: "KEY_SEMICOLON",
  Quote: "KEY_APOSTROPHE",
  Backquote: "KEY_GRAVE",
  Comma: "KEY_COMMA",
  Period: "KEY_DOT",
  Slash: "KEY_SLASH",
  IntlBackslash: "KEY_102ND",
  CapsLock: "KEY_CAPSLOCK",
  ShiftLeft: "KEY_LEFTSHIFT",
  ShiftRight: "KEY_RIGHTSHIFT",
  ControlLeft: "KEY_LEFTCTRL",
  ControlRight: "KEY_RIGHTCTRL",
  AltLeft: "KEY_LEFTALT",
  AltRight: "KEY_RIGHTALT",
  MetaLeft: "KEY_LEFTMETA",
  MetaRight: "KEY_RIGHTMETA",
  ContextMenu: "KEY_COMPOSE",
  ArrowUp: "KEY_UP",
  ArrowDown: "KEY_DOWN",
  ArrowLeft: "KEY_LEFT",
  ArrowRight: "KEY_RIGHT",
  Insert: "KEY_INSERT",
  Delete: "KEY_DELETE",
  Home: "KEY_HOME",
  End: "KEY_END",
  PageUp: "KEY_PAGEUP",
  PageDown: "KEY_PAGEDOWN",
  PrintScreen: "KEY_SYSRQ",
  ScrollLock: "KEY_SCROLLLOCK",
  Pause: "KEY_PAUSE",
  NumLock: "KEY_NUMLOCK",
  NumpadAdd: "KEY_KPPLUS",
  NumpadSubtract: "KEY_KPMINUS",
  NumpadMultiply: "KEY_KPASTERISK",
  NumpadDivide: "KEY_KPSLASH",
  NumpadDecimal: "KEY_KPDOT",
  NumpadEnter: "KEY_KPENTER",
  AudioVolumeUp: "KEY_VOLUMEUP",
  AudioVolumeDown: "KEY_VOLUMEDOWN",
  AudioVolumeMute: "KEY_MUTE",
  MediaPlayPause: "KEY_PLAYPAUSE",
  MediaTrackNext: "KEY_NEXTSONG",
  MediaTrackPrevious: "KEY_PREVIOUSSONG",
  MediaStop: "KEY_STOPCD",
};

/** `KeyboardEvent.code` → nombre evdev. Usa la posición física, no el carácter. */
export function codeToKey(code: string): KeyName | null {
  if (CODES[code]) return CODES[code];
  let m = /^Key([A-Z])$/.exec(code);
  if (m) return `KEY_${m[1]}`;
  m = /^Digit(\d)$/.exec(code);
  if (m) return `KEY_${m[1]}`;
  m = /^F(\d{1,2})$/.exec(code);
  if (m) return `KEY_F${m[1]}`;
  m = /^Numpad(\d)$/.exec(code);
  if (m) return `KEY_KP${m[1]}`;
  return null;
}

const MOUSE_BUTTONS = ["BTN_LEFT", "BTN_MIDDLE", "BTN_RIGHT", "BTN_SIDE", "BTN_EXTRA"];

/** `MouseEvent.button` → nombre evdev. */
export function mouseToKey(button: number): KeyName | null {
  return MOUSE_BUTTONS[button] ?? null;
}

export function isWheel(name: KeyName): boolean {
  return name.toUpperCase().startsWith("WHEEL_");
}

export function sameKey(a: KeyName, b: KeyName): boolean {
  return a.toUpperCase() === b.toUpperCase();
}
