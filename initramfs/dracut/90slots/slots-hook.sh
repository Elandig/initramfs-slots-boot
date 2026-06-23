#!/bin/sh

# Legacy (non-systemd) dracut path: run before the root is mounted.
exec </dev/console >/dev/console 2>&1
/usr/bin/slots-boot
