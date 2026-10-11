import { mount } from "svelte";
import App from "./App.svelte";
import { api } from "./lib/api";
import "./app.css";

// Errores no capturados de la interfaz: a la salida de la app, para poder diagnosticarlos.
window.addEventListener("error", (e) => api.logError(`${e.message} (${e.filename}:${e.lineno})`));
window.addEventListener("unhandledrejection", (e) => api.logError(`promesa rechazada: ${String(e.reason)}`));

const app = mount(App, { target: document.getElementById("app")! });

export default app;
