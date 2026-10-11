<script lang="ts">
  import { emptyMacro } from "../actions";
  import { t } from "../i18n/index.svelte";
  import { app } from "../state.svelte";
  import type { MacroRef } from "../types";
  import StepsEditor from "./StepsEditor.svelte";

  let {
    value = $bindable(),
    label,
    optional = false,
  }: { value: MacroRef | undefined; label: string; optional?: boolean } = $props();

  const OWN = "\u0000own";
  const NONE = "\u0000none";

  const names = $derived(Object.keys(app.config.macros));
  const selected = $derived(value === undefined ? NONE : typeof value === "string" ? value : OWN);
  const missing = $derived(typeof value === "string" && !app.config.macros[value]);

  function choose(choice: string) {
    if (choice === NONE) value = undefined;
    else if (choice === OWN) {
      // Partir de la macro elegida hasta ahora, si la había, para no empezar de cero.
      const named = typeof value === "string" ? app.config.macros[value] : undefined;
      value = named ? structuredClone($state.snapshot(named)) : emptyMacro();
    } else value = choice;
  }

  function saveAsNamed() {
    if (value === undefined || typeof value === "string") return;
    const name = app.newMacroName();
    app.config.macros[name] = $state.snapshot(value);
    value = name;
  }
</script>

<div class="slot">
  <div class="head">
    <label class="field">
      {label}
      <select value={selected} onchange={(e) => choose(e.currentTarget.value)} aria-invalid={missing}>
        {#if optional}<option value={NONE}>{t("slot.nothing")}</option>{/if}
        {#if missing}<option value={value}>{t("slot.missing", { name: String(value) })}</option>{/if}
        {#if names.length > 0}
          <optgroup label={t("slot.saved")}>
            {#each names as name (name)}
              <option value={name}>{name}</option>
            {/each}
          </optgroup>
        {/if}
        <option value={OWN}>{t("slot.own")}</option>
      </select>
    </label>
    {#if value !== undefined && typeof value !== "string" && value.actions.length > 0}
      <button type="button" class="ghost" onclick={saveAsNamed}>{t("slot.saveAsMacro")}</button>
    {:else if typeof value === "string" && !missing}
      <button type="button" class="ghost" onclick={() => (app.view = { kind: "macros", name: value as string })}>
        {t("slot.editMacro")}
      </button>
    {/if}
  </div>
  {#if value !== undefined && typeof value !== "string"}
    <div class="own">
      <StepsEditor bind:macro={value} />
    </div>
  {/if}
</div>

<style>
  .slot {
    display: grid;
    gap: 10px;
  }

  .head {
    display: flex;
    align-items: end;
    gap: 8px;
  }

  .head .field {
    flex: 1;
  }

  .own {
    padding-left: 12px;
    border-left: 2px solid var(--line);
  }
</style>
