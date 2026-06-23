# Arch cloud image (mkinitcpio): builds the package with makepkg. Override IMAGE_URL_OVERRIDE=.

IMAGE_URL="https://geo.mirror.pkgbuild.com/images/latest/Arch-Linux-x86_64-cloudimg.qcow2"
CACHE="arch-cloudimg.qcow2"

user_data() {
    cat <<'YAML'
#cloud-config
chpasswd:
  expire: false
  users:
    - {name: root, password: slots, type: text}
ssh_pwauth: true
write_files:
  - path: /root/slots-install.sh
    permissions: '0755'
    content: |
      #!/bin/bash
      set -euxo pipefail
      pacman -Sy --noconfirm --needed rust base-devel
      id builder >/dev/null 2>&1 || useradd -m builder
      mount -L SLOTSSRC /mnt
      cp -a /mnt /home/builder/build
      umount /mnt
      # makepkg can't fetch the unreleased source; hand it a local tarball
      tar -C /home/builder --exclude=build/target --exclude=build/.git \
        --transform 's,^build,initramfs-slots-boot-0.1.0,' \
        -czf /root/slots-boot-0.1.0.tar.gz build
      cp /root/slots-boot-0.1.0.tar.gz /home/builder/build/packaging/arch/
      chown -R builder:builder /home/builder/build
      cd /home/builder/build/packaging/arch
      sudo -u builder makepkg -f --noconfirm --skipinteg
      pkg=$(ls slots-boot-0.1.0-*-x86_64.pkg.tar.zst | grep -v debug | head -1)
      pacman -U --noconfirm "$pkg"
      # The package can't safely edit HOOKS itself, so arm it here.
      grep -Eq '^HOOKS=.*[ (]slots[ )]' /etc/mkinitcpio.conf \
        || sed -i '/^HOOKS=/ s/filesystems/slots filesystems/' /etc/mkinitcpio.conf
      grep '^HOOKS=' /etc/mkinitcpio.conf
      mkinitcpio -P
      echo SLOTS_INSTALL_OK
runcmd:
  - [ bash, /root/slots-install.sh ]
power_state:
  mode: poweroff
  timeout: 120
  condition: true
YAML
}
