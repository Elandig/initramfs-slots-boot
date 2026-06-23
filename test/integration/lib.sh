# Shared helpers for the distro integration tests. Sourced by run.sh.
# shellcheck shell=bash

ROOT="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK="$ROOT/test/integration/work"
mkdir -p "$WORK"

log() { printf '\033[1m:: %s\033[0m\n' "$*"; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

need() { command -v "$1" >/dev/null 2>&1 || die "missing required tool: $1"; }

check_tools() {
    need qemu-system-x86_64
    need qemu-img
    need xorriso
    need curl
    need python3
}

# KVM if we can - a full distro under pure TCG is unbearable.
accel() {
    if [ -w /dev/kvm ]; then
        echo "-enable-kvm -cpu host"
    else
        echo "-cpu max"
    fi
}

# fetch <url> <dest> - cached, resumable.
fetch() {
    local url="$1" dest="$2"
    if [ -s "$dest" ]; then
        log "using cached $(basename "$dest")"
        return 0
    fi
    log "downloading $(basename "$dest")"
    curl -fL --retry 3 -C - -o "$dest.part" "$url"
    mv "$dest.part" "$dest"
}

# Copy-on-write overlay so the cached base image stays pristine.
make_overlay() {
    local base="$1" overlay="$2"
    rm -f "$overlay"
    qemu-img create -q -f qcow2 -b "$base" -F qcow2 "$overlay" 16G
}

# ISO of the repo (label SLOTSSRC) for the guest to build from.
make_src_iso() {
    local out="$1"
    local stage="$WORK/src"
    rm -rf "$stage"
    mkdir -p "$stage"
    tar -C "$ROOT" \
        --exclude=./target \
        --exclude=./.git \
        --exclude=./test/integration/work \
        --exclude='__pycache__' \
        -cf - . | tar -C "$stage" -xf -
    xorriso -as mkisofs -quiet -V SLOTSSRC -J -R -o "$out" "$stage"
}

# A cloud-init NoCloud seed ISO (label cidata) from a user-data file.
make_seed_iso() {
    local user_data="$1" out="$2"
    local stage="$WORK/seed"
    rm -rf "$stage"
    mkdir -p "$stage"
    cp "$user_data" "$stage/user-data"
    cat > "$stage/meta-data" <<EOF
instance-id: slots-boot-test
local-hostname: slots-test
EOF
    xorriso -as mkisofs -quiet -V cidata -J -R -o "$out" "$stage"
}

# Boot 1: build + install the package in the guest, then power off (marker = SLOTS_INSTALL_OK).
boot_install() {
    local overlay="$1" seed="$2" src="$3" logf="$4"
    rm -f "$logf"
    log "boot 1/2: installing the package inside the guest (this builds a toolchain, give it time)"
    # shellcheck disable=SC2046
    timeout 2400 qemu-system-x86_64 $(accel) -m 2048 -smp 2 \
        -drive file="$overlay",if=virtio \
        -drive file="$seed",if=virtio,format=raw,readonly=on \
        -drive file="$src",if=virtio,format=raw,readonly=on \
        -netdev user,id=net0 -device virtio-net-pci,netdev=net0 \
        -display none -no-reboot \
        -serial "file:$logf" || true
    if grep -q SLOTS_INSTALL_OK "$logf"; then
        log "install OK"
        return 0
    fi
    echo "---- last 40 lines of install log ----"
    tail -n 40 "$logf"
    die "install did not finish (no SLOTS_INSTALL_OK marker)"
}

# Boot 2: plain boot; the driver bypasses the gate and waits for login.
boot_gate_test() {
    local overlay="$1" logf="$2"
    local sock="$WORK/serial.sock"
    rm -f "$sock" "$logf"
    log "boot 2/2: the gate should appear; the driver will bypass it and wait for login"
    # shellcheck disable=SC2046
    qemu-system-x86_64 $(accel) -m 2048 -smp 2 \
        -drive file="$overlay",if=virtio \
        -netdev user,id=net0 -device virtio-net-pci,netdev=net0 \
        -display none \
        -serial "unix:$sock,server,nowait" &
    local qpid=$!
    python3 "$ROOT/test/integration/serial_driver.py" \
        --socket "$sock" --log "$logf" --timeout 600 \
        --gate "TO BOOT" --send $'letmeboot\n' --success "login:"
    local rc=$?
    kill "$qpid" 2>/dev/null
    wait "$qpid" 2>/dev/null
    return $rc
}

# Boot the installed system interactively so a human can play the gate.
play_interactive() {
    local overlay="$1"
    log "booting the installed system - the slot machine is below, play it"
    log "spin to win, or type 'letmeboot' to skip. quit QEMU with: Ctrl-A then X"
    echo
    # shellcheck disable=SC2046
    exec qemu-system-x86_64 $(accel) -m 2048 -smp 2 \
        -drive file="$overlay",if=virtio \
        -netdev user,id=net0 -device virtio-net-pci,netdev=net0 \
        -nographic -serial mon:stdio
}
