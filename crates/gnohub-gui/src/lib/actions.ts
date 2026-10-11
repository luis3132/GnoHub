// Utilidades sobre acciones de macro y modos de asignación.

import { has, t } from "./i18n/index.svelte";
import { keyLabel } from "./keys";
import type { Action, BindingDef, Mapping, MacroDef, MacroRef, Mode } from "./types";

export type ActionKind = "tap" | "down" | "up" | "chord" | "delay" | "move" | "scroll" | "command";

export const ACTION_KINDS: ActionKind[] = ["tap", "down", "up", "chord", "delay", "move", "scroll", "command"];

export function actionKindLabel(kind: ActionKind): string {
  return t(`actions.${kind}`);
}

export function kindOf(action: Action): ActionKind {
  return Object.keys(action)[0] as ActionKind;
}

function keyOf(action: Action): string | null {
  if ("tap" in action) return action.tap;
  if ("down" in action) return action.down;
  if ("up" in action) return action.up;
  if ("chord" in action) return action.chord[0] ?? null;
  return null;
}

/** Cambia el tipo de una acción conservando la tecla cuando tiene sentido. */
export function convertAction(action: Action, kind: ActionKind): Action {
  const key = keyOf(action) ?? "KEY_A";
  switch (kind) {
    case "tap":
      return { tap: key };
    case "down":
      return { down: key };
    case "up":
      return { up: key };
    case "chord":
      return { chord: "chord" in action ? action.chord : ["KEY_LEFTCTRL", key] };
    case "delay":
      return { delay: 50 };
    case "move":
      return { move: [10, 0] };
    case "scroll":
      return { scroll: 1 };
    case "command":
      return { command: "" };
  }
}

export function describeAction(action: Action): string {
  if ("tap" in action) return keyLabel(action.tap);
  if ("down" in action) return `↓ ${keyLabel(action.down)}`;
  if ("up" in action) return `↑ ${keyLabel(action.up)}`;
  if ("chord" in action) return action.chord.map(keyLabel).join(" + ");
  if ("delay" in action) return `${action.delay} ms`;
  if ("move" in action) return t("actions.moveText", { x: action.move[0], y: action.move[1] });
  if ("scroll" in action) return t("actions.scrollText", { n: `${action.scroll > 0 ? "+" : ""}${action.scroll}` });
  return `$ ${action.command}`;
}

/** Número de eventos que emite una acción (para estimar duración). */
function eventCount(action: Action): number {
  if ("tap" in action) return 2;
  if ("chord" in action) return action.chord.length * 2;
  if ("delay" in action) return 0;
  return 1;
}

/** Duración aproximada en ms, igual que la calcula el daemon al expandir pasos. */
export function estimateMs(actions: Action[], stepDelay: number): number {
  let total = 0;
  let pending = false; // hay un paso emitido esperando el siguiente
  for (const action of actions) {
    if ("delay" in action) {
      total += action.delay;
      pending = false;
      continue;
    }
    for (let i = 0; i < eventCount(action); i++) {
      if (pending) total += stepDelay;
      pending = true;
    }
  }
  return total;
}

export const MODES: Mode[] = ["native", "disabled", "remap", "press", "while_held", "toggle", "sequence", "tap_hold"];

export function modeHint(mode: Mode): string {
  return t(`modes.${mode}.hint`);
}

export function modeLabel(mode: Mode): string {
  return t(`modes.${mode}.label`);
}

/** La macro "principal" de una asignación, para conservarla al cambiar de modo. */
function primaryMacro(b: BindingDef): MacroRef | undefined {
  switch (b.mode) {
    case "press":
    case "while_held":
    case "toggle":
      return b.macro;
    case "sequence":
      return b.while_held ?? b.on_press ?? b.on_release;
    case "tap_hold":
      return b.tap;
    case "native":
    case "disabled":
    case "remap":
      return undefined;
  }
}

export function newBinding(mode: Mode, button: string, from?: BindingDef, fallback?: MacroRef): BindingDef {
  const macro = (from && primaryMacro(from)) ?? fallback ?? emptyMacro();
  switch (mode) {
    case "native":
    case "disabled":
      return { mode, button };
    case "remap":
      return { mode, button, to: from?.mode === "remap" ? from.to : [] };
    case "press":
      return { mode, button, macro, repeat: 1 };
    case "while_held":
    case "toggle":
      return { mode, button, macro };
    case "sequence":
      return { mode, button, while_held: macro };
    case "tap_hold":
      return { mode, button, tap: macro, hold: emptyMacro(), hold_time: 250, hold_repeat: false };
  }
}

export function emptyMacro(): MacroDef {
  return { step_delay: 10, actions: [] };
}

export function describeRef(ref: MacroRef | undefined): string {
  if (ref === undefined) return t("actions.nothing");
  if (typeof ref === "string") return ref;
  const { actions } = ref;
  if (actions.length === 0) return t("actions.noSteps");
  const text = actions.slice(0, 3).map(describeAction).join(", ");
  return actions.length > 3 ? `${text}…` : text;
}

export function describeBinding(b: BindingDef): string {
  switch (b.mode) {
    case "native":
    case "disabled":
      return "";
    case "remap":
      return b.to.length ? b.to.map(keyLabel).join(" + ") : t("actions.noKeys");
    case "press":
      return b.repeat > 1 ? `${describeRef(b.macro)} ×${b.repeat}` : describeRef(b.macro);
    case "while_held":
    case "toggle":
      return describeRef(b.macro);
    case "sequence":
      return [b.on_press, b.while_held, b.on_release].map(describeRef).join(" → ");
    case "tap_hold":
      return `${describeRef(b.tap)} / ${describeRef(b.hold)}`;
  }
}

/** Claves de una asignación que pueden apuntar a una macro con nombre. */
export const REF_FIELDS = ["macro", "tap", "hold", "on_press", "while_held", "on_release"] as const;

/** Descripción de la función de un botón en la memoria del ratón. */
export function describeMapping(m: Mapping): string {
  switch (m.kind) {
    case "none":
      return t("mapping.none");
    case "button":
      return t("mapping.button", { n: m.n });
    case "special": {
      const n = String(m.code - SPECIAL_BASE);
      return has(`mapping.special.${n}`) ? t(`mapping.special.${n}`) : t("mapping.special.other");
    }
    case "key":
      return t("mapping.key", { key: keyName(m.code) });
    case "macro":
      return t("mapping.macro");
    default:
      return t("mapping.unknown");
  }
}

/** Base de los códigos especiales de libratbag (`RATBAG_BUTTON_ACTION_SPECIAL_*`). */
export const SPECIAL_BASE = 2 ** 30;

/** Funciones que se pueden devolver a un botón reprogramado si no hay copia guardada. */
export const RESTORABLE: Mapping[] = [
  ...[11, 9, 10, 7, 8, 12, 13, 14, 1, 2, 3].map((n): Mapping => ({ kind: "special", code: SPECIAL_BASE + n })),
  ...[1, 2, 3, 4, 5, 6, 7, 8].map((n): Mapping => ({ kind: "button", n })),
  { kind: "none" },
];

function keyName(code: number): string {
  // F13–F24 son las teclas que usa GnoHub; el resto se muestra por código.
  return code >= 183 && code <= 194 ? `F${code - 170}` : String(code);
}
