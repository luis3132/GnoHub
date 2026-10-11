#!/bin/sh
# Al desinstalar del todo, borrar los permisos concedidos con «Dar acceso».
case "$1" in
  0|remove|purge)
    rm -f /etc/udev/rules.d/70-gnohub.rules
    udevadm control --reload >/dev/null 2>&1 || :
    ;;
esac
