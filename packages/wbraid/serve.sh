#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Build and serve the braid WASM client with atomics support.
# Bash twin of serve.ps1.

set -euo pipefail

GREEN=$'\e[32m'
RED=$'\e[31m'
RESET=$'\e[0m'

cd "$(dirname "$0")"

# Clear any inherited RUSTFLAGS that might interfere with .cargo/config.toml
# (the devcontainer's devenv exports RUSTFLAGS=-Awarnings).
unset RUSTFLAGS

if ! ./build-wasm.sh; then
    echo "${RED}Build failed, not starting server${RESET}"
    exit 1
fi

# In the devcontainer the default port 8080 is taken by Hasura; run e.g.
# `PORT=8081 ./serve.sh` there.
PORT="${PORT:-8080}"
export PORT

echo "${GREEN}Starting development server on http://127.0.0.1:${PORT}${RESET}"
echo "${GREEN}Open http://127.0.0.1:${PORT}/emulator.html${RESET}"
python3 server.py
