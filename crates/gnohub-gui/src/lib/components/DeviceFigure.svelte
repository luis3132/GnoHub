<script lang="ts">
  // Dibujo interactivo del dispositivo, al estilo de Piper. Con un SVG que
  // sigue la convención de Piper (`buttonN` = pieza, `buttonN-path` = línea
  // guía, `buttonN-leader` = punto de la etiqueta; igual con `ledN`) las
  // etiquetas se colocan en sus guías y la pieza se ilumina. Con una imagen
  // normal se usan las coordenadas `x`, `y` de la definición.

  import { t } from "../i18n/index.svelte";
  import type { ButtonRow } from "../rows";
  import type { Led } from "../types";

  let {
    svg,
    image,
    rows,
    leds = [],
    selected,
    selectedLed,
    assigned,
    describe,
    onselect,
    onselectled,
  }: {
    svg?: string | null;
    image?: string | null;
    rows: ButtonRow[];
    leds?: Led[];
    selected: string | null;
    selectedLed: number | null;
    assigned: (row: ButtonRow) => boolean;
    /** Resumen corto de lo que hace el botón, bajo su nombre. */
    describe: (row: ButtonRow) => string;
    onselect: (row: ButtonRow) => void;
    onselectled: (index: number) => void;
  } = $props();

  interface Anchor {
    x: number;
    y: number;
    side: "left" | "right";
  }

  let frame = $state<HTMLDivElement>();
  let host = $state<HTMLDivElement>();
  let anchors = $state<Record<string, Anchor>>({});
  let ledAnchors = $state<Record<number, Anchor>>({});
  /** Sin sitio para las etiquetas laterales: marcas sobre las piezas, sin líneas guía. */
  let compact = $state(false);
  /** Ancho mínimo del marco para usar etiquetas laterales. */
  const ROOMY = 640;

  // Los dibujos de Piper usan la paleta clara de GNOME (Tango); se pasan a
  // tonos de la app para que no sean una mancha blanca sobre el fondo oscuro.
  const PALETTE: Record<string, string> = {
    "#ffffff": "#3b414b",
    "#eeeeec": "#30353d",
    "#d9dad7": "#2c3037",
    "#d3d3ce": "#2a2e35",
    "#bfc2bb": "#5b626d",
    "#babdb6": "#5b626d",
    "#888a85": "#7b818b",
    "#2e3436": "#16181c",
  };

  function recolor(markup: string): string {
    return markup.replace(/#(?:[0-9a-f]{6})\b/gi, (c) => PALETTE[c.toLowerCase()] ?? c);
  }

  /** Quita del SVG todo lo que pueda ejecutar código o cargar recursos externos. */
  function sanitize(markup: string): SVGSVGElement | null {
    const doc = new DOMParser().parseFromString(recolor(markup), "image/svg+xml");
    const root = doc.documentElement;
    if (root.nodeName !== "svg") return null;
    root.querySelectorAll("script, foreignObject, iframe, object, embed").forEach((n) => n.remove());
    for (const el of [root, ...root.querySelectorAll("*")]) {
      for (const attr of [...el.attributes]) {
        const value = attr.value.trim().toLowerCase();
        const external = /href$/i.test(attr.name) && !value.startsWith("#");
        if (attr.name.toLowerCase().startsWith("on") || external || value.startsWith("javascript:")) {
          el.removeAttribute(attr.name);
        }
      }
    }
    root.removeAttribute("width");
    root.removeAttribute("height");
    return root as unknown as SVGSVGElement;
  }

  $effect(() => {
    if (!host) return;
    const root = svg ? sanitize(svg) : null;
    host.replaceChildren(...(root ? [document.importNode(root, true)] : []));
    measure();
  });

  const el = (id: string) => host?.querySelector<SVGGraphicsElement>(`[id="${id}"]`) ?? null;

  /** Posición de cada etiqueta, relativa al marco, a partir de los puntos `*-leader`. */
  function measure() {
    if (!frame || !host) return;
    compact = frame.clientWidth < ROOMY;
    const base = frame.getBoundingClientRect();
    const svgBox = host.querySelector("svg")?.getBoundingClientRect();
    const center = svgBox ? svgBox.left + svgBox.width / 2 : base.left + base.width / 2;
    const at = (id: string): Anchor | null => {
      // En modo compacto la marca va en el centro de la pieza.
      if (compact) {
        const shape = el(id);
        if (!shape) return null;
        const r = shape.getBoundingClientRect();
        return { x: r.left + r.width / 2 - base.left, y: r.top + r.height / 2 - base.top, side: "right" };
      }
      const target = el(`${id}-leader`) ?? el(id);
      if (!target) return null;
      const r = target.getBoundingClientRect();
      const mid = r.left + r.width / 2;
      const side = mid < center ? "left" : "right";
      return { x: (side === "left" ? r.left : r.right) - base.left, y: r.top + r.height / 2 - base.top, side };
    };
    const next: Record<string, Anchor> = {};
    for (const row of rows) {
      if (row.hwIndex == null) continue;
      const a = at(`button${row.hwIndex}`);
      if (a) next[row.key] = a;
    }
    anchors = next;
    const nextLeds: Record<number, Anchor> = {};
    for (const led of leds) {
      const a = at(`led${led.index}`);
      if (a) nextLeds[led.index] = a;
    }
    ledAnchors = nextLeds;
  }

  $effect(() => {
    if (!frame) return;
    const observer = new ResizeObserver(() => measure());
    observer.observe(frame);
    return () => observer.disconnect();
  });

  // Vuelve a medir si cambian las filas (p. ej. tras reprogramar un botón).
  $effect(() => {
    void rows;
    void leds;
    queueMicrotask(measure);
  });

  /**
   * Cambia una propiedad del estilo en línea recordando su valor original, que
   * se restaura con `null` (los SVG de Piper definen sus colores ahí).
   */
  function paint(node: SVGElement, prop: string, value: string | null) {
    const key = `orig${prop.replace(/-./g, (m) => m[1].toUpperCase())}`;
    if (!(key in node.dataset)) node.dataset[key] = node.style.getPropertyValue(prop);
    node.style.setProperty(prop, value ?? node.dataset[key] ?? "");
  }

  // Colores de las piezas: asignada, seleccionada o normal.
  $effect(() => {
    if (!host) return;
    void svg;
    const css = getComputedStyle(document.documentElement);
    const signal = css.getPropertyValue("--signal").trim();
    const text = css.getPropertyValue("--text").trim();
    for (const row of rows) {
      if (row.hwIndex == null) continue;
      const shape = el(`button${row.hwIndex}`);
      const leader = el(`button${row.hwIndex}-path`);
      const isSelected = selected === row.key;
      const lit = isSelected || assigned(row);
      if (shape) {
        paint(shape, "fill", lit ? signal : null);
        paint(shape, "fill-opacity", lit ? (isSelected ? "1" : "0.6") : null);
        shape.style.cursor = "pointer";
        shape.onclick = () => onselect(row);
      }
      if (leader) {
        paint(leader, "display", compact ? "none" : null);
        for (const part of [leader, ...leader.querySelectorAll<SVGElement>("*")]) {
          const filled = (part.dataset.origFill ?? part.style.fill) && part.style.fill !== "none";
          paint(part, "stroke", isSelected && part.style.stroke !== "none" ? signal : null);
          if (filled) paint(part, "fill", isSelected ? signal : null);
        }
      }
    }
    for (const led of leds) {
      const ledLeader = el(`led${led.index}-path`);
      if (ledLeader) paint(ledLeader, "display", compact ? "none" : null);
      const ring = el(`led${led.index}`);
      if (ring) {
        paint(ring, "stroke", led.mode === 0 ? null : `rgb(${led.color.join(",")})`);
        ring.style.cursor = "pointer";
        ring.onclick = () => onselectled(led.index);
        paint(ring, "filter", selectedLed === led.index ? `drop-shadow(0 0 3px ${text})` : null);
      }
    }
  });

  const freeRows = $derived(rows.filter((r) => !anchors[r.key] && r.x != null && r.y != null));
</script>

<figure>
  <div class="frame" class:svg={!!svg} class:compact bind:this={frame}>
    {#if svg}
      <div class="host" bind:this={host}></div>
    {:else if image}
      <img src={image} alt="" />
    {/if}

    {#each rows as row (row.key)}
      {@const a = anchors[row.key]}
      {#if a}
        <button
          type="button"
          class="label {compact ? 'mark' : a.side}"
          class:assigned={assigned(row)}
          class:firmware={!row.emits}
          class:selected={selected === row.key}
          style:left="{a.x}px"
          style:top="{a.y}px"
          onclick={() => onselect(row)}
          title={compact ? `${row.tag}: ${describe(row)}` : undefined}
        >
          <span class="tag">{row.tag}</span>
          {#if !compact}<span class="desc">{describe(row)}</span>{/if}
        </button>
      {/if}
    {/each}

    {#each leds as led (led.index)}
      {@const a = ledAnchors[led.index]}
      {#if a}
        <button
          type="button"
          class="label led {compact ? 'mark' : a.side}"
          class:selected={selectedLed === led.index}
          style:left="{a.x}px"
          style:top="{a.y}px"
          onclick={() => onselectled(led.index)}
        >
          <span class="swatch" style:background={led.mode === 0 ? "transparent" : `rgb(${led.color.join(",")})`}></span>
          {#if !compact}<span class="desc">{t("led.title", { n: led.index + 1 })}</span>{/if}
        </button>
      {/if}
    {/each}

    {#each freeRows as row (row.key)}
      <button
        type="button"
        class="spot"
        class:assigned={assigned(row)}
        class:selected={selected === row.key}
        style:left="{row.x! * 100}%"
        style:top="{row.y! * 100}%"
        onclick={() => onselect(row)}
        title="{row.tag}: {row.label}"
      >
        {row.tag}
      </button>
    {/each}
  </div>
</figure>

<style>
  figure {
    margin: 0 0 20px;
    display: flex;
    justify-content: center;
  }

  .frame {
    position: relative;
    width: 100%;
    max-width: 760px;
  }

  /* Hueco a los lados para las etiquetas, como en Piper. */
  .frame.svg {
    padding: 0 150px;
  }

  .frame.svg.compact {
    max-width: 420px;
    padding: 0;
  }

  .label.mark {
    width: auto;
    min-width: 26px;
    padding: 1px 6px;
    transform: translate(-50%, -50%);
    justify-items: center;
    border-radius: 11px;
    opacity: 0.92;
  }

  .label.mark .tag {
    font-size: 11px;
  }

  .label.mark.led {
    padding: 3px;
  }

  .host :global(svg) {
    display: block;
    width: 100%;
    height: auto;
    max-height: 420px;
  }

  img {
    display: block;
    max-width: 100%;
    max-height: 360px;
    margin: 0 auto;
  }

  .label {
    position: absolute;
    transform: translateY(-50%);
    display: grid;
    gap: 0;
    width: 148px;
    padding: 4px 9px;
    border-radius: var(--radius-s);
    background: var(--panel);
    text-align: left;
    line-height: 1.25;
  }

  .label.left {
    transform: translate(-100%, -50%);
    text-align: right;
    justify-items: end;
  }

  .tag {
    font-weight: 700;
    font-size: 12.5px;
  }

  .desc {
    font-size: 12px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 130px;
  }

  .label.assigned {
    border-color: var(--signal);
  }

  .label.assigned .tag {
    color: var(--signal);
  }

  .label.firmware {
    border-style: dashed;
  }

  .label.selected {
    background: var(--signal);
    border-color: var(--signal);
  }

  .label.selected .tag,
  .label.selected .desc {
    color: var(--signal-ink);
  }

  .label.led {
    grid-auto-flow: column;
    align-items: center;
    gap: 6px;
  }

  .swatch {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 1px solid var(--line);
  }

  .spot {
    position: absolute;
    transform: translate(-50%, -50%);
    min-width: 26px;
    height: 22px;
    padding: 0 6px;
    justify-content: center;
    border-radius: 11px;
    font-size: 11.5px;
    font-weight: 700;
  }

  .spot.assigned {
    background: var(--signal);
    border-color: var(--signal);
    color: var(--signal-ink);
  }

  .spot.selected {
    outline: 2px solid var(--text);
    outline-offset: 2px;
  }
</style>
