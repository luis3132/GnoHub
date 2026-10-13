<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "./lib/api";
  import DevicePage from "./lib/components/DevicePage.svelte";
  import Icon from "./lib/components/Icon.svelte";
  import MacrosPage from "./lib/components/MacrosPage.svelte";
  import SettingsPage from "./lib/components/SettingsPage.svelte";
  import UpdateNotifier from "./lib/components/UpdateNotifier.svelte";
  import { i18n, initLocale, setLocale, t } from "./lib/i18n/index.svelte";
  import { app } from "./lib/state.svelte";
  import type { LockedDevice } from "./lib/types";

  const KIND_ICON = { mouse: "mouse", keyboard: "keyboard", other: "gamepad" } as const;

  let refreshing = $state(false);
  let granting = $state(false);

  // Botones de encendido, tomas de audio, etc. no interesan aquí.
  const lockedMain = $derived(app.locked.filter((d) => d.kind !== "other"));

  async function grant(device: LockedDevice) {
    granting = true;
    await app.grantAccess(device);
    granting = false;
  }

  onMount(async () => {
    await initLocale();
    await app.init();
    // Solo en la vista previa del navegador: #dispositivo=VID:PID abre ese dispositivo.
    if (import.meta.env.DEV) {
      const m = /dispositivo=([0-9a-f]{4}:[0-9a-f]{4})/.exec(location.hash);
      if (m) app.view = { kind: "device", key: m[1] };
      const lang = /lang=(\w+)/.exec(location.hash)?.[1];
      if (lang === "en" || lang === "es") await setLocale(lang, false);
      if (location.hash.includes("ajustes")) app.view = { kind: "settings" };
    }
  });

  // Validación en vivo con el mismo código que usa el daemon.
  // Se repite al cambiar de idioma para que el mensaje de error salga traducido.
  $effect(() => {
    const file = $state.snapshot(app.config);
    void i18n.locale;
    const timer = setTimeout(async () => {
      try {
        await api.validateConfig(file);
        app.validationError = null;
      } catch (e) {
        app.validationError = String(e);
      }
    }, 250);
    return () => clearTimeout(timer);
  });

  const currentEntry = $derived(app.view.kind === "device" ? app.entries.find((e) => e.key === (app.view as { key: string }).key) : undefined);

  async function startService() {
    try {
      const service = app.status!.service;
      app.status!.service = await api.serviceAction(service.enabled || !service.installed ? "start" : "install");
    } catch (e) {
      app.notify("error", String(e));
    }
  }

  async function refresh() {
    refreshing = true;
    await Promise.all([app.refreshDevices(), app.refreshStatus()]);
    refreshing = false;
  }

  function onKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
      e.preventDefault();
      if (app.dirty && !app.validationError) app.save();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<datalist id="gnohub-keys">
  {#each app.keys as key (key)}
    <option value={key}></option>
  {/each}
</datalist>

<div class="shell">
  <aside class="rail">
    <div class="brand">
      <img src="/logo.svg" alt="" width="28" height="28" />
      <span>GnoHub</span>
    </div>

    <div class="rail-head">
      <h3>{t("app.devices")}</h3>
      <button type="button" class="ghost" onclick={refresh} disabled={refreshing} title={t("app.findDevices")}>
        <Icon name="refresh" size={15} />
      </button>
    </div>
    <nav aria-label={t("app.devices")}>
      {#each app.entries as entry (entry.key)}
        {@const configured = app.deviceDef(entry.key)}
        <button
          type="button"
          class="nav-item"
          class:current={app.view.kind === "device" && currentEntry?.key === entry.key}
          class:offline={!entry.connected}
          onclick={() => (app.view = { kind: "device", key: entry.key })}
        >
          <Icon name={KIND_ICON[entry.kind]} />
          <span class="nav-text">
            <span class="nav-name">{configured?.name ?? entry.name}</span>
            <span class="nav-sub">
              {configured
                ? t("app.assignments", { count: configured.bindings.length })
                : entry.connected
                  ? t("app.notConfigured")
                  : t("app.disconnected")}
            </span>
          </span>
        </button>
      {:else}
        <p class="hint rail-empty">{t("app.noDevices")}</p>
      {/each}

      {#if lockedMain.length > 0}
        <h3 class="rail-sub">{t("app.noAccess")}</h3>
        {#each lockedMain as device (device.vendor + ":" + device.product)}
          <div class="locked">
            <Icon name={KIND_ICON[device.kind]} />
            <span class="nav-text">
              <span class="nav-name">{device.name}</span>
              <button
                type="button"
                class="link"
                disabled={granting}
                onclick={() => grant(device)}
                title={device.kind === "keyboard" ? t("app.grantKeyboardTitle") : t("app.grantAccessTitle")}
              >
                {t("app.grantAccess")}
              </button>
            </span>
          </div>
        {/each}
      {/if}
    </nav>

    <div class="rail-foot">
      <button type="button" class="nav-item" class:current={app.view.kind === "macros"} onclick={() => (app.view = { kind: "macros" })}>
        <Icon name="macro" />
        <span class="nav-name">{t("app.macros")}</span>
        <span class="count">{Object.keys(app.config.macros).length}</span>
      </button>
      <button type="button" class="nav-item" class:current={app.view.kind === "settings"} onclick={() => (app.view = { kind: "settings" })}>
        <Icon name="settings" />
        <span class="nav-name">{t("app.settings")}</span>
        <span
          class="status-dot"
          class:on={app.status?.service.active}
          title={app.status?.service.active ? t("app.serviceRunning") : t("app.serviceStopped")}
        ></span>
      </button>
    </div>
  </aside>

  <main>
    {#if app.status && !app.status.service.active}
      <div class="service-banner" role="alert">
        <Icon name="alert" size={16} />
        <span>{t("app.serviceStoppedBanner")}</span>
        <button type="button" class="primary" onclick={startService}>{t("app.start")}</button>
      </div>
    {:else if app.status?.service.embedded}
      <div class="service-banner info" role="status">
        <Icon name="alert" size={16} />
        <span>{t("app.embeddedBanner", { command: "sudo dnf install GnoHub-*.rpm" })}</span>
      </div>
    {/if}
    {#if app.loadError}
      <div class="load-error">
        <h2>{t("app.loadErrorTitle")}</h2>
        <p>{app.loadError}</p>
        <p class="hint">{t("app.loadErrorHint")}</p>
      </div>
    {:else}
      <!-- Si una página falla se muestra el error en vez de dejar la app congelada. -->
      <svelte:boundary onerror={(e) => api.logError(`page: ${e instanceof Error ? (e.stack ?? e.message) : String(e)}`)}>
        {#if app.view.kind === "device" && currentEntry}
          <!-- Página nueva por dispositivo, para que el editor abierto no pase de uno a otro. -->
          {#key currentEntry.key}
            <DevicePage entry={currentEntry} />
          {/key}
        {:else if app.view.kind === "macros"}
          <MacrosPage initial={app.view.name} />
        {:else}
          <SettingsPage />
        {/if}

        {#snippet failed(error, reset)}
          <div class="load-error">
            <h2>{t("app.pageFailed")}</h2>
            <p>{error instanceof Error ? error.message : String(error)}</p>
            <p class="hint">{t("app.pageFailedHint")}</p>
            <div><button type="button" onclick={reset}>{t("app.retry")}</button></div>
          </div>
        {/snippet}
      </svelte:boundary>
    {/if}
  </main>

  {#if app.dirty || app.validationError}
    <footer class="savebar" class:invalid={app.validationError}>
      {#if app.validationError}
        <Icon name="alert" size={16} />
        <span class="msg">{app.validationError}</span>
      {:else}
        <span class="msg">{t("app.unsaved")}</span>
      {/if}
      {#if app.dirty}
        <button type="button" class="ghost" onclick={() => app.discard()}>{t("app.discard")}</button>
        <button type="button" class="primary" disabled={!!app.validationError} onclick={() => app.save()}>
          {t("app.save")}
        </button>
      {/if}
    </footer>
  {/if}

  <UpdateNotifier />

  {#if app.toast}
    <div class="toast" class:error={app.toast.kind === "error"} role="status">{app.toast.text}</div>
  {/if}
</div>

<style>
  .shell {
    display: grid;
    grid-template-columns: 250px 1fr;
    grid-template-rows: 1fr auto;
    height: 100%;
  }

  .rail {
    grid-row: 1 / 3;
    background: var(--rail);
    border-right: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 16px 10px;
    gap: 8px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 8px 14px;
    font-weight: 700;
    font-size: 17px;
  }

  .rail-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 4px 0 8px;
    color: var(--muted);
  }

  .rail-head h3 {
    font-weight: 500;
    font-size: 13px;
  }

  .rail-head button {
    padding: 4px;
  }

  nav {
    flex: 1;
    overflow-y: auto;
    display: grid;
    align-content: start;
    gap: 2px;
  }

  .nav-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    text-align: left;
    background: transparent;
    border-color: transparent;
    padding: 8px;
    color: var(--muted);
  }

  .nav-item:hover:not(:disabled) {
    color: var(--text);
    border-color: transparent;
    background: var(--panel);
  }

  .nav-item.current {
    color: var(--text);
    background: var(--panel-2);
    box-shadow: inset 3px 0 0 var(--signal);
  }

  .nav-item.offline {
    opacity: 0.6;
  }

  .nav-text {
    display: grid;
    min-width: 0;
  }

  .nav-name {
    color: inherit;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .nav-sub {
    font-size: 12px;
    color: var(--faint);
  }

  .rail-empty {
    padding: 8px;
  }

  .rail-sub {
    font-weight: 500;
    font-size: 13px;
    color: var(--muted);
    padding: 14px 8px 4px;
  }

  .locked {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 8px;
    color: var(--faint);
  }

  .link {
    justify-self: start;
    padding: 0;
    border: 0;
    background: none;
    color: var(--signal);
    font-size: 12.5px;
  }

  .link:hover:not(:disabled) {
    text-decoration: underline;
  }

  .rail-foot {
    border-top: 1px solid var(--line);
    padding-top: 8px;
    display: grid;
    gap: 2px;
  }

  .count {
    margin-left: auto;
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }

  .status-dot {
    margin-left: auto;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--faint);
  }

  .status-dot.on {
    background: var(--ok);
  }

  main {
    min-height: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  main > :global(*:not(.service-banner)) {
    flex: 1 1 0;
    min-height: 0;
  }

  .service-banner {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 20px;
    background: var(--signal-soft);
    border-bottom: 1px solid var(--signal);
    color: var(--signal);
  }

  .service-banner span {
    flex: 1;
  }

  .service-banner.info {
    background: transparent;
    border-bottom-color: var(--line);
    color: var(--muted);
  }



  .load-error {
    padding: 28px;
    display: grid;
    gap: 10px;
    max-width: 70ch;
  }

  .load-error p:first-of-type {
    color: var(--danger);
    font-family: var(--mono);
    font-size: 12px;
    white-space: pre-wrap;
  }

  .savebar {
    grid-column: 2;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 20px;
    border-top: 1px solid var(--line);
    background: var(--panel);
  }

  .savebar .msg {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .savebar.invalid {
    color: var(--danger);
  }

  .toast {
    position: fixed;
    right: 20px;
    bottom: 64px;
    max-width: 420px;
    padding: 10px 14px;
    border-radius: var(--radius-m);
    background: var(--panel-2);
    border: 1px solid var(--ok);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
  }

  .toast.error {
    border-color: var(--danger);
  }
</style>
