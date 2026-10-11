// Internacionalización: idioma activo y función `t()`.
//
// El idioma inicial es el que se eligió en Ajustes (guardado en el navegador
// de la app) o, si no, el del sistema. `t()` lee el idioma activo, así que
// todo lo que la usa se vuelve a pintar al cambiarlo.

import { api } from "../api";
import { en, type Dictionary } from "./en";
import { es } from "./es";

export const LOCALES = { en: "English", es: "Español" } as const;
export type Locale = keyof typeof LOCALES;

const DICTIONARIES: Record<Locale, Dictionary> = { en, es };
const STORAGE_KEY = "gnohub.locale";

class I18n {
  locale = $state<Locale>("en");
}

export const i18n = new I18n();

function lookup(dict: unknown, key: string): string | undefined {
  let node = dict;
  for (const part of key.split(".")) {
    if (typeof node !== "object" || node === null) return undefined;
    node = (node as Record<string, unknown>)[part];
  }
  return typeof node === "string" ? node : undefined;
}

/** Texto de `key` en el idioma activo (inglés si falta), con `{nombre}` sustituidos. */
export function t(key: string, params: Record<string, string | number> = {}): string {
  const text = lookup(DICTIONARIES[i18n.locale], key) ?? lookup(en, key) ?? key;
  return text.replace(/\{(\w+)\}/g, (m, name) => (name in params ? String(params[name]) : m));
}

/** ¿Existe `key` en el diccionario? (para textos opcionales, como etiquetas de teclas). */
export function has(key: string): boolean {
  return lookup(en, key) !== undefined;
}

function isLocale(value: unknown): value is Locale {
  return typeof value === "string" && value in LOCALES;
}

export async function initLocale() {
  let saved: string | null = null;
  try {
    saved = localStorage.getItem(STORAGE_KEY);
  } catch {
    // Sin almacenamiento: se usa el idioma del sistema.
  }
  const system = await api.systemLocale().catch(() => "en");
  await setLocale(isLocale(saved) ? saved : isLocale(system) ? system : "en", false);
}

export async function setLocale(locale: Locale, remember = true) {
  i18n.locale = locale;
  document.documentElement.lang = locale;
  if (remember) {
    try {
      localStorage.setItem(STORAGE_KEY, locale);
    } catch {
      // No es grave: solo no se recordará.
    }
  }
  // Los mensajes del backend (errores de validación, de ratbagd…) también.
  await api.setLocale(locale).catch(() => {});
}

/** Texto de una definición de periférico: cadena o `{ en, es, … }`. */
export function localized(text: string | Record<string, string> | null | undefined): string {
  if (text == null) return "";
  if (typeof text === "string") return text;
  return text[i18n.locale] ?? text.en ?? Object.values(text)[0] ?? "";
}
