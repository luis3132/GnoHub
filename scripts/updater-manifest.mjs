#!/usr/bin/env bun
/**
 * Arma `latest.json`: lo que lee el actualizador de la app para saber si hay una versión
 * nueva, de dónde bajarla y con qué firma verificarla.
 *
 *   bun scripts/updater-manifest.mjs <carpeta-con-paquetes> <tag> <owner/repo> [notas.md]
 *
 * Lo corre el workflow de release después de juntar los paquetes. Cada paquete firmado
 * (`X` + `X.sig`) entra con la clave `linux-{arquitectura}-{formato}` que busca el plugin,
 * así cada instalación se actualiza con su mismo formato: un .rpm con un .rpm, un .deb con
 * un .deb.
 *
 * Sin ningún `.sig` (el build no se firmó: falta el secret) no escribe nada y sale bien:
 * la release se publica igual, solo que sin actualización automática.
 */
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

/** De un nombre de paquete a sus claves, según los nombres que produce el bundler. */
const RULES = [
  { re: /\.x86_64\.rpm$/, keys: ["linux-x86_64-rpm"] },
  { re: /\.aarch64\.rpm$/, keys: ["linux-aarch64-rpm"] },
  { re: /_amd64\.deb$/, keys: ["linux-x86_64-deb"] },
  { re: /_arm64\.deb$/, keys: ["linux-aarch64-deb"] },
];

export function platformsFor(files, baseUrl, readSig) {
  const platforms = {};
  for (const file of files) {
    if (file.endsWith(".sig") || !files.includes(`${file}.sig`)) continue;
    const rule = RULES.find((r) => r.re.test(file));
    if (!rule) continue;
    const entry = { url: `${baseUrl}/${encodeURIComponent(file)}`, signature: readSig(`${file}.sig`).trim() };
    for (const key of rule.keys) platforms[key] = entry;
  }
  return platforms;
}

if (import.meta.main) {
  const [dir, tag, repo, notesFile] = process.argv.slice(2);
  if (!dir || !tag || !repo) {
    console.error("uso: bun scripts/updater-manifest.mjs <carpeta> <tag> <owner/repo> [notas.md]");
    process.exit(2);
  }
  const files = readdirSync(dir);
  const baseUrl = `https://github.com/${repo}/releases/download/${tag}`;
  const platforms = platformsFor(files, baseUrl, (f) => readFileSync(join(dir, f), "utf8"));
  if (Object.keys(platforms).length === 0) {
    console.log("No hay paquetes firmados: la release sale sin actualización automática.");
    process.exit(0);
  }
  const manifest = {
    version: tag.replace(/^v/, ""),
    notes: notesFile && existsSync(notesFile) ? readFileSync(notesFile, "utf8") : "",
    pub_date: new Date().toISOString(),
    platforms,
  };
  writeFileSync(join(dir, "latest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
  console.log(`latest.json: ${Object.keys(platforms).sort().join(", ")}`);
}
