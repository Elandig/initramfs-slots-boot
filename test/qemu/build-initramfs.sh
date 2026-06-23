#!/bin/sh
set -eu

# Build a tiny initramfs (busybox + slots-boot + /init) as a cpio.gz QEMU can boot.
# AUTO=1 bakes in scripted input for the headless test.

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
HERE="$ROOT/test/qemu"
WORK="$HERE/work"
OUT="$HERE/initramfs.cpio.gz"
AUTO="${AUTO:-0}"

log() { printf '%s\n' "$*"; }

# 1. the static binary (always rebuild; cargo is incremental, so it's cheap)
MUSL="$(uname -m)-unknown-linux-musl"
BIN="$ROOT/target/$MUSL/release/slots-boot"
log ":: building the static binary ($MUSL)"
(cd "$ROOT" && (rustup target add "$MUSL" 2>/dev/null || true) && cargo build --release --target "$MUSL")
[ -x "$BIN" ] || { echo "no static binary at $BIN"; exit 1; }

# 2. a static busybox - from $BUSYBOX, the host, or the busybox docker image
mkdir -p "$WORK"
BB="${BUSYBOX:-}"
if [ -z "$BB" ] && command -v busybox >/dev/null 2>&1; then
    BB="$(command -v busybox)"
fi
if [ -z "$BB" ]; then
    log ":: pulling a static busybox out of the busybox:musl image"
    cid="$(docker create busybox:musl)"
    docker cp "$cid:/bin/busybox" "$WORK/busybox" >/dev/null
    docker rm "$cid" >/dev/null
    BB="$WORK/busybox"
fi

# 3. lay out the rootfs
R="$WORK/root"
rm -rf "$R"
mkdir -p "$R"/bin "$R"/proc "$R"/sys "$R"/dev "$R"/etc "$R"/mnt
cp "$BB" "$R/bin/busybox"
chmod 755 "$R/bin/busybox"
if applets="$("$BB" --list 2>/dev/null)"; then
    for a in $applets; do ln -sf busybox "$R/bin/$a"; done
else
    for a in sh mount mkdir cat echo sleep poweroff ln cp modprobe umount mknod; do
        ln -sf busybox "$R/bin/$a"
    done
fi
cp "$BIN" "$R/bin/slots-boot"
chmod 755 "$R/bin/slots-boot"
cp "$HERE/init" "$R/init"
chmod 755 "$R/init"

if [ "$AUTO" = "1" ]; then
    log ":: baking automated input"
    yes '' | head -5000 > "$R/slots-input"
fi

# 4. pack it
( cd "$R" && find . -print0 | cpio --null -o -H newc 2>/dev/null | gzip -9 ) > "$OUT"
log ":: wrote $OUT"
