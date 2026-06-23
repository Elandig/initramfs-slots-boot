#!/bin/sh
set -eu

# Remove slots-boot and regenerate the initramfs. Run as root.

PREFIX="${PREFIX:-/usr}"

log() { printf '%s\n' "$*"; }
[ "$(id -u)" -eq 0 ] || { echo "run as root" >&2; exit 1; }

rm -f "$PREFIX/bin/slots-boot"
rm -f "$PREFIX/lib/initcpio/install/slots" "$PREFIX/lib/initcpio/hooks/slots"
rm -f "$PREFIX/lib/systemd/system/slots-boot.service"
rm -rf "$PREFIX/lib/dracut/modules.d/90slots"
rm -f /etc/dracut.conf.d/90-slots.conf
rm -f /etc/initramfs-tools/hooks/slots /etc/initramfs-tools/scripts/init-premount/slots

log ":: removed slots-boot"
log ":: if you added 'slots' to HOOKS in /etc/mkinitcpio.conf, remove it now"

if command -v mkinitcpio >/dev/null 2>&1; then
    mkinitcpio -P
elif command -v update-initramfs >/dev/null 2>&1; then
    update-initramfs -u
elif command -v dracut >/dev/null 2>&1; then
    dracut -f
fi
