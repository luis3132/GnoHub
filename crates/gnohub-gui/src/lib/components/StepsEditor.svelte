<script lang="ts">
  import { ACTION_KINDS, actionKindLabel, convertAction, estimateMs, kindOf, type ActionKind } from "../actions";
  import { t } from "../i18n/index.svelte";
  import { codeToKey, mouseToKey } from "../keys";
  import type { Action, MacroDef } from "../types";
  import Icon from "./Icon.svelte";
  import KeyInput from "./KeyInput.svelte";
  import KeyList from "./KeyList.svelte";
  import NumberInput from "./NumberInput.svelte";

  let { macro = $bindable() }: { macro: MacroDef } = $props();

  const actions = $derived(macro.actions);

  let recording = $state(false);
  let recordDelays = $state(true);
  let stopButton = $state<HTMLButtonElement>();

  const total = $derived(estimateMs(actions, macro.step_delay));

  function add(kind: ActionKind) {
    actions.push(convertAction({ tap: "KEY_A" }, kind));
  }

  function move(i: number, by: number) {
    const j = i + by;
    if (j < 0 || j >= actions.length) return;
    [actions[i], actions[j]] = [actions[j], actions[i]];
  }

  function setKind(i: number, kind: ActionKind) {
    actions[i] = convertAction(actions[i], kind);
  }

  /** Une `down X` + `up X` consecutivos en `tap X` cuando no hay retardo entre ellos. */
  function compact(list: Action[]): Action[] {
    const out: Action[] = [];
    for (const a of list) {
      const prev = out[out.length - 1];
      if (prev && "down" in prev && "up" in a && prev.down === a.up) {
        out[out.length - 1] = { tap: a.up };
      } else {
        out.push(a);
      }
    }
    return out;
  }

  // Grabación: cada tecla o clic dentro de la ventana se añade como paso, con
  // el tiempo real transcurrido entre eventos (mínimo 1 ms).
  $effect(() => {
    if (!recording) return;
    const recorded: Action[] = [];
    let last = performance.now();

    const push = (action: Action) => {
      const now = performance.now();
      if (recordDelays && recorded.length > 0) {
        recorded.push({ delay: Math.max(1, Math.round(now - last)) });
      }
      last = now;
      recorded.push(action);
    };
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.repeat) return;
      const key = codeToKey(e.code);
      if (key) push(e.type === "keydown" ? { down: key } : { up: key });
    };
    const onMouse = (e: MouseEvent) => {
      if (stopButton?.contains(e.target as Node)) return;
      e.preventDefault();
      e.stopPropagation();
      const key = mouseToKey(e.button);
      if (key) push(e.type === "mousedown" ? { down: key } : { up: key });
    };
    const block = (e: Event) => e.preventDefault();

    const events = [
      ["keydown", onKey],
      ["keyup", onKey],
      ["mousedown", onMouse],
      ["mouseup", onMouse],
      ["contextmenu", block],
    ] as const;
    const timer = setTimeout(() => events.forEach(([t, f]) => window.addEventListener(t, f as EventListener, true)));
    return () => {
      clearTimeout(timer);
      events.forEach(([t, f]) => window.removeEventListener(t, f as EventListener, true));
      if (recorded.length) actions.push(...(recordDelays ? recorded : compact(recorded)));
    };
  });
</script>

