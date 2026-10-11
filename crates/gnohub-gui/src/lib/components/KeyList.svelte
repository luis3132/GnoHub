<script lang="ts">
  import { t } from "../i18n/index.svelte";
  import KeyInput from "./KeyInput.svelte";
  import Icon from "./Icon.svelte";

  let { keys = $bindable(), label }: { keys: string[]; label?: string } = $props();
</script>

<div class="key-list" role="group" aria-label={label}>
  {#each keys as _, i (i)}
    {#if i > 0}<span class="plus">+</span>{/if}
    <span class="item">
      <KeyInput bind:value={keys[i]} />
      {#if keys.length > 1}
        <button type="button" class="ghost remove" onclick={() => keys.splice(i, 1)} title={t("key.removeKey")}>
          <Icon name="x" size={14} />
        </button>
      {/if}
    </span>
  {/each}
  <button type="button" class="ghost add" onclick={() => keys.push("KEY_A")}>
    <Icon name="plus" size={14} />
    {keys.length ? t("key.keyShort") : t("key.addKey")}
  </button>
</div>

<style>
  .key-list {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
  }

  .item {
    display: inline-flex;
    align-items: center;
  }

  .plus {
    color: var(--faint);
  }

  .remove,
  .add {
    padding: 4px 6px;
  }
</style>
