# Definiciones de periféricos

Cada archivo `.json` de esta carpeta describe un modelo: qué botones tiene, qué
envía cada uno y dónde está en la imagen. GnoHub funciona con cualquier
dispositivo aunque no tenga definición; la definición solo añade los nombres
reales de los botones (G4, M1…), la imagen y el enlace con la memoria interna.

## Imagen interactiva

Si Piper está instalado y tiene dibujo del modelo, GnoHub lo usa automáticamente
(no hace falta `image`). Para un dibujo propio, un SVG que siga la misma
convención que Piper queda interactivo:

- `buttonN`: la pieza física del botón N de libratbag (se ilumina al asignarlo).
- `buttonN-path`: su línea guía.
- `buttonN-leader`: el punto donde termina la línea y va la etiqueta.
- `ledN`, `ledN-path`, `ledN-leader`: lo mismo para los LED.

Con una imagen PNG, o un SVG sin esa convención, usa `x` e `y` en cada botón para
colocar las marcas.

## Añadir un periférico

1. Busca su VID:PID con `gnohubd list` (o en la barra lateral de GnoHub).
2. Copia `logitech-g502-hero.json` con otro nombre y cambia `name` y `match`.
3. Opcional: pon una imagen (vista superior, SVG o PNG) junto al JSON y su nombre
   en `image` (ver «Imagen interactiva»).
4. Para cada botón:
   - `id` y `label`: cómo se llama y qué es. `label` (y `notes`) puede ser un
     texto o uno por idioma: `{ "en": "Side back", "es": "Lateral atrás" }`.
   - `emits`: lo que envía. Pulsa **Detectar botón** en GnoHub o usa
     `gnohubd watch VID:PID` para verlo. Omítelo si el botón no envía nada
     (lo gestiona el firmware: DPI, perfiles…).
   - `hw_index`: su índice en libratbag, si el ratón está soportado (Piper lo
     muestra al configurar botones). Permite reprogramarlo desde GnoHub.
   - `x`, `y`: dónde está en la imagen, de 0 a 1.
5. Guárdalo en `~/.config/gnohub/devices/` y vuelve a abrir GnoHub. Si algo
   está mal, Ajustes muestra el error.

`schema.json` describe el formato; los editores que entienden JSON Schema
autocompletan y validan los campos gracias a la línea `"$schema"`.

Si lo compartes, añádelo a esta carpeta del proyecto para que lo tenga todo el mundo.
