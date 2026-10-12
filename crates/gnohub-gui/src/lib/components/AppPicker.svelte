<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "../i18n/index.svelte";
  import { app } from "../state.svelte";
  import Icon from "./Icon.svelte";

  let { apps = $bindable() }: { apps: string[] } = $props();

  let query = $state("");
  let fileInput = $state<HTMLInputElement>();

  onMount(() => {
    app.loadApps().catch((e) => app.notify("error", String(e)));
    // Solo en la vista previa del navegador: #buscar=texto rellena el buscador.
    if (import.meta.env.DEV) query = decodeURIComponent(/buscar=([^&]+)/.exec(location.hash)?.[1] ?? "");
  });

  interface Candidate {
    id: string;
    name: string;
    /** Clase de ventana, que otros entornos usan en vez del id. */
    wmClass?: string | null;
    source: "steam" | "lutris" | "heroic" | "app";
  }

  // Juegos primero: es lo que más se busca para un perfil.
  const candidates = $derived<Candidate[]>([
    ...app.games.map((g) => ({ id: g.id, name: g.name, source: g.source })),
    ...app.installedApps.map((a) => ({ id: a.id, name: a.name, wmClass: a.wm_class, source: "app" as const })),
  ]);

  function nameOf(id: string): string | undefined {
    const lower = id.toLowerCase();
    const found = candidates.find((c) => c.id.toLowerCase() === lower || c.wmClass?.toLowerCase() === lower);
    if (found) return found.name;
    if (lower.startsWith("exe:")) return `${id.slice(4)} (${t("picker.executable")})`;
    return undefined;
  }

  const matches = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return [];
    return candidates
      .filter((c) => c.name.toLowerCase().includes(q) || c.id.toLowerCase().includes(q))
      .filter((c) => !apps.includes(c.id))
      .slice(0, 10);
  });

  function add(id: string, wmClass?: string | null) {
    if (!apps.includes(id)) apps.push(id);
    // Otros entornos identifican la ventana por su clase, que puede no coincidir con el id.
    if (wmClass && !apps.some((a) => a.toLowerCase() === wmClass.toLowerCase())) apps.push(wmClass);
    query = "";
  }

  function addTyped() {
    const first = matches[0];
    if (first) add(first.id, first.wmClass);
    else if (query.trim()) add(query.trim());
  }

  /** Solo hace falta el nombre del archivo: se compara con el del proceso de la ventana. */
  function pickExecutable(e: Event & { currentTarget: HTMLInputElement }) {
    const file = e.currentTarget.files?.[0];
    if (file) add(`exe:${file.name.toLowerCase()}`);
    e.currentTarget.value = "";
  }
</script>

<div class="picker">
  <ul class="chips">
    {#each apps as id, i (id)}
      <li>
        <span>{nameOf(id) ?? id}</span>
        {#if nameOf(id) && !id.toLowerCase().startsWith("exe:")}<span class="code">{id}</span>{/if}
        <button type="button" class="ghost" onclick={() => apps.splice(i, 1)} title={t("picker.remove", { id })}>
          <Icon name="x" size={13} />
        </button>
      </li>
    {/each}
  </ul>
  <div class="row">
    <div class="search">
      <input
        bind:value={query}
        placeholder={t("picker.placeholder")}
        aria-label={t("picker.add")}
        onkeydown={(e) => e.key === "Enter" && addTyped()}
      />
      {#if matches.length > 0}
        <ul class="results" role="listbox">
          {#each matches as c (c.id)}
            <li>
              <button type="button" onclick={() => add(c.id, c.wmClass)}>
                <span class="source {c.source}">{t(`picker.sources.${c.source}`)}</span>
                {c.name}
                <span class="code">{c.id}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
    <button type="button" onclick={() => fileInput?.click()} title={t("picker.chooseExeHint")}>
      {t("picker.chooseExe")}
    </button>
    <input bind:this={fileInput} type="file" class="hidden" onchange={pickExecutable} tabindex="-1" aria-hidden="true" />
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

  .row {
    display: flex;
    gap: 8px;
    align-items: start;
    flex-wrap: wrap;
  }

  .search {
    position: relative;
    flex: 1;
    min-width: 260px;
    max-width: 460px;
  }

  .hidden {
    display: none;
  }

  .source {
    font-size: 11px;
    padding: 0 6px;
    border-radius: 999px;
    border: 1px solid var(--line);
    color: var(--muted);
  }

  .source.steam,
  .source.lutris,
  .source.heroic {
    color: var(--signal);
    border-color: var(--signal);
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
    gap: 8px;
    border: 0;
    background: transparent;
  }

  .results button:hover {
    background: var(--panel);
  }
</style>
