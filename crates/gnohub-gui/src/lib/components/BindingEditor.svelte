<script lang="ts">
  import { describeBinding, MODES, modeHint, modeLabel, newBinding } from "../actions";
  import { t } from "../i18n/index.svelte";
  import { isWheel, keyLabel, sameKey } from "../keys";
  import { app } from "../state.svelte";
  import type { BindingDef, Mode } from "../types";
  import Icon from "./Icon.svelte";
  import KeyList from "./KeyList.svelte";
  import MacroSlot from "./MacroSlot.svelte";
  import NumberInput from "./NumberInput.svelte";

  let {
    bindings,
    inherited,
    profileName,
    tag,
    instantKey = false,
    button,
    onclose,
  }: {
    /** Lista que se edita: la general del dispositivo o la de un perfil. */
    bindings: BindingDef[];
    /** En un perfil, la asignación general que se hereda si este no la cambia. */
    inherited?: BindingDef;
    /** Nombre del perfil de aplicación, o `undefined` en el general. */
    profileName?: string;
    /** Nombre del botón en el periférico (G4), si su definición lo da. */
    tag?: string;
    /** El ratón envía este botón como tecla pulsada y soltada al instante. */
    instantKey?: boolean;
    button: string;
    onclose: () => void;
  } = $props();

  const index = $derived(bindings.findIndex((b) => sameKey(b.button, button)));
  const binding = $derived(index >= 0 ? bindings[index] : undefined);
  const modes = $derived(MODES.filter((m) => m !== "native" || profileName !== undefined));
  // Si no se indica pausa entre repeticiones, el daemon usa el tiempo entre
  // pasos de la macro que se repite.
  const defaultGap = $derived.by(() => {
    switch (binding?.mode) {
      case "press":
      case "while_held":
      case "toggle":
        return app.stepDelayOf(binding.macro);
      case "sequence":
        return app.stepDelayOf(binding.while_held);
      case "tap_hold":
        return app.stepDelayOf(binding.hold);
      default:
        return 10;
    }
  });
  const firstMacro = $derived(Object.keys(app.config.macros)[0]);

  function setMode(mode: Mode) {
    if (binding?.mode === mode) return;
    const next = newBinding(mode, binding?.button ?? button, binding, firstMacro);
    if (index >= 0) bindings[index] = next;
    else bindings.push(next);
  }

  function remove() {
    if (index >= 0) bindings.splice(index, 1);
  }

</script>

