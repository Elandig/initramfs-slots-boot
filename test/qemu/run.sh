#!/bin/sh
set -eu

# Boot the test initramfs in QEMU.
#   run.sh        play it yourself over the serial console
#   run.sh auto   headless: build a self-playing image and assert it won
# Override the kernel with KERNEL= (defaults to the host's).

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
KERNEL="${KERNEL:-$(ls /boot/vmlinuz-* 2>/dev/null | head -n1)}"
INITRD="$HERE/initramfs.cpio.gz"

[ -n "$KERNEL" ] || { echo "no kernel found - set KERNEL=/path/to/vmlinuz"; exit 1; }

# KVM if we can - a big distro kernel under pure TCG is painfully slow.
ACCEL=""
if [ -w /dev/kvm ]; then
    ACCEL="-enable-kvm -cpu host"
fi

if [ "${1:-play}" = "auto" ]; then
    AUTO=1 "$HERE/build-initramfs.sh"
    LOG="$(mktemp)"
    echo ":: booting headless (kernel: $KERNEL)"
    timeout 150 qemu-system-x86_64 $ACCEL \
        -kernel "$KERNEL" -initrd "$INITRD" \
        -append "console=ttyS0 panic=-1" \
        -m 512 -no-reboot \
        -serial "file:$LOG" -display none || true
    echo "------------------ serial ------------------"
    cat "$LOG"
    echo "--------------------------------------------"
    if grep -q JACKPOT "$LOG" && grep -q SLOTS_DONE "$LOG"; then
        echo "QEMU AUTO TEST PASSED"
        rm -f "$LOG"
    else
        echo "QEMU AUTO TEST FAILED"
        rm -f "$LOG"
        exit 1
    fi
else
    # Always rebuild, so a leftover self-play image from `run.sh auto` doesn't make
    # the interactive run play itself.
    "$HERE/build-initramfs.sh"
    echo ":: booting (kernel: $KERNEL).  spin to win.  quit QEMU with Ctrl-A then X"
    exec qemu-system-x86_64 $ACCEL \
        -kernel "$KERNEL" -initrd "$INITRD" \
        -append "console=ttyS0" \
        -m 512 -nographic -no-reboot \
        -serial mon:stdio
fi
