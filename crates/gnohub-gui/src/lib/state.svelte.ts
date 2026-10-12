// Estado global de la interfaz: la configuración en edición, lo guardado y
// lo que se sabe del sistema.

import { api } from "./api";
import { t } from "./i18n/index.svelte";
import { REF_FIELDS } from "./actions";
import { deviceKey } from "./keys";
import type {
  ConfigFile,
  Definition,
  DeviceDef,
  DeviceKind,
  DeviceSummary,
  FocusStatus,
  Game,
  Hardware,
  InstalledApp,
  LockedDevice,
  MacroRef,
  SystemStatus,
} from "./types";

export type View = { kind: "device"; key: string } | { kind: "macros"; name?: string } | { kind: "settings" };

export interface DeviceEntry {
  key: string;
  name: string;
  kind: DeviceKind;
  vendor: number;
  product: number;
  connected: boolean;
  summary?: DeviceSummary;
}

const EMPTY: ConfigFile = { macros: {}, devices: [] };

/** Retardo entre pasos de una macro nueva, igual que el valor por defecto del daemon. */
export const DEFAULT_STEP_DELAY = 10;

class AppState {
  config = $state<ConfigFile>(structuredClone(EMPTY));
  savedJson = $state(JSON.stringify(EMPTY));
  path = $state("");
  devices = $state<DeviceSummary[]>([]);
  locked = $state<LockedDevice[]>([]);
  installedApps = $state<InstalledApp[]>([]);
  games = $state<Game[]>([]);
  focus = $state<FocusStatus | null>(null);
  definitions = $state<Definition[]>([]);
  definitionErrors = $state<string[]>([]);
  /** Hardware por dispositivo (`null` si ratbagd no lo soporta). */
  hardware = $state<Record<string, Hardware | null>>({});
  /** Dibujo de Piper por dispositivo (`null` si no hay). */
  piperSvgs = $state<Record<string, string | null>>({});
  keys = $state<string[]>([]);
  status = $state<SystemStatus | null>(null);
  validationError = $state<string | null>(null);
  loadError = $state<string | null>(null);
  toast = $state<{ kind: "ok" | "error"; text: string } | null>(null);
  view = $state<View>({ kind: "settings" });

  dirty = $derived(JSON.stringify(this.config) !== this.savedJson);

  /** Dispositivos conectados más los configurados que no estén conectados. */
  entries = $derived.by((): DeviceEntry[] => {
    const list: DeviceEntry[] = this.devices.map((d) => ({
      key: deviceKey(d.vendor, d.product),
      name: d.name,
      kind: d.kind,
      vendor: d.vendor,
      product: d.product,
      connected: true,
      summary: d,
    }));
    for (const def of this.config.devices) {
      const key = deviceKey(def.vendor, def.product);
      if (!list.some((e) => e.key === key)) {
        list.push({
          key,
          name: def.name ?? key,
          kind: "other",
          vendor: def.vendor,
          product: def.product,
          connected: false,
        });
      }
    }
    return list;
  });

  async init() {
    try {
      const [loaded, devices, locked, keys] = await Promise.all([
        api.loadConfig(),
        api.listDevices(),
        api.lockedDevices(),
        api.keyNames(),
      ]);
      this.locked = locked;
      this.path = loaded.path;
      this.config = loaded.file;
      this.savedJson = JSON.stringify(loaded.file);
      this.devices = devices;
      this.keys = keys;
      const first = this.entries.find((e) => this.deviceDef(e.key)) ?? this.entries.find((e) => e.kind === "mouse");
      if (first) this.view = { kind: "device", key: first.key };
    } catch (e) {
      this.loadError = String(e);
    }
    await this.refreshStatus();
    // Parado: arrancarlo. Instalado, arranca el servicio de systemd; sin
    // instalar, la app ejecuta el daemon mientras está abierta.
    const service = this.status?.service;
    if (service && !service.active) {
      try {
        this.status!.service = await api.serviceAction(service.enabled || !service.installed ? "start" : "install");
      } catch (e) {
        this.notify("error", t("app.serviceStartFailed", { error: String(e) }));
      }
    }
    await this.refreshFocus().catch(() => {});
    await this.loadDefinitions();
  }

  async loadDefinitions() {
    try {
      const loaded = await api.deviceDefinitions();
      this.definitions = loaded.definitions;
      this.definitionErrors = loaded.errors;
    } catch (e) {
      this.definitionErrors = [String(e)];
    }
  }

