#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -euo pipefail

# Only toolchain definitions enter the image; the checkout and its .env stay local.
mkdir -p .devcontainer
: > .devcontainer/.env
git init -q
git add devenv.nix devenv.lock devenv.yaml
devenv shell bash -- -c 'node --version && yarn --version && rustc --version && wasm-bindgen --version'
