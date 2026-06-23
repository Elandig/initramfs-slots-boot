# Distro integration tests

These boot a **real** distro in QEMU, build and install the actual package
(`.deb` / `.rpm` / Arch package), regenerate its initramfs, reboot, and check that the
slot machine gates the boot - then type the recovery word and confirm the system boots
the rest of the way.

```sh
test/integration/run.sh ubuntu     # Debian/Ubuntu, initramfs-tools, .deb
test/integration/run.sh fedora     # Fedora, dracut, .rpm
test/integration/run.sh arch       # Arch, mkinitcpio, makepkg
test/integration/run.sh all
```

## Play it yourself

Once a distro has been set up, boot that exact installed image and play the machine
over the console in your terminal:

```sh
test/integration/run.sh ubuntu play
test/integration/run.sh fedora play
test/integration/run.sh arch play
```

The slot machine shows up in your terminal - spin to win, or type `letmeboot` to skip.
Quit QEMU with **Ctrl-A** then **X**. After you're through the gate the distro finishes
booting to a login prompt; the root password is `slots` (set on freshly-installed
images).

## What it does

Each run is two boots of a throwaway copy-on-write overlay:

1. **Install boot** - the repo goes in on a labelled ISO, cloud-init installs the build
   toolchain, builds the package, installs it, and regenerates the initramfs, then
   powers off. Success is the `SLOTS_INSTALL_OK` marker on the serial console.
2. **Gate boot** - a plain boot. The slot machine should now be in the initramfs.
   [`serial_driver.py`](serial_driver.py) watches the serial console, types `letmeboot`
   when it sees the gate, and waits for a `login:` prompt. Seeing the gate *and* then a
   login prompt is a pass.

## Requirements

- `qemu-system-x86_64`, `qemu-img`, `xorriso`, `curl`, `python3`
- KVM (`/dev/kvm`). Without it QEMU falls back to TCG, which is too slow to be useful
  here.
- Bandwidth and disk: a cloud image (~0.5-1 GB, cached in `work/`) plus a toolchain
  downloaded inside the guest.

## Notes

- Cloud image URLs are pinned per distro in `ubuntu.sh` / `fedora.sh` / `arch.sh`.
  They move over time; override with `IMAGE_URL_OVERRIDE=<url>`.
- Everything lands in `work/` (git-ignored). Delete it to reclaim space; cached base
  images are kept between runs.
- This is the same thing CI runs in `.github/workflows/integration.yml` (on demand and
  weekly, since it needs KVM and takes a while).