  definitionFor(key: string): Definition | undefined {
    return this.definitions.find((d) => d.matches.includes(key));
  }

  async loadPiperSvg(entry: { key: string; vendor: number; product: number }) {
    if (entry.key in this.piperSvgs) return;
    this.piperSvgs[entry.key] = await api.piperSvg(entry.vendor, entry.product).catch(() => null);
  }

  async loadHardware(entry: { key: string; vendor: number; product: number }) {
    try {
      this.hardware[entry.key] = await api.hardware(entry.vendor, entry.product);
    } catch (e) {
      this.hardware[entry.key] = null;
      this.notify("error", t("app.hardwareReadFailed", { error: String(e) }));
    }
  }

  async refreshDevices() {
    [this.devices, this.locked] = await Promise.all([api.listDevices(), api.lockedDevices()]);
  }

  async grantAccess(device: LockedDevice) {
    try {
      await api.grantAccess(device.vendor, device.product);
      await this.refreshDevices();
      await this.refreshStatus();
      this.view = { kind: "device", key: deviceKey(device.vendor, device.product) };
      this.notify("ok", t("app.accessGranted", { name: device.name }));
    } catch (e) {
      this.notify("error", String(e));
    }
  }

  async refreshStatus() {
    try {
      this.status = await api.systemStatus();
    } catch (e) {
      this.notify("error", String(e));
    }
  }

  deviceDef(key: string): DeviceDef | undefined {
    return this.config.devices.find((d) => deviceKey(d.vendor, d.product) === key);
  }

  addDevice(entry: DeviceEntry): DeviceDef {
    this.config.devices.push({ name: entry.name, vendor: entry.vendor, product: entry.product, bindings: [] });
    return this.config.devices[this.config.devices.length - 1];
  }

  removeDevice(key: string) {
    this.config.devices = this.config.devices.filter((d) => deviceKey(d.vendor, d.product) !== key);
  }

  /** Cuántas asignaciones usan la macro con nombre `name`. */
  macroUsage(name: string): number {
    let count = 0;
    for (const binding of this.allBindings()) {
      for (const field of REF_FIELDS) {
        if ((binding as Record<string, unknown>)[field] === name) count++;
      }
    }
    return count;
  }

  renameMacro(from: string, to: string) {
    if (from === to || !to || this.config.macros[to]) return false;
    const macros: Record<string, (typeof this.config.macros)[string]> = {};
    for (const [name, def] of Object.entries(this.config.macros)) {
      macros[name === from ? to : name] = def;
    }
    this.config.macros = macros;
    for (const binding of this.allBindings()) {
      const fields = binding as Record<string, unknown>;
      for (const field of REF_FIELDS) {
        if (fields[field] === from) fields[field] = to;
      }
    }
    return true;
  }

  /** Asignaciones de todos los dispositivos, incluidas las de sus perfiles. */
  *allBindings() {
    for (const device of this.config.devices) {
      yield* device.bindings;
      for (const profile of device.apps ?? []) yield* profile.bindings;
    }
  }

  async loadApps() {
    if (this.installedApps.length === 0) {
      [this.installedApps, this.games] = await Promise.all([api.installedApps(), api.installedGames()]);
    }
  }

  async refreshFocus() {
    this.focus = await api.focusStatus();
  }

  /** Retardo entre pasos de la macro a la que apunta `ref`. */
  stepDelayOf(ref: MacroRef | undefined): number {
    if (ref === undefined) return DEFAULT_STEP_DELAY;
    if (typeof ref === "string") return this.config.macros[ref]?.step_delay ?? DEFAULT_STEP_DELAY;
    return ref.step_delay;
  }

  newMacroName(base = "macro"): string {
    for (let i = 1; ; i++) {
      const name = `${base}_${i}`;
      if (!this.config.macros[name]) return name;
    }
  }

  async save() {
    const file = $state.snapshot(this.config);
    try {
      await api.saveConfig(file);
      this.savedJson = JSON.stringify(file);
      const active = this.status?.service.active;
      this.notify("ok", active ? t("app.saved") : t("app.savedServiceOff"));
    } catch (e) {
      this.notify("error", String(e));
    }
  }

  discard() {
    this.config = JSON.parse(this.savedJson);
  }

  notify(kind: "ok" | "error", text: string) {
    this.toast = { kind, text };
    const current = this.toast;
    setTimeout(() => {
      if (this.toast === current) this.toast = null;
    }, 4000);
  }
}

export const app = new AppState();
