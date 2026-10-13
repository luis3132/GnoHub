#!/usr/bin/env bun
/**
 * Compila y empaqueta GnoHub: lo mismo que `bun tauri build`, más la firma para el
 * actualizador de la app cuando hay llave.
 *
 *   bun run release            (desde crates/gnohub-gui; acepta los argumentos de `tauri build`)
 *
 * Con `TAURI_SIGNING_PRIVATE_KEY` (y su contraseña) en el entorno, cada paquete sale con su
 * firma (`X` + `X.sig`) y la app lleva dentro la llave pública `GNOHUB_UPDATER_PUBKEY`
 * para verificar lo que baja. Sin la llave se compila igual, sin actualización
 * automática: así un build local no la necesita, y uno de CI sin el secret no se rompe.
 *
 * Es el mismo comando que corre el workflow de release, para que no haya un "camino de CI"
 * distinto del local.
 */
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const gui = join(dirname(fileURLToPath(import.meta.url)), "..", "crates", "gnohub-gui");
const args = ["tauri", "build", ...process.argv.slice(2)];

if (process.env.TAURI_SIGNING_PRIVATE_KEY) {
  const pubkey = process.env.GNOHUB_UPDATER_PUBKEY ?? "";
  if (!pubkey) {
    console.error("Hay llave de firma pero falta GNOHUB_UPDATER_PUBKEY (la pública).");
    process.exit(1);
  }
  args.push(
    "--config",
    JSON.stringify({
      bundle: { createUpdaterArtifacts: true },
      plugins: { updater: { pubkey } },
    }),
  );
  console.log("Firmando los paquetes para el actualizador.");
} else {
  console.log("Sin TAURI_SIGNING_PRIVATE_KEY: los paquetes salen sin firma (sin actualización automática).");
}

const result = spawnSync("bunx", args, { cwd: gui, stdio: "inherit", env: process.env });
process.exit(result.status ?? 1);
