<script lang="ts">
  import { emptyMacro, estimateMs } from "../actions";
  import { t } from "../i18n/index.svelte";
  import { app } from "../state.svelte";
  import Icon from "./Icon.svelte";
  import StepsEditor from "./StepsEditor.svelte";

  let { initial }: { initial?: string } = $props();

  const names = $derived(Object.keys(app.config.macros));
  let selected = $state<string | undefined>(undefined);
  let renameDraft = $state("");

  $effect(() => {
    if (initial && app.config.macros[initial]) selected = initial;
  });

  $effect(() => {
    if (!selected || !app.config.macros[selected]) selected = names[0];
  });

  $effect(() => {
    renameDraft = selected ?? "";
  });

  const current = $derived(selected ? app.config.macros[selected] : undefined);

  function create() {
    const name = app.newMacroName();
    app.config.macros[name] = emptyMacro();
    selected = name;
  }

  function rename() {
    const to = renameDraft.trim();
    if (!selected || to === selected) return;
    if (!/^[\w-]+$/.test(to)) {
      app.notify("error", t("macros.badName"));
      renameDraft = selected;
      return;
    }
    if (app.renameMacro(selected, to)) selected = to;
    else {
      app.notify("error", t("macros.exists", { name: to }));
      renameDraft = selected;
    }
  }

  function remove(name: string) {
    const uses = app.macroUsage(name);
    if (uses > 0) {
      app.notify("error", t("macros.inUse", { name, count: uses }));
      return;
    }
    delete app.config.macros[name];
  }

</script>

<div class="page">
  <nav aria-label={t("macros.title")}>
    <div class="nav-head">
      <h2>{t("macros.title")}</h2>
      <button type="button" onclick={create}><Icon name="plus" size={14} /> {t("macros.new")}</button>
    </div>
    {#if names.length === 0}
      <p class="hint">{t("macros.empty")}</p>
    {/if}
    <ul>
      {#each names as name (name)}
        {@const m = app.config.macros[name]}
        {@const uses = app.macroUsage(name)}
        <li>
          <button type="button" class="item" class:selected={selected === name} onclick={() => (selected = name)}>
            <span class="item-name">{name}</span>
            <span class="hint">
              {t("macros.summary", { steps: m.actions.length, ms: estimateMs(m.actions, m.step_delay) })}{uses === 1
                ? t("macros.usedInOne")
                : uses > 1
                  ? t("macros.usedIn", { count: uses })
                  : ""}
            </span>
          </button>
        </li>
      {/each}
    </ul>
  </nav>

  <section class="editor">
    {#if current && selected}
      <header>
        <input
          class="name"
          bind:value={renameDraft}
          onblur={rename}
          onkeydown={(e) => e.key === "Enter" && e.currentTarget.blur()}
          aria-label={t("macros.name")}
        />
        <button type="button" class="ghost danger" onclick={() => selected && remove(selected)}>
          <Icon name="trash" size={15} /> {t("macros.delete")}
        </button>
      </header>
      <StepsEditor bind:macro={app.config.macros[selected]} />
    {:else}
      <div class="empty">
        <p>{t("macros.emptyEditor")}</p>
        <button type="button" class="primary" onclick={create}><Icon name="plus" size={14} /> {t("macros.newMacro")}</button>
      </div>
    {/if}
  </section>
</div>

<style>
  .page {
    display: grid;
    grid-template-columns: 280px 1fr;
    height: 100%;
    min-height: 0;
  }

  nav {
    border-right: 1px solid var(--line);
    padding: 24px 16px;
    overflow-y: auto;
    display: grid;
    align-content: start;
    gap: 12px;
  }

  .nav-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
  }

  .item {
    width: 100%;
    display: grid;
    justify-items: start;
    text-align: left;
    gap: 0;
    border-color: transparent;
    background: transparent;
    padding: 8px 10px;
  }

  .item.selected {
    background: var(--panel);
    border-color: var(--line);
  }

  .item.selected .item-name {
    color: var(--signal);
  }

  .item-name {
    font-weight: 600;
  }

  .editor {
    padding: 24px 28px;
    overflow-y: auto;
    display: grid;
    align-content: start;
    gap: 18px;
  }

  header {
    display: flex;
    justify-content: space-between;
    gap: 12px;
  }

  .name {
    font-size: 22px;
    font-weight: 600;
    border-color: transparent;
    background: transparent;
    padding: 2px 4px;
    margin-left: -5px;
    flex: 1;
  }

  .name:hover,
  .name:focus {
    border-color: var(--line);
  }

  .empty {
    display: grid;
    gap: 12px;
    justify-items: start;
    padding: 24px;
    border: 1px dashed var(--line);
    border-radius: var(--radius-m);
    color: var(--muted);
  }
</style>
