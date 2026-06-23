# Fedora cloud image (dracut): builds the .rpm. Filenames rot, so discover the latest
# image rather than hardcoding it. Override with IMAGE_URL_OVERRIDE=.

_fed_base="https://download.fedoraproject.org/pub/fedora/linux/releases"
_fed_ver="$(curl -fsL "$_fed_base/" 2>/dev/null | grep -oE 'releases/[0-9]+/' | grep -oE '[0-9]+' | sort -n | tail -1)"
_fed_imgdir="$_fed_base/$_fed_ver/Cloud/x86_64/images"
_fed_img="$(curl -fsL "$_fed_imgdir/" 2>/dev/null | grep -oE 'Fedora-Cloud-Base-Generic[^"]*\.qcow2' | sort -u | head -1)"

if [ -n "$_fed_ver" ] && [ -n "$_fed_img" ]; then
    IMAGE_URL="$_fed_imgdir/$_fed_img"
else
    IMAGE_URL="" # discovery failed; run.sh reports a clear error
fi
CACHE="fedora-${_fed_ver:-cloud}.qcow2"

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
      export HOME=/root
      mount -L SLOTSSRC /mnt
      cp -a /mnt /root/build
      umount /mnt
      dnf install -y --setopt=install_weak_deps=False cargo rust rpm-build
      mkdir -p /root/rpmbuild/SOURCES
      tar -C /root --exclude=build/target --exclude=build/.git \
        --transform 's,^build,initramfs-slots-boot-0.1.0,' \
        -czf /root/rpmbuild/SOURCES/slots-boot-0.1.0.tar.gz build
      rpmbuild -ba /root/build/packaging/fedora/slots-boot.spec
      dnf install -y /root/rpmbuild/RPMS/*/slots-boot-*.rpm
      echo SLOTS_INSTALL_OK
runcmd:
  - [ bash, /root/slots-install.sh ]
power_state:
  mode: poweroff
  timeout: 120
  condition: true
YAML
}
