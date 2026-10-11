<script lang="ts">
  import { untrack } from "svelte";
  import { api } from "../api";
  import { t } from "../i18n/index.svelte";
  import { app, type DeviceEntry } from "../state.svelte";
  import type { Led } from "../types";
  import Icon from "./Icon.svelte";

  let { entry, led, onclose }: { entry: DeviceEntry; led: Led; onclose: () => void } = $props();


  // Copia local para editar sin escribir en el ratón a cada movimiento del selector.
  // Copia del estado inicial; la página recrea el editor al cambiar de LED.
  const initial = untrack(() => led);
  let mode = $state(initial.mode);
  let color = $state(toHex(initial.color));
  let duration = $state(initial.duration || 3000);
  let brightness = $state(initial.brightness);
  let saving = $state(false);

  function toHex([r, g, b]: number[]): string {
    return "#" + [r, g, b].map((c) => c.toString(16).padStart(2, "0")).join("");
  }

  function fromHex(hex: string): [number, number, number] {
    const n = parseInt(hex.slice(1), 16);
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  }

  const PRESETS = ["#f0a63a", "#ff3b30", "#ff2d95", "#8000ff", "#2f6bff", "#00c2ff", "#00d17a", "#ffffff"];

  const usesColor = $derived(mode === 1 || mode === 3);
  const usesSpeed = $derived(mode === 2 || mode === 3);
  const dirty = $derived(
    mode !== led.mode || color !== toHex(led.color) || brightness !== led.brightness || (usesSpeed && duration !== led.duration),
  );

  async function apply() {
    saving = true;
    try {
      await api.setLed(entry.vendor, entry.product, led.index, { mode, color: fromHex(color), duration, brightness });
      await app.loadHardware(entry);
      app.notify("ok", t("led.saved"));
    } catch (e) {
      app.notify("error", String(e));
    } finally {
      saving = false;
    }
  }
</script>

<section class="led-editor" aria-label={t("led.title", { n: led.index + 1 })}>
  <header>
    <div class="title">
      <span class="swatch" style:background={mode === 0 ? "transparent" : color}></span>
      <h2>{t("led.title", { n: led.index + 1 })}</h2>
    </div>
    <button type="button" class="ghost" onclick={onclose} title={t("binding.close")}><Icon name="x" /></button>
  </header>

  <fieldset class="modes">
    <legend>{t("led.effect")}</legend>
    {#each led.modes as m (m)}
      <label class="mode" class:active={mode === m}>
        <input type="radio" name="led-mode" value={m} checked={mode === m} onchange={() => (mode = m)} />
        {t(`led.modes.${m}`)}
      </label>
    {/each}
  </fieldset>

  {#if usesColor && led.color_depth > 0}
    <div class="field">
      {t("led.color")}
      <div class="colors">
        <input type="color" bind:value={color} aria-label={t("led.pickColor")} />
        {#each PRESETS as preset (preset)}
          <button
            type="button"
            class="preset"
            class:active={color === preset}
            style:background={preset}
            onclick={() => (color = preset)}
            aria-label="{t('led.color')} {preset}"
          ></button>
        {/each}
      </div>
    </div>
  {/if}

  {#if usesSpeed}
    <label class="field">
      {t("led.duration", { seconds: (duration / 1000).toFixed(1) })}
      <input type="range" min="500" max="10000" step="100" bind:value={duration} />
    </label>
  {/if}

  {#if mode !== 0}
    <label class="field">
      {t("led.brightness", { percent: Math.round((brightness / 255) * 100) })}
      <input type="range" min="0" max="255" bind:value={brightness} />
    </label>
  {/if}

  <div>
    <button type="button" class="primary" disabled={!dirty || saving} onclick={apply}>
      {saving ? t("led.saving") : t("led.apply")}
    </button>
  </div>
  <p class="hint">{t("led.hint")}</p>
</section>

<style>
  .led-editor {
    display: grid;
    gap: 18px;
    align-content: start;
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: start;
  }

  .title {
    display: flex;
    gap: 12px;
    align-items: center;
  }

  .swatch {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    border: 1px solid var(--line);
  }

  .modes {
    border: 0;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  legend {
    color: var(--muted);
    font-size: 13px;
    margin-bottom: 6px;
  }

  .mode {
    padding: 8px 12px;
    border: 1px solid var(--line);
    border-radius: var(--radius-m);
    cursor: pointer;
  }

  .mode input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .mode:has(input:focus-visible) {
    outline: 2px solid var(--signal);
  }

  .mode.active {
    border-color: var(--signal);
    background: var(--signal-soft);
    color: var(--signal);
  }

  .field {
    display: grid;
    gap: 8px;
    color: var(--muted);
    font-size: 13px;
  }

  .colors {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }

  input[type="color"] {
    width: 44px;
    height: 32px;
    padding: 2px;
  }

  .preset {
    width: 26px;
    height: 26px;
    padding: 0;
    border-radius: 50%;
    border: 2px solid transparent;
  }

  .preset.active {
    border-color: var(--text);
  }

  input[type="range"] {
    accent-color: var(--signal);
    max-width: 320px;
  }
</style>