<div class="steps">
  <div class="timing">
    <label class="delay">
      {t("steps.stepDelay")}
      <NumberInput bind:value={macro.step_delay} min={1} label={t("steps.stepDelayLabel")} />
      ms
    </label>
    {#if actions.length > 0}
      <span class="hint">{t("steps.duration", { ms: total })}</span>
    {/if}
  </div>

  {#if actions.length === 0 && !recording}
    <p class="empty">{t("steps.empty")}</p>
  {/if}

  <ol>
    {#each actions as action, i (i)}
      {@const kind = kindOf(action)}
      <li class:pause={kind === "delay"}>
        <span class="index">{i + 1}</span>
        <select value={kind} onchange={(e) => setKind(i, e.currentTarget.value as ActionKind)} aria-label={t("steps.stepType")}>
          {#each ACTION_KINDS as k (k)}
            <option value={k}>{actionKindLabel(k)}</option>
          {/each}
        </select>

        <span class="value">
          {#if "tap" in action}
            <KeyInput bind:value={action.tap} />
          {:else if "down" in action}
            <KeyInput bind:value={action.down} />
          {:else if "up" in action}
            <KeyInput bind:value={action.up} />
          {:else if "chord" in action}
            <KeyList bind:keys={action.chord} label={t("key.combination")} />
          {:else if "delay" in action}
            <NumberInput bind:value={action.delay} min={1} label={t("steps.milliseconds")} />
            <span class="hint">{t("steps.ms")}</span>
          {:else if "move" in action}
            <NumberInput bind:value={action.move[0]} label={t("steps.moveX")} />
            <NumberInput bind:value={action.move[1]} label={t("steps.moveY")} />
            <span class="hint">{t("steps.moveUnit")}</span>
          {:else if "scroll" in action}
            <NumberInput bind:value={action.scroll} label={t("steps.scrollSteps")} />
            <span class="hint">{t("steps.scrollHint")}</span>
          {:else if "command" in action}
            <input class="command" bind:value={action.command} placeholder="notify-send 'Hola'" aria-label={t("steps.command")} />
          {/if}
        </span>

        <span class="row-actions">
          <button type="button" class="ghost" onclick={() => move(i, -1)} disabled={i === 0} title={t("steps.up")}>
            <Icon name="up" size={15} />
          </button>
          <button type="button" class="ghost" onclick={() => move(i, 1)} disabled={i === actions.length - 1} title={t("steps.down")}>
            <Icon name="down" size={15} />
          </button>
          <button type="button" class="ghost" onclick={() => actions.splice(i, 1)} title={t("steps.remove")}>
            <Icon name="trash" size={15} />
          </button>
        </span>
      </li>
    {/each}
  </ol>

  {#if recording}
    <div class="recording" role="status">
      <span class="dot"></span>
      {t("steps.recording")}
      <button type="button" bind:this={stopButton} onclick={() => (recording = false)}>
        <Icon name="stop" size={14} /> {t("steps.stop")}
      </button>
    </div>
  {:else}
    <div class="toolbar">
      {#each ACTION_KINDS.filter((k) => k !== "up") as k (k)}
        <button type="button" onclick={() => add(k)}>
          <Icon name="plus" size={14} />
          {actionKindLabel(k)}
        </button>
      {/each}
      <span class="spacer"></span>
      <label class="check">
        <input type="checkbox" bind:checked={recordDelays} />
        {t("steps.recordDelays")}
      </label>
      <button type="button" class="record" onclick={() => (recording = true)}>
        <Icon name="record" size={14} /> {t("steps.record")}
      </button>
    </div>
  {/if}

</div>

<style>
  .steps {
    display: grid;
    gap: 10px;
  }

  .timing {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }

  .delay {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
  }

  .delay :global(input) {
    width: 72px;
  }

  ol {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 4px;
  }

  li {
    display: grid;
    grid-template-columns: 26px 140px 1fr auto;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border: 1px solid var(--line);
    border-radius: var(--radius-s);
    background: var(--panel);
  }

  li.pause {
    background: transparent;
    border-style: dashed;
  }

  .index {
    color: var(--faint);
    font-variant-numeric: tabular-nums;
    text-align: right;
  }

  .value {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .command {
    flex: 1;
    font-family: var(--mono);
    font-size: 12px;
  }

  .row-actions {
    display: flex;
  }

  .row-actions button {
    padding: 4px;
  }

  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }

  .toolbar button {
    padding: 4px 10px;
  }

  .spacer {
    flex: 1;
  }

  .check {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
  }

  .record {
    color: var(--danger);
  }

  .recording {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border: 1px solid var(--danger);
    border-radius: var(--radius-s);
    color: var(--danger);
  }

  .recording button {
    margin-left: auto;
  }

  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--danger);
    animation: blink 1s steps(2) infinite;
  }

  @keyframes blink {
    50% {
      opacity: 0.2;
    }
  }

  .empty {
    color: var(--muted);
    padding: 12px;
    border: 1px dashed var(--line);
    border-radius: var(--radius-s);
  }
</style>
