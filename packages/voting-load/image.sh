#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -euo pipefail
if [[ $# != 2 || ( "$1" != k6 && "$1" != chromium ) ]]; then
    echo 'Usage: image.sh k6|chromium IMAGE_TAG' >&2
    exit 2
fi
runner_root="$(cd -- "$(dirname -- "$0")/../.." && pwd)"
cd "$runner_root"
# An explicit source-only context also works with Docker's classic builder.
# Private census, tokens, ciphertexts and node_modules never reach the daemon.
if docker buildx version >/dev/null 2>&1; then
    build_command=(docker buildx build --load)
else
    build_command=(docker build)
fi
tar -cf - packages/voting-load/Dockerfile packages/voting-load/*.py \
    packages/voting-load/*.js packages/voting-portal/playwright.scale.config.ts \
    packages/voting-portal/test/load/{flow,scale.spec}.ts |
    "${build_command[@]}" --target "$1" -t "$2" -f packages/voting-load/Dockerfile -
