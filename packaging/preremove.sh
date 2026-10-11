#!/bin/sh
# Solo al desinstalar del todo (no al actualizar): desactivar el servicio.
# rpm pasa "0" al quitar el paquete; dpkg pasa "remove".
case "$1" in
  0|remove|purge) systemctl --global disable gnohubd.service >/dev/null 2>&1 || : ;;
esac
