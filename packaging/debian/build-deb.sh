#!/bin/sh
set -eu

# Build a .deb without debhelper. Run from anywhere.

VERSION="${VERSION:-0.1.0}"
ARCH="${ARCH:-$(dpkg --print-architecture 2>/dev/null || echo amd64)}"
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
cd "$ROOT"

# Prefer a static musl build; fall back to a normal one.
MUSL="$(uname -m)-unknown-linux-musl"
if rustup target list --installed 2>/dev/null | grep -q "$MUSL"; then
    cargo build --release --target "$MUSL"
    BIN="target/$MUSL/release/slots-boot"
else
    cargo build --release
    BIN="target/release/slots-boot"
fi

PKGDIR="$(mktemp -d)"
trap 'rm -rf "$PKGDIR"' EXIT

install -Dm755 "$BIN" "$PKGDIR/usr/bin/slots-boot"
install -Dm755 initramfs/initramfs-tools/hooks/slots \
    "$PKGDIR/usr/share/initramfs-tools/hooks/slots"
install -Dm755 initramfs/initramfs-tools/scripts/init-premount/slots \
    "$PKGDIR/usr/share/initramfs-tools/scripts/init-premount/slots"
install -Dm644 LICENSE "$PKGDIR/usr/share/doc/slots-boot/copyright"

mkdir -p "$PKGDIR/DEBIAN"
cat > "$PKGDIR/DEBIAN/control" <<EOF
Package: slots-boot
Version: $VERSION
Section: admin
Priority: optional
Architecture: $ARCH
Maintainer: Elandig <elan@sestudio.org>
Depends: initramfs-tools
Description: slot machine that gates your boot until you hit the jackpot
 Runs from the initramfs and won't let the boot finish until you spin 7-7-7.
EOF

for s in postinst postrm; do
    sed '/#DEBHELPER#/d' "packaging/debian/$s" > "$PKGDIR/DEBIAN/$s"
    chmod 755 "$PKGDIR/DEBIAN/$s"
done

OUT="slots-boot_${VERSION}_${ARCH}.deb"
dpkg-deb --build --root-owner-group "$PKGDIR" "$OUT"
echo "built $OUT"
