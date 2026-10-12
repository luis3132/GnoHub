<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../api";
  import { describeBinding, modeLabel } from "../actions";
  import { sameKey } from "../keys";
  import { buildRows, type ButtonRow } from "../rows";
  import { app, type DeviceEntry } from "../state.svelte";
  import AppPicker from "./AppPicker.svelte";
  import BindingEditor from "./BindingEditor.svelte";
  import DeviceFigure from "./DeviceFigure.svelte";
  import HardwarePanel from "./HardwarePanel.svelte";
  import Icon from "./Icon.svelte";
  import LedEditor from "./LedEditor.svelte";
  import TakenOverPanel from "./TakenOverPanel.svelte";
  import { localized, t } from "../i18n/index.svelte";

  let { entry }: { entry: DeviceEntry } = $props();

  /** Clave de la fila seleccionada (ver `ButtonRow.key`). */
  let selected = $state<string | null>(null);
  let selectedLed = $state<number | null>(null);
  let detecting = $state(false);
  let takingOver = $state(false);
  /** Perfil que se edita: -1 es el general; si no, índice en `def.apps`. */
  let profileIndex = $state(-1);

  onMount(() => {
    if (entry.connected && !(entry.key in app.hardware)) app.loadHardware(entry);
    app.loadPiperSvg(entry);
    // Solo en la vista previa del navegador: #boton=CLAVE o #led=N abren un panel.
    if (import.meta.env.DEV) {
      const button = /boton=([^&]+)/.exec(location.hash)?.[1];
      const ledIndex = /led=(\d+)/.exec(location.hash)?.[1];
      if (button) selected = decodeURIComponent(button);
      if (ledIndex) selectedLed = Number(ledIndex);
      if (location.hash.includes("perfil")) addProfile();
    }
  });

  const def = $derived(app.deviceDef(entry.key));
  const definition = $derived(app.definitionFor(entry.key));
  const hardware = $derived(app.hardware[entry.key]);
  const profile = $derived(def && profileIndex >= 0 ? def.apps?.[profileIndex] : undefined);
  const editing = $derived(profile ? profile.bindings : def?.bindings);

  const rows = $derived(
    buildRows(entry, definition, hardware, [...(def?.bindings ?? []), ...(def?.apps ?? []).flatMap((p) => p.bindings)]),
  );
  const selectedRow = $derived(rows.find((r) => r.key === selected));
  const led = $derived(selectedLed == null ? undefined : hardware?.leds.find((l) => l.index === selectedLed));
  // La imagen de la definición manda; si no hay, el dibujo de Piper.
  const figureSvg = $derived(definition?.svg ?? (definition?.image ? null : app.piperSvgs[entry.key]));
  const hasFigure = $derived(!!figureSvg || !!definition?.image);

  /** Texto corto de lo que hace un botón, para las etiquetas del dibujo. */
  function describeRow(row: ButtonRow): string {
    if (!row.emits) return row.firmwareAction ?? t("mapping.handledByFirmware");
    const own = ownFor(row.emits);
    const b = own ?? (profile ? generalFor(row.emits) : undefined);
    if (!b) return row.label;
    if (b.mode === "native" || b.mode === "disabled") return modeLabel(b.mode);
    const text = describeBinding(b);
    return text ? `${modeLabel(b.mode)}: ${text}` : modeLabel(b.mode);
  }

  const generalFor = (button: string) => def?.bindings.find((b) => sameKey(b.button, button));
  const ownFor = (button: string) => editing?.find((b) => sameKey(b.button, button));
  const isAssigned = (row: ButtonRow) => {
    if (!row.emits) return false;
    const b = ownFor(row.emits) ?? (profile ? generalFor(row.emits) : undefined);
    return !!b && b.mode !== "native";
  };

  function addProfile() {
    const d = def ?? app.addDevice(entry);
    d.apps ??= [];
    d.apps.push({ name: t("device.newProfile", { n: d.apps.length + 1 }), apps: [], bindings: [] });
    profileIndex = d.apps.length - 1;
    selected = null;
  }

  function removeProfile() {
    if (!def?.apps || profileIndex < 0) return;
    def.apps.splice(profileIndex, 1);
    if (def.apps.length === 0) delete def.apps;
    profileIndex = -1;
    selected = null;
  }

  async function detect() {
    detecting = true;
    try {
      const key = await api.detectButton(entry.vendor, entry.product, 8000);
      if (key) {
        if (!def) app.addDevice(entry);
        selected = rows.find((r) => r.emits && sameKey(r.emits, key))?.key ?? key.toUpperCase();
      } else {
        app.notify("error", t("device.detectNothing"));
      }
    } catch (e) {
      app.notify("error", String(e));
    } finally {
      detecting = false;
    }
  }

  function open(row: ButtonRow) {
    if (row.emits && !def) app.addDevice(entry);
    selected = row.key;
    selectedLed = null;
  }

  /** El ratón envía este botón como tecla pulsada y soltada al instante. */
  function instantKeyOf(row: ButtonRow): boolean {
    const kind = hardware?.buttons.find((b) => b.index === row.hwIndex)?.mapping.kind;
    return kind === "key" || kind === "macro";
  }

  function openLed(index: number) {
    selectedLed = index;
    selected = null;
  }

  async function takeOver(row: ButtonRow) {
    if (row.hwIndex == null) return;
    takingOver = true;
    try {
      const key = await api.takeOverButton(entry.vendor, entry.product, row.hwIndex);
      await app.loadHardware(entry);
      const d = def ?? app.addDevice(entry);
      // Hasta que se le asigne algo, la tecla que envía ahora (F13…) no debe
      // llegar al sistema: se deja desactivado y se guarda ya.
      if (!d.bindings.some((b) => sameKey(b.button, key))) {
        d.bindings.push({ mode: "disabled", button: key });
        await app.save();
      }
      selected = key;
      app.notify("ok", t("device.takenOver", { tag: row.tag }));
    } catch (e) {
      app.notify("error", String(e));
    } finally {
      takingOver = false;
    }
  }
