<script lang="ts">
  import { api } from "../api";
  import { i18n, LOCALES, setLocale, t, type Locale } from "../i18n/index.svelte";
  import { app } from "../state.svelte";
  import type { ServiceAction } from "../types";
  import Icon from "./Icon.svelte";

  let busy = $state(false);

  const status = $derived(app.status);
  const service = $derived(status?.service);
  const permissionsOk = $derived(status && status.unreadable === 0 && status.uinput_writable);

  async function act(action: ServiceAction) {
    busy = true;
    try {
      if (app.status) app.status.service = await api.serviceAction(action);
    } catch (e) {
      app.notify("error", String(e));
    } finally {
      busy = false;
    }
  }

  const DESKTOPS: Record<string, string> = {
    gnome: "GNOME",
    hyprland: "Hyprland",
    sway: "Sway",
    i3: "i3",
    x11: "X11",
  };

  async function enableExtension() {
    busy = true;
    try {
      app.focus = await api.enableGnomeExtension();
      app.notify("ok", app.focus.extension_active ? t("settings.extensionEnabled") : t("settings.extensionEnabledRelogin"));
    } catch (e) {
      app.notify("error", String(e));
    } finally {
      busy = false;
    }
  }
</script>

<div class="page">
  <h1>{t("settings.title")}</h1>

  <section>
    <label class="field language">
      {t("settings.language")}
      <select value={i18n.locale} onchange={(e) => setLocale(e.currentTarget.value as Locale)}>
        {#each Object.entries(LOCALES) as [code, name] (code)}
          <option value={code}>{name}</option>
        {/each}
      </select>
      <span class="hint">{t("settings.languageHint")}</span>
    </label>
  </section>

  <section>
    <div class="section-head">
      <h2>{t("settings.service")}</h2>
      <button type="button" class="ghost" onclick={() => app.refreshStatus()} title={t("settings.refresh")}>
        <Icon name="refresh" size={15} />
      </button>
    </div>
    <p class="hint">{t("settings.serviceHint", { daemon: "gnohubd" })}</p>
    {#if service}
      <ul class="states">
        <li class:on={service.installed}><Icon name={service.installed ? "check" : "x"} size={15} /> {t("settings.installed")}</li>
        <li class:on={service.active}><Icon name={service.active ? "check" : "x"} size={15} /> {t("settings.running")}</li>
        <li class:on={service.enabled}><Icon name={service.enabled ? "check" : "x"} size={15} /> {t("settings.atLogin")}</li>
      </ul>
      <div class="buttons">
        {#if service.embedded}
          <p class="warn"><Icon name="alert" size={15} /> {t("settings.embedded")}</p>
          <button type="button" disabled={busy} onclick={() => act("restart")}>{t("settings.restart")}</button>
        {:else if !service.installed}
          <p class="warn"><Icon name="alert" size={15} /> {t("settings.notInstalled")}</p>
          <button type="button" class="primary" disabled={busy} onclick={() => act("start")}>{t("settings.start")}</button>
        {:else if !service.enabled}
          <button type="button" class="primary" disabled={busy} onclick={() => act("install")}>
            {t("settings.enableAndStart")}
          </button>
        {:else if service.active}
          <button type="button" disabled={busy} onclick={() => act("restart")}>{t("settings.restart")}</button>
          <button type="button" disabled={busy} onclick={() => act("stop")}>{t("settings.stop")}</button>
        {:else}
          <button type="button" class="primary" disabled={busy} onclick={() => act("start")}>{t("settings.start")}</button>
        {/if}
      </div>
    {/if}
  </section>

  <section>
    <h2>{t("settings.profiles")}</h2>
    {#if app.focus}
      {#if app.focus.desktop === "gnome"}
        {#if app.focus.extension_active}
          <p class="ok"><Icon name="check" size={15} /> {t("settings.extensionActive")}</p>
        {:else}
          <p class="hint">{t("settings.extensionWhy")}</p>
          {#if app.focus.extension_installed}
            <p class="hint">{t("settings.extensionRelogin")}</p>
            <div class="buttons">
              <button type="button" class="primary" disabled={busy} onclick={enableExtension}>
                {t("settings.enableExtension")}
              </button>
            </div>
          {:else}
            <p class="warn"><Icon name="alert" size={15} /> {t("settings.extensionMissing")}</p>
          {/if}
        {/if}
      {:else if DESKTOPS[app.focus.desktop]}
        <p class="ok"><Icon name="check" size={15} /> {t("settings.desktopOk", { desktop: DESKTOPS[app.focus.desktop] })}</p>
      {:else}
        <p class="warn"><Icon name="alert" size={15} /> {t("settings.desktopUnknown")}</p>
      {/if}
    {/if}
  </section>

  <section>
    <h2>{t("settings.permissions")}</h2>
    {#if status}
      {#if permissionsOk}
        <p class="ok"><Icon name="check" size={15} /> {t("settings.permissionsOk")}</p>
      {:else}
        {#if status.unreadable > 0}
          <p class="warn"><Icon name="alert" size={15} /> {t("settings.unreadable", { count: status.unreadable })}</p>
        {/if}
        {#if !status.uinput_writable}
          <p class="warn"><Icon name="alert" size={15} /> {t("settings.uinput")}</p>
        {/if}
        <p class="hint">{t("settings.noInputGroup")}</p>
      {/if}
    {/if}
  </section>

  {#if app.definitionErrors.length > 0}
    <section>
      <h2>{t("settings.definitionErrors")}</h2>
      <p class="hint">{t("settings.definitionErrorsHint")}</p>
      {#each app.definitionErrors as error (error)}
        <p class="warn"><Icon name="alert" size={15} /> <span class="code">{error}</span></p>
      {/each}
    </section>
  {/if}

  <section>
    <h2>{t("settings.configFile")}</h2>
    <p class="code">{app.path}</p>
  </section>
</div>

<style>
  .page {
    padding: 24px 28px 48px;
    overflow-y: auto;
    height: 100%;
    display: grid;
    align-content: start;
    gap: 28px;
    max-width: 780px;
  }

  section {
    display: grid;
    gap: 10px;
  }

  .section-head {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .states {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    color: var(--faint);
  }

  .states li {
    display: inline-flex;
    gap: 6px;
    align-items: center;
  }

  .states li.on {
    color: var(--ok);
  }

  .language select {
    max-width: 220px;
  }

  .buttons {
    display: flex;
    gap: 8px;
  }

  .warn,
  .ok {
    display: flex;
    gap: 8px;
    align-items: start;
  }

  .warn {
    color: var(--signal);
  }

  .ok {
    color: var(--ok);
  }

</style>
