// Estado de las actualizaciones: si hay versión nueva, y el progreso de instalarla.

import { api, onUpdateProgress } from "./api";
import type { UpdateInfo } from "./types";

type Phase = "idle" | "checking" | "installing" | "installed" | "error";

class Updates {
  info = $state<UpdateInfo | null>(null);
  phase = $state<Phase>("idle");
  progress = $state<{ downloaded: number; total: number | null } | null>(null);
  error = $state<string | null>(null);
  /** El aviso se cerró hasta la próxima búsqueda que encuentre otra versión. */
  dismissed = $state<string | null>(null);

  get visible(): boolean {
    return !!this.info?.newer && this.dismissed !== this.info.latest;
  }

  async check(): Promise<UpdateInfo | null> {
    if (this.phase === "installing") return this.info;
    this.phase = "checking";
    this.error = null;
    try {
      this.info = await api.updateCheck();
      this.phase = "idle";
    } catch (e) {
      this.error = String(e);
      this.phase = "error";
    }
    return this.info;
  }

  async install() {
    this.phase = "installing";
    this.error = null;
    this.progress = null;
    const stop = await onUpdateProgress((p) => (this.progress = p));
    try {
      await api.updateInstall();
      this.phase = "installed";
    } catch (e) {
      this.error = String(e);
      this.phase = "error";
    } finally {
      stop();
    }
  }

  async download() {
    const url = this.info?.download_url ?? this.info?.page_url;
    if (url) await api.openReleaseUrl(url).catch((e) => (this.error = String(e)));
  }

  restart() {
    return api.updateRestart();
  }

  dismiss() {
    this.dismissed = this.info?.latest ?? null;
  }
}

export const updates = new Updates();
