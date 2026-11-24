#!/usr/bin/env bash
set -euo pipefail

# SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Recovery: regenerates the committed sequent-core packages and removes their
# installed copies, so the next yarn install extracts them again. Other
# node_modules, the shared Yarn cache and dist trees are left alone.
#
# Run from inside packages/sequent-core via nix develop:
#   cd packages/sequent-core && nix develop --command ../../.devcontainer/scripts/rebuild-sequent-core-full.sh
# then run yarn install in packages/.

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
"${root}/scripts/dev/step-dev" wasm --release-package
rm -rf "${root}"/packages/node_modules/sequent-core "${root}"/packages/*/node_modules/sequent-core
