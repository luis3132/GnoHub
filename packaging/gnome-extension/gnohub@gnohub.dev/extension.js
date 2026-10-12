// Publica en el bus de sesión la aplicación con el foco:
//   servicio dev.gnohub.Focus, objeto /dev/gnohub/Focus
//   Get() -> (app_id, wm_class, pid) y señal Changed(app_id, wm_class, pid)
// El PID permite a GnoHub saber qué ejecutable o juego de Steam hay detrás.
// Nunca se publica el título de la ventana.
// El daemon de GnoHub lo usa para los perfiles por aplicación.

import Gio from "gi://Gio";
import GLib from "gi://GLib";
import Shell from "gi://Shell";
import { Extension } from "resource:///org/gnome/shell/extensions/extension.js";

const IFACE = `
<node>
  <interface name="dev.gnohub.Focus">
    <method name="Get">
      <arg type="s" direction="out" name="app_id"/>
      <arg type="s" direction="out" name="wm_class"/>
      <arg type="u" direction="out" name="pid"/>
    </method>
    <signal name="Changed">
      <arg type="s" name="app_id"/>
      <arg type="s" name="wm_class"/>
      <arg type="u" name="pid"/>
    </signal>
  </interface>
</node>`;

export default class GnoHubFocus extends Extension {
  enable() {
    this._last = null;
    this._service = Gio.DBusExportedObject.wrapJSObject(IFACE, this);
    this._service.export(Gio.DBus.session, "/dev/gnohub/Focus");
    this._nameId = Gio.bus_own_name_on_connection(
      Gio.DBus.session,
      "dev.gnohub.Focus",
      Gio.BusNameOwnerFlags.NONE,
      null,
      null,
    );
    this._focusId = global.display.connect("notify::focus-window", () => this._notify());
  }

  disable() {
    global.display.disconnect(this._focusId);
    Gio.bus_unown_name(this._nameId);
    this._service.unexport();
    this._service = null;
    this._last = null;
  }

  _current() {
    const win = global.display.focus_window;
    if (!win) return ["", "", 0];
    const app = Shell.WindowTracker.get_default().get_window_app(win);
    const appId = (app?.get_id() ?? "").replace(/\.desktop$/, "");
    return [appId, win.get_wm_class() ?? "", Math.max(0, win.get_pid() ?? 0)];
  }

  Get() {
    return this._current();
  }

  _notify() {
    const current = this._current();
    const key = current.join("\n");
    if (key === this._last) return;
    this._last = key;
    this._service.emit_signal("Changed", new GLib.Variant("(ssu)", current));
  }
}
