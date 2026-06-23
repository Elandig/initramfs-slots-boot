#!/bin/sh
set -eu

# Build slots-boot in a container and drive it through stdin to test the logic.

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
cd "$ROOT"
IMAGE="${IMAGE:-slots-boot-test}"

echo ":: building image (runs cargo test inside)"
docker build -f test/docker/Dockerfile -t "$IMAGE" . || {
    echo ":: build failed - retrying with host networking (restricted bridge?)"
    docker build --network=host -f test/docker/Dockerfile -t "$IMAGE" .
}

echo ":: deterministic play - a fixed seed must reach 7-7-7"
out="$(yes '' | head -5000 | docker run --rm -i -e SLOTS_SEED=1 "$IMAGE")"
printf '%s\n' "$out" | tail -n 3
printf '%s\n' "$out" | grep -q 'JACKPOT' || { echo "FAIL: never hit the jackpot"; exit 1; }
printf '%s\n' "$out" | grep -q '^WON$' || { echo "FAIL: no win reported"; exit 1; }

echo ":: the bypass phrase must let the boot continue"
out="$(printf 'letmeboot' | docker run --rm -i "$IMAGE")"
printf '%s\n' "$out"
printf '%s\n' "$out" | grep -q 'BYPASS' || { echo "FAIL: bypass phrase ignored"; exit 1; }

echo "ALL DOCKER TESTS PASSED"
