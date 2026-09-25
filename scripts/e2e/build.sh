#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

# Build the backend binaries for the backend E2E stack. They are compiled in
# the same bookworm cargo-packages image the stack runs them in, so the host's
# glibc does not matter.
#
# STEP_E2E_RUNTIME_IMAGE  image to build in (default step-e2e-cargo-packages:local)
# STEP_E2E_BIN_DIR        output directory (default .cache/backend-e2e/bin)
# STEP_E2E_CARGO_TARGET   cargo target dir: a docker volume name or an absolute
#                         host path (default volume step-e2e-cargo-target)
# STEP_E2E_CARGO_HOME     registry/git cache: a volume name or an absolute host
#                         path (default volume step-e2e-cargo-home)
# CARGO_BUILD_JOBS        parallel rustc jobs (default: all cores)
set -euo pipefail

ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
IMAGE=${STEP_E2E_RUNTIME_IMAGE:-step-e2e-cargo-packages:local}
BIN_DIR=${STEP_E2E_BIN_DIR:-$ROOT/.cache/backend-e2e/bin}
TARGET=${STEP_E2E_CARGO_TARGET:-step-e2e-cargo-target}
CARGO_CACHE=${STEP_E2E_CARGO_HOME:-step-e2e-cargo-home}
DOCKER=${DOCKER:-docker}

mkdir -p "$BIN_DIR"
if [[ "$CARGO_CACHE" == /* ]]; then
    mkdir -p "$CARGO_CACHE/registry" "$CARGO_CACHE/git"
    registry="$CARGO_CACHE/registry"
    git_cache="$CARGO_CACHE/git"
else
    registry="$CARGO_CACHE-registry"
    git_cache="$CARGO_CACHE-git"
fi
[[ "$TARGET" == /* ]] && mkdir -p "$TARGET"

# The dev profile matches the devcontainer's cargo-watch builds. The trustees
# and B4 run release builds there too, and so they do here.
$DOCKER run --rm \
    --volume "$ROOT:/workspaces/step:ro" \
    --volume "$TARGET:/cargo-target" \
    --volume "$registry:/usr/local/cargo/registry" \
    --volume "$git_cache:/usr/local/cargo/git" \
    --volume "$BIN_DIR:/out" \
    --workdir /workspaces/step/packages \
    --env CARGO_TARGET_DIR=/cargo-target \
    --env CARGO_INCREMENTAL=0 \
    --env CARGO_PROFILE_DEV_DEBUG=line-tables-only \
    --env CARGO_TERM_COLOR=never \
    --env RUSTFLAGS=-Awarnings \
    --env CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-}" \
    --env OUT_UID="$(id -u)" \
    --env OUT_GID="$(id -g)" \
    "$IMAGE" bash -euo pipefail -c '
        [[ -n "$CARGO_BUILD_JOBS" ]] || unset CARGO_BUILD_JOBS
        build() { echo "::group::cargo build $*"; cargo build --locked "$@"; echo "::endgroup::"; }
        build -p windmill --bin main --bin beat
        build -p harvest --bin harvest
        build -p immu-board --bin bb_helper
        build -p step-cli --bin step-cli
        build --release -p b4 --bin b4 --features native
        build --release -p braid --bin main
        install -m 0755 /cargo-target/debug/main /out/windmill
        install -m 0755 /cargo-target/debug/beat /out/beat
        install -m 0755 /cargo-target/debug/harvest /out/harvest
        install -m 0755 /cargo-target/debug/bb_helper /out/bb_helper
        install -m 0755 /cargo-target/debug/step-cli /out/step-cli
        install -m 0755 /cargo-target/release/b4 /out/b4
        install -m 0755 /cargo-target/release/main /out/trustee
        chown "$OUT_UID:$OUT_GID" /out/*
    '
echo "Backend E2E binaries are in $BIN_DIR"