<section class="editor" aria-label={keyLabel(button)}>
  <header>
    <div class="title">
      <span class="keycap lit">{tag ?? keyLabel(button)}</span>
      <div>
        <h2>{tag ? `${keyLabel(button)} (${tag})` : keyLabel(button)}</h2>
        <span class="code">{button}</span>
      </div>
    </div>
    <button type="button" class="ghost" onclick={onclose} title={t("binding.close")}>
      <Icon name="x" />
    </button>
  </header>

  <fieldset class="modes">
    <legend>{t("binding.whatItDoes")}</legend>
    {#each modes as m (m)}
      <label class="mode" class:active={binding?.mode === m}>
        <input type="radio" name="mode" value={m} checked={binding?.mode === m} onchange={() => setMode(m)} />
        <strong>{modeLabel(m)}</strong>
        <span>{modeHint(m)}</span>
      </label>
    {/each}
  </fieldset>

  {#if instantKey}
    <p class="warn-text">{t("firmware.instantKey", { key: button })}</p>
  {/if}

  {#if isWheel(button)}
    <p class="hint">
      {t("binding.wheelHint")}
      {#if button.toUpperCase() === "WHEEL_UP" || button.toUpperCase() === "WHEEL_DOWN"}
        {t("binding.wheelScrollHint")}
      {/if}
    </p>
  {/if}

  {#if binding}
    <div class="body">
      {#if binding.mode === "disabled"}
        <p class="hint">{t("binding.disabledHint")}</p>
      {:else if binding.mode === "native"}
        <p class="hint">{t("binding.nativeHint", { profile: profileName ?? "" })}</p>
      {:else if binding.mode === "remap"}
        <div class="field-block">
          <span class="hint">{t("binding.remapHint")}</span>
          <KeyList bind:keys={binding.to} label={t("binding.keys")} />
        </div>
      {:else if binding.mode === "press"}
        <MacroSlot bind:value={binding.macro} label={t("binding.macro")} />
        <div class="row">
          <label class="field">
            {t("binding.times")}
            <NumberInput bind:value={binding.repeat} min={1} />
          </label>
          {#if binding.repeat > 1}
            <label class="field">
              {t("binding.repeatGap")}
              <NumberInput bind:value={binding.repeat_delay} min={1} optional placeholder={String(defaultGap)} />
            </label>
          {/if}
        </div>
      {:else if binding.mode === "while_held" || binding.mode === "toggle"}
        <MacroSlot bind:value={binding.macro} label={t("binding.repeatedMacro")} />
        <label class="field narrow">
          {t("binding.repeatGap")}
          <NumberInput bind:value={binding.repeat_delay} min={1} optional placeholder={String(defaultGap)} />
        </label>
      {:else if binding.mode === "sequence"}
        <MacroSlot bind:value={binding.on_press} label={t("binding.onPress")} optional />
        <MacroSlot bind:value={binding.while_held} label={t("binding.whileHeld")} optional />
        {#if binding.while_held !== undefined}
          <label class="field narrow">
            {t("binding.repeatGap")}
            <NumberInput bind:value={binding.repeat_delay} min={1} optional placeholder={String(defaultGap)} />
          </label>
        {/if}
        <MacroSlot bind:value={binding.on_release} label={t("binding.onRelease")} optional />
      {:else if binding.mode === "tap_hold"}
        <MacroSlot bind:value={binding.tap} label={t("binding.tap")} />
        <MacroSlot bind:value={binding.hold} label={t("binding.hold")} />
        <div class="row">
          <label class="field">
            {t("binding.holdFor")}
            <NumberInput bind:value={binding.hold_time} min={1} />
          </label>
          <label class="check">
            <input type="checkbox" bind:checked={binding.hold_repeat} />
            {t("binding.repeatWhileHeld")}
          </label>
          {#if binding.hold_repeat}
            <label class="field">
              {t("binding.repeatGap")}
              <NumberInput bind:value={binding.repeat_delay} min={1} optional placeholder={String(defaultGap)} />
            </label>
          {/if}
        </div>
      {/if}
    </div>

    <footer>
      <button type="button" class="ghost danger" onclick={remove}>
        <Icon name="trash" size={15} />
        {profileName !== undefined ? t("binding.useGeneral") : t("binding.remove")}
      </button>
    </footer>
  {:else}
    {#if profileName !== undefined && inherited}
      <p class="hint unassigned">
        {t("binding.inheritsInProfile", {
          profile: profileName,
          mode: modeLabel(inherited.mode),
          text: describeBinding(inherited),
        })}
      </p>
    {:else}
      <p class="hint unassigned">{t("binding.unassigned")}</p>
    {/if}
  {/if}
</section>

<style>
  .warn-text {
    color: var(--signal);
  }

  .editor {
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

  .title .keycap {
    font-size: 15px;
    padding: 6px 12px 8px;
  }

  .modes {
    border: 0;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 6px;
  }

  legend {
    color: var(--muted);
    font-size: 13px;
    margin-bottom: 6px;
  }

  .mode {
    display: grid;
    gap: 2px;
    padding: 10px 12px;
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

  .mode span {
    color: var(--muted);
    font-size: 12.5px;
  }

  .mode.active {
    border-color: var(--signal);
    background: var(--signal-soft);
  }

  .mode.active strong {
    color: var(--signal);
  }

  .body {
    display: grid;
    gap: 18px;
  }

  .field-block {
    display: grid;
    gap: 8px;
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    gap: 16px;
  }

  .narrow {
    max-width: 240px;
  }

  .check {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding-bottom: 6px;
  }

  footer {
    border-top: 1px solid var(--line);
    padding-top: 12px;
  }

  .unassigned {
    padding: 14px;
    border: 1px dashed var(--line);
    border-radius: var(--radius-m);
  }
</style>
