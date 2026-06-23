#!/usr/bin/env bash
set -euo pipefail

# Distro integration test + an interactive "play" mode. See README.md.
#   run.sh <distro>        build + install the package, check the gate
#   run.sh all             all three distros
#   run.sh <distro> play   boot the installed image and play it yourself

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
# shellcheck source=lib.sh
. "$HERE/lib.sh"

distro="${1:-}"
mode="${2:-test}"

case "$distro" in
    arch | ubuntu | fedora) ;;
    all)
        rc=0
        for d in ubuntu fedora arch; do "$0" "$d" || rc=1; done
        exit "$rc"
        ;;
    *) die "usage: run.sh <arch|ubuntu|fedora|all> [test|play]" ;;
esac
case "$mode" in
    test | play) ;;
    *) die "usage: run.sh <distro> [test|play]" ;;
esac

check_tools
overlay="$WORK/$distro-overlay.qcow2"

# Play mode: just boot the image a previous test run installed.
if [ "$mode" = "play" ]; then
    [ -f "$overlay" ] || die "no installed $distro image yet - run 'test/integration/run.sh $distro' first to build one"
    play_interactive "$overlay"
fi

# Test mode: build + install the package, then headlessly check the gate.
# shellcheck source=/dev/null
. "$HERE/$distro.sh"
IMAGE_URL="${IMAGE_URL_OVERRIDE:-$IMAGE_URL}"
[ -n "$IMAGE_URL" ] || die "could not resolve a $distro image URL (set IMAGE_URL_OVERRIDE=<url>)"

base="$WORK/$CACHE"
seed="$WORK/$distro-seed.iso"
src="$WORK/src.iso"
ud="$WORK/$distro-user-data"

log "distro: $distro"
fetch "$IMAGE_URL" "$base"
make_overlay "$base" "$overlay"
make_src_iso "$src"
user_data > "$ud"
make_seed_iso "$ud" "$seed"

boot_install "$overlay" "$seed" "$src" "$WORK/$distro-install.log"

if boot_gate_test "$overlay" "$WORK/$distro-gate.log"; then
    log "PASS: $distro - package installed, the gate appeared, and the recovery word booted it through"
    log "play it yourself with: test/integration/run.sh $distro play"
else
    echo "---- last 40 lines of the gate log ----"
    tail -n 40 "$WORK/$distro-gate.log" 2>/dev/null || true
    die "FAIL: $distro gate test"
fi
