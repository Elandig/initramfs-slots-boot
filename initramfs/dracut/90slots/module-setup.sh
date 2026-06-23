#!/bin/bash
# slots-boot dracut module: a pre-mount hook on the legacy path, a unit on systemd.

check() {
    require_binaries /usr/bin/slots-boot || return 1
    return 0
}

depends() {
    return 0
}

install() {
    inst_binary /usr/bin/slots-boot

    if dracut_module_included "systemd"; then
        inst_simple "$moddir/slots-boot.service" "$systemdsystemunitdir/slots-boot.service"
        mkdir -p "${initdir}${systemdsystemunitdir}/initrd.target.wants"
        ln -sf ../slots-boot.service \
            "${initdir}${systemdsystemunitdir}/initrd.target.wants/slots-boot.service"
    else
        inst_hook pre-mount 50 "$moddir/slots-hook.sh"
    fi
}

installkernel() {
    instmods atkbd i8042 hid-generic usbhid # keyboard
}
