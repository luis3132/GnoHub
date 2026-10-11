// Lista de botones de un dispositivo, combinando lo que se sabe de él:
// su definición JSON (nombres e imagen), el hardware según ratbagd (qué hace
// cada botón ahora) y lo que el propio dispositivo declara.

import { describeMapping } from "./actions";
import { localized, t } from "./i18n/index.svelte";
import { keyLabel, sameKey } from "./keys";
import type { DeviceEntry } from "./state.svelte";
import type { BindingDef, Definition, Hardware } from "./types";

export interface ButtonRow {
  /** Clave estable: el evento que envía, o `hw:N` si lo gestiona el firmware. */
  key: string;
  /** Nombre corto (G4) o, si no hay, el nombre legible del evento. */
  tag: string;
  label: string;
  /** Evento que GnoHub recibe de este botón; `null` si lo gestiona el firmware. */
  emits: string | null;
  /** Función del firmware cuando no envía nada asignable. */
  firmwareAction: string | null;
  /** Lo reprogramó GnoHub (envía F13–F24) y se le puede devolver su función. */
  takenOver: boolean;
  hasBackup: boolean;
  /** Otro botón envía lo mismo: GnoHub no puede distinguirlos. */
  duplicate?: boolean;
  hwIndex: number | null;
  x: number | null;
  y: number | null;
}

export function buildRows(
  entry: DeviceEntry,
  definition: Definition | undefined,
  hardware: Hardware | null | undefined,
  bindings: BindingDef[],
): ButtonRow[] {
  const hwButton = (index: number | null | undefined) =>
    index == null ? undefined : hardware?.buttons.find((b) => b.index === index);

  let rows: ButtonRow[];
  if (definition) {
    rows = definition.buttons.map((b) => {
      const hw = hwButton(b.hw_index);
      // El hardware manda: refleja si el botón se reprogramó.
      const emits = hw ? hw.emits : (b.emits ?? null);
      return {
        key: emits ?? `hw:${b.hw_index ?? b.id}`,
        tag: b.id,
        label: localized(b.label),
        emits,
        firmwareAction: hw?.firmware ? describeMapping(hw.mapping) : emits ? null : t("mapping.handledByFirmware"),
        takenOver: hw?.taken_over ?? false,
        hasBackup: hw?.has_backup ?? false,
        hwIndex: b.hw_index ?? null,
        x: b.x ?? null,
        y: b.y ?? null,
      };
    });
  } else if (hardware && hardware.buttons.length > 0) {
    rows = hardware.buttons.map((b) => ({
      key: b.emits ?? `hw:${b.index}`,
      tag: b.emits ? keyLabel(b.emits) : t("mapping.button", { n: b.index + 1 }),
      label: b.emits ? keyLabel(b.emits) : describeMapping(b.mapping),
      emits: b.emits,
      firmwareAction: b.firmware ? describeMapping(b.mapping) : null,
      takenOver: b.taken_over,
      hasBackup: b.has_backup,
      hwIndex: b.index,
      x: null,
      y: null,
    }));
  } else {
    rows = (entry.summary?.buttons ?? []).map((name) => ({
      key: name,
      tag: keyLabel(name),
      label: keyLabel(name),
      emits: name,
      firmwareAction: null,
      takenOver: false,
      hasBackup: false,
      hwIndex: null,
      x: null,
      y: null,
    }));
  }

  // Sin claves repetidas (Svelte no las admite). Si dos botones físicos envían
  // lo mismo (p. ej. dos reprogramados con la misma tecla), se conservan los
  // dos con una clave propia para poder seleccionarlos y arreglarlo.
  const seen = new Set<string>();
  rows = rows
    .map((row) => {
      if (!seen.has(row.key)) return row;
      return row.hwIndex != null ? { ...row, key: `hw:${row.hwIndex}`, duplicate: true } : null;
    })
    .filter((row): row is ButtonRow => {
      if (!row || seen.has(row.key)) return false;
      seen.add(row.key);
      return true;
    });

  // Botones con asignación que no salen arriba (p. ej. teclas detectadas a mano).
  for (const b of bindings) {
    if (!rows.some((r) => r.emits && sameKey(r.emits, b.button))) {
      const name = b.button.toUpperCase();
      rows.push({
        key: name,
        tag: keyLabel(name),
        label: keyLabel(name),
        emits: name,
        firmwareAction: null,
        takenOver: false,
        hasBackup: false,
        hwIndex: null,
        x: null,
        y: null,
      });
    }
  }
  return rows;
}
