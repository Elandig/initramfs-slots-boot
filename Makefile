CARGO ?= cargo
PREFIX ?= /usr
DESTDIR ?=

# musl gives a fully static binary that runs in any distro's initramfs.
MUSL_TARGET := $(shell uname -m)-unknown-linux-musl

.PHONY: all build static test install uninstall clean

all: build

build:
	$(CARGO) build --release

static:
	rustup target add $(MUSL_TARGET) 2>/dev/null || true
	$(CARGO) build --release --target $(MUSL_TARGET)
	@echo "==> target/$(MUSL_TARGET)/release/slots-boot"
	@file target/$(MUSL_TARGET)/release/slots-boot 2>/dev/null || true

test:
	$(CARGO) test

install:
	PREFIX=$(PREFIX) DESTDIR=$(DESTDIR) ./install.sh

uninstall:
	PREFIX=$(PREFIX) ./uninstall.sh

clean:
	$(CARGO) clean
