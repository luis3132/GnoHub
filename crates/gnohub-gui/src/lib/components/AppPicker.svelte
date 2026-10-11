<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "../i18n/index.svelte";
  import { app } from "../state.svelte";
  import Icon from "./Icon.svelte";

  let { apps = $bindable() }: { apps: string[] } = $props();

  let query = $state("");

  onMount(() => {
    app.loadApps().catch((e) => app.notify("error", String(e)));
  });

  const nameOf = (id: string) =>
    app.installedApps.find((a) => a.id.toLowerCase() === id.toLowerCase() || a.wm_class?.toLowerCase() === id.toLowerCase())
      ?.name;

  const matches = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return [];
    return app.installedApps
      .filter((a) => a.name.toLowerCase().includes(q) || a.id.toLowerCase().includes(q))
      .filter((a) => !apps.includes(a.id))
      .slice(0, 8);
  });

  function add(id: string, wmClass?: string | null) {
    if (!apps.includes(id)) apps.push(id);
    // Otros entornos identifican la ventana por su clase, que puede no coincidir con el id.
    if (wmClass && !apps.some((a) => a.toLowerCase() === wmClass.toLowerCase())) apps.push(wmClass);
    query = "";
  }

  function addTyped() {
    const exact = matches[0];
    if (exact) add(exact.id, exact.wm_class);
    else if (query.trim()) add(query.trim());
  }
</script>

<div class="picker">
  <ul class="chips">
    {#each apps as id, i (id)}
      <li>
        <span>{nameOf(id) ?? id}</span>
        {#if nameOf(id)}<span class="code">{id}</span>{/if}
        <button type="button" class="ghost" onclick={() => apps.splice(i, 1)} title={t("picker.remove", { id })}>
          <Icon name="x" size={13} />
        </button>
      </li>
    {/each}
  </ul>
  <div class="search">
    <input
      bind:value={query}
      placeholder={t("picker.placeholder")}
      aria-label={t("picker.add")}
      onkeydown={(e) => e.key === "Enter" && addTyped()}
    />
    {#if matches.length > 0}
      <ul class="results" role="listbox">
        {#each matches as a (a.id)}
          <li>
            <button type="button" onclick={() => add(a.id, a.wm_class)}>
              {a.name} <span class="code">{a.id}</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>

<style>
  .picker {
    display: grid;
    gap: 8px;
  }

  .chips {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chips li {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 4px 2px 10px;
    border: 1px solid var(--line);
    border-radius: 999px;
    background: var(--panel-2);
  }

  .chips button {
    padding: 2px;
  }

  .search {
    position: relative;
    max-width: 460px;
  }

  .search input {
    width: 100%;
  }

  .results {
    position: absolute;
    z-index: 5;
    left: 0;
    right: 0;
    top: calc(100% + 4px);
    list-style: none;
    margin: 0;
    padding: 4px;
    background: var(--panel-2);
    border: 1px solid var(--line);
    border-radius: var(--radius-m);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
  }

  .results button {
    width: 100%;
    justify-content: flex-start;
    border: 0;
    background: transparent;
  }

  .results button:hover {
    background: var(--panel);
  }
</style>
