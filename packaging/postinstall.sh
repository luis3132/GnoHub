#!/bin/sh
# Servicio de usuario activado para todas las sesiones y regla de uinput aplicada.
systemctl --global enable gnohubd.service >/dev/null 2>&1 || :
udevadm control --reload >/dev/null 2>&1 || :
udevadm trigger --subsystem-match=misc --action=change >/dev/null 2>&1 || :
