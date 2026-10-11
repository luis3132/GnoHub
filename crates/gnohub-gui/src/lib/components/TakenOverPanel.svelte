<script lang="ts">
  // Botón que GnoHub reprogramó en el ratón (envía F13–F24): permite
  // devolverle su función original y quita sus asignaciones.

  import { api } from "../api";
  import { describeMapping, RESTORABLE } from "../actions";
  import { t } from "../i18n/index.svelte";
  import { sameKey } from "../keys";
  import type { ButtonRow } from "../rows";
  import { app, type DeviceEntry } from "../state.svelte";

  let {
    entry,
    row,
    conflict = false,
    onrestored,
  }: { entry: DeviceEntry; row: ButtonRow; conflict?: boolean; onrestored: () => void } = $props();

  let busy = $state(false);
  let choice = $state(0);

  async function restore() {
    if (row.hwIndex == null || !row.emits) return;
    busy = true;
    try {
      await api.restoreButton(entry.vendor, entry.product, row.hwIndex, row.hasBackup ? undefined : RESTORABLE[choice]);
      // El botón deja de enviar su tecla: sus asignaciones ya no sirven.
      const def = app.deviceDef(entry.key);
      if (def) {
        const keep = (list: { button: string }[]) => list.filter((b) => !sameKey(b.button, row.emits!));
        def.bindings = keep(def.bindings) as typeof def.bindings;
        for (const profile of def.apps ?? []) profile.bindings = keep(profile.bindings) as typeof profile.bindings;
        await app.save();
      }
      await app.loadHardware(entry);
      app.notify("ok", t("device.restored", { tag: row.tag }));
      onrestored();
    } catch (e) {
      app.notify("error", String(e));
    } finally {
      busy = false;
    }
  }
</script>

<section class="taken-over">
  <h3>{t("firmware.reprogrammed")}</h3>
  <p class="hint">{t("firmware.reprogrammedHint", { key: row.emits ?? "" })}</p>
  {#if conflict}
    <p class="conflict">{t("firmware.conflict", { key: row.emits ?? "" })}</p>
  {/if}
  {#if !row.hasBackup}
    <label class="field">
      {t("firmware.restoreChoose")}
      <select bind:value={choice}>
        {#each RESTORABLE as mapping, i (i)}
          <option value={i}>{describeMapping(mapping)}</option>
        {/each}
      </select>
    </label>
  {/if}
  <div class="actions">
    <button type="button" disabled={busy} onclick={restore}>
      {busy ? t("firmware.restoring") : row.hasBackup ? t("firmware.restore") : t("firmware.restoreApply")}
    </button>
    <span class="hint">{t("firmware.restoreRemovesBindings")}</span>
  </div>
</section>

<style>
  .taken-over {
    display: grid;
    gap: 10px;
    padding: 12px 14px;
    margin-bottom: 18px;
    border: 1px dashed var(--line);
    border-radius: var(--radius-m);
  }

  .conflict {
    color: var(--danger);
  }

  h3 {
    color: var(--muted);
    font-weight: 600;
  }

  .actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
  }

  select {
    max-width: 320px;
  }
</style>
