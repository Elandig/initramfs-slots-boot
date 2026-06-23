# Ubuntu cloud image (initramfs-tools): builds the .deb. Override IMAGE_URL_OVERRIDE=.

IMAGE_URL="https://cloud-images.ubuntu.com/releases/noble/release/ubuntu-24.04-server-cloudimg-amd64.img"
CACHE="ubuntu-24.04-cloudimg-amd64.img"

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
      # set -e so a failure aborts before the marker; HOME so cargo has a cache dir.
      set -euxo pipefail
      export HOME=/root DEBIAN_FRONTEND=noninteractive
      mount -L SLOTSSRC /mnt
      cp -a /mnt /root/build
      umount /mnt
      apt-get update
      apt-get install -y cargo dpkg-dev
      cd /root/build
      packaging/debian/build-deb.sh
      dpkg -i slots-boot_*.deb
      echo SLOTS_INSTALL_OK
runcmd:
  - [ bash, /root/slots-install.sh ]
power_state:
  mode: poweroff
  timeout: 120
  condition: true
YAML
}
