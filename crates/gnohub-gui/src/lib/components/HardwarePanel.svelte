<script lang="ts">
  import { api } from "../api";
  import { t } from "../i18n/index.svelte";
  import { app, type DeviceEntry } from "../state.svelte";
  import type { Hardware } from "../types";

  let { entry, hardware }: { entry: DeviceEntry; hardware: Hardware } = $props();

  let busy = $state(false);

  const levels = $derived(hardware.resolutions.filter((r) => !r.disabled));

  async function apply(action: () => Promise<void>, done: string) {
    busy = true;
    try {
      await action();
      await app.loadHardware(entry);
      app.notify("ok", done);
    } catch (e) {
      app.notify("error", String(e));
    } finally {
      busy = false;
    }
  }
</script>

<section class="hw" aria-label={t("hardware.dpi")}>
  <div class="group">
    <span class="title">{t("hardware.dpi")}</span>
    <div class="levels">
      {#each levels as level (level.index)}
        <label class="level" class:active={level.active}>
          <select
            value={level.dpi}
            disabled={busy}
            aria-label={t("hardware.dpiLevel", { n: level.index + 1 })}
            onchange={(e) => {
              const dpi = Number(e.currentTarget.value);
              apply(() => api.setDpi(entry.vendor, entry.product, level.index, dpi), t("hardware.dpiChanged", { dpi }));
            }}
          >
            {#each hardware.dpi_values as dpi (dpi)}
              <option value={dpi}>{dpi}</option>
            {/each}
          </select>
          {#if level.active}<span class="now">{t("hardware.current")}</span>{/if}
        </label>
      {/each}
    </div>
  </div>

  {#if hardware.report_rates.length > 1}
    <label class="group">
      <span class="title">{t("hardware.reportRate")}</span>
      <select
        value={hardware.report_rate}
        disabled={busy}
        onchange={(e) => {
          const rate = Number(e.currentTarget.value);
          apply(() => api.setReportRate(entry.vendor, entry.product, rate), t("hardware.rateChanged", { rate }));
        }}
      >
        {#each hardware.report_rates as rate (rate)}
          <option value={rate}>{rate} Hz</option>
        {/each}
      </select>
    </label>
  {/if}

  <p class="hint">
    {t("hardware.savedNow", { profile: hardware.profile + 1, profiles: hardware.profiles })}
  </p>
</section>

<style>
  .hw {
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    gap: 14px 24px;
    padding: 12px 14px;
    margin-bottom: 16px;
    border: 1px solid var(--line);
    border-radius: var(--radius-m);
  }

  .group {
    display: grid;
    gap: 4px;
  }

  .title {
    color: var(--muted);
    font-size: 13px;
  }

  .levels {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .level {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .level.active select {
    border-color: var(--signal);
  }

  .now {
    color: var(--signal);
    font-size: 12px;
  }

  .hint {
    flex-basis: 100%;
  }
</style>
