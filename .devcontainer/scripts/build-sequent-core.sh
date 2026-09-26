#!/usr/bin/env bash
set -euo pipefail

# SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Regenerates the committed sequent-core tgz files and their yarn.lock hashes.
# The release package needs wasm-opt, which the sequent-core flake provides:
#   cd packages/sequent-core && nix develop --command ../../.devcontainer/scripts/build-sequent-core.sh
# For development builds use scripts/dev/step-dev wasm instead.

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
exec "${root}/scripts/dev/step-dev" wasm --release-package
