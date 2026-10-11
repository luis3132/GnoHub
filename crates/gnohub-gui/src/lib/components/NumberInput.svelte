<script lang="ts">
  // Campo numérico que nunca deja la configuración con un valor vacío o
  // inválido: mientras se escribe solo se aplica un número válido, y al salir
  // del campo se restaura el último valor correcto.
  let {
    value = $bindable(),
    min,
    optional = false,
    placeholder,
    label,
  }: { value: number | undefined; min?: number; optional?: boolean; placeholder?: string; label?: string } = $props();

  let text = $state(value === undefined ? "" : String(value));
  let focused = $state(false);

  // Si el valor cambia desde fuera (deshacer, cambiar de macro…), se refleja.
  $effect(() => {
    const v = value;
    if (!focused) text = v === undefined ? "" : String(v);
  });

  function parse(raw: string): number | undefined | null {
    if (raw.trim() === "") return optional ? undefined : null;
    const n = Number(raw);
    if (!Number.isFinite(n) || !Number.isInteger(n)) return null;
    if (min !== undefined && n < min) return null;
    return n;
  }

  const invalid = $derived(parse(text) === null);
</script>

<input
  type="number"
  {min}
  {placeholder}
  aria-label={label}
  aria-invalid={invalid}
  value={text}
  onfocus={() => (focused = true)}
  oninput={(e) => {
    text = e.currentTarget.value;
    const n = parse(text);
    if (n !== null) value = n;
  }}
  onblur={() => {
    focused = false;
    text = value === undefined ? "" : String(value);
  }}
/>
