# Compile slots-boot as a static musl binary and run its unit tests. This only
# exercises the game logic: a container shares the host kernel and never runs an
# initramfs, so the real boot path is tested with QEMU instead (see test/qemu).
FROM rust:alpine AS build
RUN apk add --no-cache musl-dev
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo test --release
RUN cargo build --release

FROM alpine:3.20
COPY --from=build /src/target/release/slots-boot /usr/bin/slots-boot
ENTRYPOINT ["/usr/bin/slots-boot"]
