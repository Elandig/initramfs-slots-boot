#!/bin/sh
set -eu

# Install slots-boot and wire it into the local initramfs generator. Run as root.
# Vars: PREFIX (/usr), DESTDIR (stage only), SLOTS_NO_HOOK_EDIT=1 (skip the HOOKS edit).

PREFIX="${PREFIX:-/usr}"
DESTDIR="${DESTDIR:-}"
SRC="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"

log() { printf '%s\n' "$*"; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

if [ -z "$DESTDIR" ] && [ "$(id -u)" -ne 0 ]; then
    die "run as root (or set DESTDIR to stage the install)"
fi

find_binary() {
    for p in \
        "$SRC"/target/*-linux-musl/release/slots-boot \
        "$SRC"/target/release/slots-boot \
        "$SRC"/slots-boot; do
        [ -x "$p" ] && { printf '%s\n' "$p"; return 0; }
    done
    return 1
}

inst() { # mode src dest
    install -Dm"$1" "$2" "$DESTDIR$3"
}

add_mkinitcpio_hook() {
    conf="$DESTDIR/etc/mkinitcpio.conf"
    [ -f "$conf" ] || return 0
    if grep -Eq '^HOOKS=.*[ (]slots[ )]' "$conf"; then
        log "   HOOKS already lists 'slots'"
        return 0
    fi
    if [ "${SLOTS_NO_HOOK_EDIT:-0}" = "1" ]; then
        log "   add 'slots' to HOOKS in $conf yourself (SLOTS_NO_HOOK_EDIT=1)"
        return 0
    fi
    if grep -Eq '^HOOKS=.*filesystems' "$conf"; then
        cp -a "$conf" "$conf.slots.bak"
        sed -i -E '/^HOOKS=/ s/filesystems/slots filesystems/' "$conf"
        log "   inserted 'slots' before 'filesystems' (backup: $conf.slots.bak)"
    else
        log "   couldn't find 'filesystems' in HOOKS - add 'slots' manually before it"
    fi
}

regenerate() {
    if command -v mkinitcpio >/dev/null 2>&1; then
        mkinitcpio -P
    elif command -v update-initramfs >/dev/null 2>&1; then
        update-initramfs -u
    elif command -v dracut >/dev/null 2>&1; then
        dracut -f
    else
        log ":: no initramfs generator on PATH - regenerate yours by hand"
    fi
}

BIN="$(find_binary)" || die "no slots-boot binary found - run 'make' (or 'make static') first"
log ":: using binary $BIN"

inst 755 "$BIN" "$PREFIX/bin/slots-boot"
log ":: installed $PREFIX/bin/slots-boot"

found=0

if command -v mkinitcpio >/dev/null 2>&1 || [ -f "$DESTDIR/etc/mkinitcpio.conf" ]; then
    log ":: mkinitcpio detected"
    inst 644 "$SRC/initramfs/mkinitcpio/install/slots" "$PREFIX/lib/initcpio/install/slots"
    inst 644 "$SRC/initramfs/mkinitcpio/hooks/slots" "$PREFIX/lib/initcpio/hooks/slots"
    # systemd-initramfs unit, used when mkinitcpio is built with the 'systemd' hook.
    inst 644 "$SRC/initramfs/dracut/90slots/slots-boot.service" "$PREFIX/lib/systemd/system/slots-boot.service"
    add_mkinitcpio_hook
    found=1
fi

if command -v dracut >/dev/null 2>&1 || [ -d "$DESTDIR/usr/lib/dracut" ]; then
    log ":: dracut detected"
    inst 755 "$SRC/initramfs/dracut/90slots/module-setup.sh" "$PREFIX/lib/dracut/modules.d/90slots/module-setup.sh"
    inst 755 "$SRC/initramfs/dracut/90slots/slots-hook.sh" "$PREFIX/lib/dracut/modules.d/90slots/slots-hook.sh"
    inst 644 "$SRC/initramfs/dracut/90slots/slots-boot.service" "$PREFIX/lib/dracut/modules.d/90slots/slots-boot.service"
    inst 644 "$SRC/packaging/dracut/90-slots.conf" "/etc/dracut.conf.d/90-slots.conf"
    found=1
fi

if command -v update-initramfs >/dev/null 2>&1 || [ -d "$DESTDIR/etc/initramfs-tools" ]; then
    log ":: initramfs-tools detected"
    inst 755 "$SRC/initramfs/initramfs-tools/hooks/slots" "/etc/initramfs-tools/hooks/slots"
    inst 755 "$SRC/initramfs/initramfs-tools/scripts/init-premount/slots" "/etc/initramfs-tools/scripts/init-premount/slots"
    found=1
fi

[ "$found" -eq 1 ] || die "no supported initramfs generator found (mkinitcpio, dracut, initramfs-tools)"

if [ -n "$DESTDIR" ]; then
    log ":: staged into $DESTDIR (not regenerating initramfs)"
    exit 0
fi

log ":: regenerating initramfs"
regenerate
log ":: done. reboot to meet the machine - the recovery word is in the README, keep it handy"
