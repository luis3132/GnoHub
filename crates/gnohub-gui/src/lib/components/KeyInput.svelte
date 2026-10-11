<script lang="ts">
  import { app } from "../state.svelte";
  import { t } from "../i18n/index.svelte";
  import { codeToKey, keyLabel, mouseToKey } from "../keys";
  import Icon from "./Icon.svelte";

  let { value = $bindable(), label }: { value: string; label?: string } = $props();
  const name = $derived(label ?? t("key.key"));

  let capturing = $state(false);
  let editing = $state(false);
  let draft = $state("");

  const known = $derived(app.keys.length === 0 || app.keys.includes(value.toUpperCase()));

  // Mientras se captura, la siguiente tecla o clic se convierte en el valor.
  $effect(() => {
    if (!capturing) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const key = codeToKey(e.code);
      if (key) value = key;
      capturing = false;
    };
    const onMouse = (e: MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const key = mouseToKey(e.button);
      if (key) value = key;
      capturing = false;
    };
    const block = (e: Event) => e.preventDefault();
    // Se espera al siguiente ciclo para no capturar el propio clic que activó la captura.
    const timer = setTimeout(() => {
      window.addEventListener("keydown", onKey, true);
      window.addEventListener("mousedown", onMouse, true);
      window.addEventListener("contextmenu", block, true);
    });
    return () => {
      clearTimeout(timer);
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("mousedown", onMouse, true);
      window.removeEventListener("contextmenu", block, true);
    };
  });

  function startEdit() {
    draft = value;
    editing = true;
  }

  function commit() {
    const name = draft.trim().toUpperCase();
    if (name) value = name;
    editing = false;
  }
</script>

<span class="key-input">
  {#if editing}
    <!-- svelte-ignore a11y_autofocus -->
    <input
      list="gnohub-keys"
      bind:value={draft}
      onblur={commit}
      onkeydown={(e) => {
        if (e.key === "Enter") commit();
        if (e.key === "Escape") editing = false;
      }}
      aria-label={name}
      autofocus
    />
  {:else}
    <button
      type="button"
      class="keycap"
      class:capturing
      class:invalid={!known}
      onclick={() => (capturing = !capturing)}
      title={capturing ? t("key.captureTitle") : t("key.clickToCapture", { name: value })}
      aria-label={t("key.captureLabel", { label: name, key: keyLabel(value) })}
    >
      {capturing ? t("key.pressAKey") : keyLabel(value)}
    </button>
    <button type="button" class="ghost edit" onclick={startEdit} title={t("key.typeName")}>
      <Icon name="keycap" size={15} />
    </button>
  {/if}
</span>

<style>
  .key-input {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }

  .keycap {
    cursor: pointer;
  }

  .keycap.capturing {
    border-color: var(--signal);
    color: var(--signal);
  }

  .keycap.invalid {
    border-color: var(--danger);
    color: var(--danger);
  }

  .edit {
    padding: 4px;
  }

  input {
    width: 170px;
    font-family: var(--mono);
    font-size: 12px;
  }
</style>
