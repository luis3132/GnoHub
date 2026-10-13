<script lang="ts">
  // Aviso de versión nueva, en una esquina: no interrumpe lo que se está haciendo.
  // Con actualización automática, "Actualizar" baja, verifica e instala el paquete (el
  // sistema pide la contraseña) y "Reiniciar" abre la versión nueva. Sin ella, ofrece
  // descargar el paquete de este sistema.

  import { onMount } from "svelte";
  import { t } from "../i18n/index.svelte";
  import { updates } from "../updates.svelte";
  import Icon from "./Icon.svelte";

  /** Primera búsqueda un rato después de abrir, y luego cada tanto. */
  const FIRST_CHECK_MS = 20_000;
  const EVERY_MS = 6 * 60 * 60_000;

  let notesOpen = $state(false);

  onMount(() => {
    const first = setTimeout(() => updates.check(), FIRST_CHECK_MS);
    const every = setInterval(() => updates.check(), EVERY_MS);
    return () => {
      clearTimeout(first);
      clearInterval(every);
    };
  });

  const info = $derived(updates.info);
  const pct = $derived(
    updates.progress?.total ? Math.round((updates.progress.downloaded / updates.progress.total) * 100) : null,
  );
</script>

{#if updates.visible && info}
  <aside class="notifier" role="status">
    <header>
      <div>
        <strong>{t("updates.available", { version: info.latest })}</strong>
        <span class="hint">{t("updates.current", { version: info.current })}</span>
      </div>
      {#if updates.phase !== "installing"}
        <button type="button" class="ghost close" onclick={() => updates.dismiss()} title={t("binding.close")}>
          <Icon name="x" size={14} />
        </button>
      {/if}
    </header>

    {#if info.notes}
      <button type="button" class="link" onclick={() => (notesOpen = !notesOpen)}>
        {notesOpen ? t("updates.hideNotes") : t("updates.showNotes")}
      </button>
      {#if notesOpen}<pre class="notes">{info.notes}</pre>{/if}
    {/if}

    {#if updates.phase === "installing"}
      <div class="bar" aria-label={t("updates.installing")}>
        <div class="fill" class:indeterminate={pct === null} style:width={pct === null ? "40%" : `${pct}%`}></div>
      </div>
      <span class="hint">{t("updates.installing")}{pct === null ? "" : ` ${pct} %`}</span>
    {:else if updates.phase === "installed"}
      <p class="hint">{t("updates.installed")}</p>
      <div class="actions">
        <button type="button" class="primary" onclick={() => updates.restart()}>{t("updates.restart")}</button>
      </div>
    {:else}
      {#if updates.error}<p class="error">{updates.error}</p>{/if}
      <div class="actions">
        {#if info.install === "auto"}
          <button type="button" class="primary" onclick={() => updates.install()}>{t("updates.update")}</button>
        {:else}
          <button type="button" class="primary" onclick={() => updates.download()}>
            {info.download_url ? t("updates.download", { name: info.asset_name ?? "" }) : t("updates.openPage")}
          </button>
        {/if}
      </div>
    {/if}
  </aside>
{/if}

<style>
  .notifier {
    position: fixed;
    right: 20px;
    bottom: 64px;
    z-index: 20;
    width: 360px;
    max-width: calc(100vw - 2rem);
    display: grid;
    gap: 10px;
    padding: 14px 16px;
    border: 1px solid var(--line);
    border-radius: var(--radius-m);
    background: var(--panel-2);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.4);
  }

  header {
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }

  header div {
    display: grid;
  }

  .close {
    padding: 2px;
    align-self: start;
  }

  .link {
    justify-self: start;
    padding: 0;
    border: 0;
    background: none;
    color: var(--signal);
    font-size: 12.5px;
  }

  .notes {
    margin: 0;
    max-height: 220px;
    overflow: auto;
    white-space: pre-wrap;
    font: inherit;
    font-size: 12.5px;
    padding: 8px 10px;
    border: 1px solid var(--line);
    border-radius: var(--radius-s);
    background: var(--bg);
  }

  .bar {
    height: 6px;
    border-radius: 3px;
    background: var(--bg);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--signal);
  }

  .fill.indeterminate {
    animation: slide 1.2s ease-in-out infinite;
  }

  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(250%);
    }
  }

  .actions {
    display: flex;
    justify-content: flex-end;
  }

  .error {
    color: var(--danger);
    font-size: 12.5px;
  }
</style>
