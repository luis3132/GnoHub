import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import type {
  ConfigFile,
  Definition,
  DeviceSummary,
  Hardware,
  Led,
  Mapping,
  FocusStatus,
  Game,
  InstalledApp,
  LoadedConfig,
  LockedDevice,
  ServiceAction,
  ServiceStatus,
  SystemStatus,
  UpdateInfo,
} from "./types";

// Fuera de Tauri (vista previa en el navegador) se usa un backend simulado.
const inTauri = "__TAURI_INTERNALS__" in window;

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) return tauriInvoke<T>(cmd, args);
  const { mockInvoke } = await import("./mock");
  return (await mockInvoke(cmd, args)) as T;
}

/** Progreso de la descarga de una actualización (evento del backend). */
export async function onUpdateProgress(cb: (p: { downloaded: number; total: number | null }) => void) {
  if (!inTauri) return () => {};
  const { listen } = await import("@tauri-apps/api/event");
  return listen<{ downloaded: number; total: number | null }>("gnohub-update-progress", (e) => cb(e.payload));
}

export const api = {
  /** Registra un error de la interfaz en la salida estándar de error de la app. */
  logError: (message: string) => invoke<void>("log_frontend_error", { message }).catch(() => {}),
  loadConfig: () => invoke<LoadedConfig>("load_config"),
  validateConfig: (file: ConfigFile) => invoke<void>("validate_config", { file }),
  saveConfig: (file: ConfigFile) => invoke<void>("save_config", { file }),
  listDevices: () => invoke<DeviceSummary[]>("list_devices"),
  lockedDevices: () => invoke<LockedDevice[]>("locked_devices"),
  grantAccess: (vendor: number, product: number) => invoke<void>("grant_access", { vendor, product }),
  installedApps: () => invoke<InstalledApp[]>("installed_apps"),
  installedGames: () => invoke<Game[]>("installed_games"),
  focusStatus: () => invoke<FocusStatus>("focus_status"),
  enableGnomeExtension: () => invoke<FocusStatus>("enable_gnome_extension"),
  deviceDefinitions: () => invoke<{ definitions: Definition[]; errors: string[] }>("device_definitions"),
  hardware: (vendor: number, product: number) => invoke<Hardware | null>("hardware", { vendor, product }),
  setDpi: (vendor: number, product: number, index: number, dpi: number) =>
    invoke<void>("set_dpi", { vendor, product, index, dpi }),
  setReportRate: (vendor: number, product: number, rate: number) =>
    invoke<void>("set_report_rate", { vendor, product, rate }),
  takeOverButton: (vendor: number, product: number, index: number) =>
    invoke<string>("take_over_button", { vendor, product, index }),
  /** Sin `mapping`, restaura la copia guardada de la función original. */
  restoreButton: (vendor: number, product: number, index: number, mapping?: Mapping) =>
    invoke<void>("restore_button", { vendor, product, index, mapping: mapping ?? null }),
  updateCheck: () => invoke<UpdateInfo>("update_check"),
  updateInstall: () => invoke<void>("update_install"),
  updateRestart: () => invoke<void>("update_restart"),
  openReleaseUrl: (url: string) => invoke<void>("open_release_url", { url }),
  systemLocale: () => invoke<string>("system_locale"),
  setLocale: (locale: string) => invoke<void>("set_locale", { locale }),
  setLed: (vendor: number, product: number, index: number, led: Pick<Led, "mode" | "color" | "duration" | "brightness">) =>
    invoke<void>("set_led", { vendor, product, index, led }),
  piperSvg: (vendor: number, product: number) => invoke<string | null>("piper_svg", { vendor, product }),
  keyNames: () => invoke<string[]>("key_names"),
  systemStatus: () => invoke<SystemStatus>("system_status"),
  serviceAction: (action: ServiceAction) => invoke<ServiceStatus>("service_action", { action }),
  detectButton: (vendor: number, product: number, timeoutMs: number) =>
    invoke<string | null>("detect_button", { vendor, product, timeoutMs }),
};