</script>

<div class="page" class:with-editor={selectedRow || led}>
  <div class="main">
    <header>
      <div>
        {#if def}
          <input class="name" bind:value={def.name} aria-label={t("device.name")} placeholder={entry.name} />
        {:else}
          <h1>{definition?.name ?? entry.name}</h1>
        {/if}
        <p class="hint">
          <span class="code">{entry.key}</span>
          {#if !entry.connected}<span class="badge">{t("app.disconnected")}</span>{/if}
        </p>
      </div>
      <div class="actions">
        <button type="button" onclick={detect} disabled={detecting || !entry.connected}>
          <Icon name="target" size={16} />
          {detecting ? t("device.detecting") : t("device.detect")}
        </button>
        {#if def}
          <button type="button" class="ghost danger" onclick={() => app.removeDevice(entry.key)} title={t("device.deleteConfig")}>
            <Icon name="trash" size={16} />
          </button>
        {/if}
      </div>
    </header>

    {#if hardware}
      <HardwarePanel {entry} {hardware} />
    {/if}

    <div class="profiles" role="tablist" aria-label={t("device.general")}>
      <button type="button" role="tab" class:active={profileIndex === -1} aria-selected={profileIndex === -1}
        onclick={() => (profileIndex = -1)}>{t("device.general")}</button>
      {#each def?.apps ?? [] as p, i (i)}
        <button type="button" role="tab" class:active={profileIndex === i} aria-selected={profileIndex === i}
          onclick={() => (profileIndex = i)}>{p.name || t("device.unnamedProfile")}</button>
      {/each}
      <button type="button" class="ghost add" onclick={addProfile}>
        <Icon name="plus" size={14} /> {t("device.addProfile")}
      </button>
    </div>

    {#if profile}
      <section class="profile-head">
        <div class="row-fields">
          <label class="field">
            {t("device.profileName")}
            <input bind:value={profile.name} />
          </label>
          <button type="button" class="ghost danger" onclick={removeProfile}>
            <Icon name="trash" size={15} /> {t("device.deleteProfile")}
          </button>
        </div>
        <div class="field">
          {t("device.activeWhen")}
          <AppPicker bind:apps={profile.apps} />
        </div>
        {#if app.focus?.desktop === "gnome" && !app.focus.extension_active}
          <p class="warn">
            <Icon name="alert" size={15} /> {t("device.gnomeNeedsExtension")}
          </p>
        {/if}
        <p class="hint">{t("device.focusOnly")} {t("device.inherits")}</p>
      </section>
    {/if}

    {#if hasFigure}
      <DeviceFigure
        svg={figureSvg}
        image={definition?.image}
        {rows}
        leds={hardware?.leds ?? []}
        {selected}
        {selectedLed}
        assigned={isAssigned}
        describe={describeRow}
        onselect={open}
        onselectled={openLed}
      />
    {/if}

    <div class="layout">

      {#if rows.length === 0}
        <div class="empty">
          <p>{t("device.noButtons", { detect: t("device.detect") })}</p>
        </div>
      {:else}
        <ul class="buttons">
          {#each rows as row (row.key)}
            {@const own = row.emits ? ownFor(row.emits) : undefined}
            {@const inherited = row.emits && profile && !own ? generalFor(row.emits) : undefined}
            <li>
              <button type="button" class="row" class:firmware={!row.emits} class:selected={selected === row.key} onclick={() => open(row)}>
                <span class="keycap" class:lit={isAssigned(row)}>{row.tag}</span>
                <span class="what">
                  <span class="label">{row.label}</span>
                  {#if !row.emits}
                    <span class="summary muted">{t("device.firmware", { action: row.firmwareAction ?? "" })}</span>
                  {:else if own}
                    <span class="summary"><strong>{modeLabel(own.mode)}</strong> {describeBinding(own)}</span>
                  {:else if inherited}
                    <span class="summary muted">
                      {t("device.inherited", { mode: modeLabel(inherited.mode), text: describeBinding(inherited) })}
                    </span>
                  {:else}
                    <span class="summary muted">{t("device.unassigned")}</span>
                  {/if}
                </span>
                <span class="code">{row.emits ?? ""}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    {#if definition?.notes}
      <p class="hint note">{localized(definition.notes)}</p>
    {/if}
  </div>

  {#if led}
    <aside>
      {#key led.index}
        <LedEditor {entry} {led} onclose={() => (selectedLed = null)} />
      {/key}
    </aside>
  {:else if selectedRow}
    <aside>
      {#if selectedRow.emits && def}
        {#if !selectedRow.takenOver && selectedRow.hwIndex != null && instantKeyOf(selectedRow)}
          <div class="reprogram">
            <button type="button" class="primary" disabled={takingOver} onclick={() => takeOver(selectedRow)}>
              {takingOver ? t("firmware.takingOver") : t("firmware.takeOver")}
            </button>
            <span class="hint">{t("firmware.takeOverHint")}</span>
          </div>
        {/if}
        {#if selectedRow.takenOver}
          <TakenOverPanel
            {entry}
            row={selectedRow}
            conflict={rows.filter((r) => r.emits === selectedRow.emits).length > 1}
            onrestored={() => (selected = null)}
          />
        {/if}
        {#key `${profileIndex}:${selectedRow.key}`}
          <BindingEditor
            bindings={editing ?? def.bindings}
            inherited={profile ? generalFor(selectedRow.emits) : undefined}
            profileName={profile ? profile.name || t("device.thisProfile") : undefined}
            tag={selectedRow.tag !== selectedRow.label ? selectedRow.tag : undefined}
            instantKey={instantKeyOf(selectedRow)}
            button={selectedRow.emits}
            onclose={() => (selected = null)}
          />
        {/key}
      {:else}
        <section class="firmware-panel">
          <header>
            <div class="title">
              <span class="keycap">{selectedRow.tag}</span>
              <h2>{selectedRow.label}</h2>
            </div>
            <button type="button" class="ghost" onclick={() => (selected = null)} title={t("binding.close")}>
              <Icon name="x" />
            </button>
          </header>
          <p>{t("firmware.current", { action: selectedRow.firmwareAction ?? "" })}</p>
          {#if selectedRow.hwIndex != null && hardware}
            <p class="hint">{t("firmware.takeOverHint")}</p>
            <div>
              <button type="button" class="primary" disabled={takingOver} onclick={() => takeOver(selectedRow)}>
                {takingOver ? t("firmware.takingOver") : t("firmware.takeOver")}
              </button>
            </div>
          {:else}
            <p class="hint">{t("firmware.unsupported")}</p>
          {/if}
        </section>
      {/if}
    </aside>
  {/if}
</div>

<style>
  .page {
    display: grid;
    grid-template-columns: 1fr;
    height: 100%;
    min-height: 0;
  }

  .page.with-editor {
    grid-template-columns: minmax(340px, 1fr) minmax(400px, 460px);
  }

  .main,
  aside {
    overflow-y: auto;
    padding: 24px 28px;
    min-height: 0;
  }

  aside {
    border-left: 1px solid var(--line);
    background: var(--panel);
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: start;
    gap: 16px;
    margin-bottom: 20px;
  }

  .name {
    font-size: 22px;
    font-weight: 600;
    border-color: transparent;
    background: transparent;
    padding: 2px 4px;
    margin-left: -5px;
    width: 100%;
  }

  .name:hover,
  .name:focus {
    border-color: var(--line);
  }

  header .hint {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-top: 2px;
  }

  .badge {
    font-size: 12px;
    color: var(--muted);
    border: 1px solid var(--line);
    border-radius: 999px;
    padding: 0 8px;
  }

  .actions {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }

  .profiles {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-bottom: 16px;
    border-bottom: 1px solid var(--line);
  }

  .profiles button {
    border: 0;
    border-bottom: 2px solid transparent;
    border-radius: 0;
    background: transparent;
    color: var(--muted);
    padding: 8px 12px;
  }

  .profiles button.active {
    color: var(--text);
    border-bottom-color: var(--signal);
  }

  .profiles .add {
    margin-left: auto;
  }

  .profile-head {
    display: grid;
    gap: 12px;
    padding: 14px;
    margin-bottom: 16px;
    border: 1px solid var(--line);
    border-radius: var(--radius-m);
    background: var(--panel);
  }

  .row-fields {
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: 12px;
  }

  .warn {
    display: flex;
    gap: 8px;
    align-items: center;
    color: var(--signal);
  }

  .buttons {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 6px;
  }

  .row {
    width: 100%;
    display: grid;
    grid-template-columns: 110px 1fr auto;
    align-items: center;
    gap: 14px;
    padding: 10px 14px;
    background: var(--panel);
    border-radius: var(--radius-m);
    text-align: left;
  }

  .row .keycap {
    justify-self: start;
  }

  .row.selected {
    border-color: var(--signal);
  }

  .what {
    display: grid;
    min-width: 0;
  }

  .label {
    font-weight: 600;
  }

  .summary {
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .summary strong {
    color: var(--signal);
    font-weight: 600;
    margin-right: 4px;
  }

  .summary.muted {
    color: var(--faint);
  }

  .empty {
    padding: 20px;
    border: 1px dashed var(--line);
    border-radius: var(--radius-m);
    color: var(--muted);
    max-width: 60ch;
  }

  .note {
    margin-top: 20px;
    max-width: 70ch;
  }

  .layout {
    display: grid;
    gap: 20px;
  }

  .row.firmware .keycap {
    border-style: dashed;
    color: var(--muted);
  }

  .reprogram {
    display: grid;
    gap: 8px;
    justify-items: start;
    margin-bottom: 18px;
  }

  .firmware-panel {
    display: grid;
    gap: 14px;
    align-content: start;
  }

  .firmware-panel header {
    margin: 0;
  }

  .firmware-panel .keycap {
    font-size: 15px;
    padding: 6px 12px 8px;
  }

  .firmware-panel .title {
    display: flex;
    gap: 12px;
    align-items: center;
  }


</style>
