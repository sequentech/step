#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -euo pipefail
cd /workspaces/step/packages
export CARGO_TARGET_DIR=/workspaces/step/.e2e/cargo/coverage
source <(cargo llvm-cov show-env --sh)
export RUSTFLAGS="${RUSTFLAGS:-} -C llvm-args=-runtime-counter-relocation"
mkdir -p "$E2E_ARTIFACTS/coverage/rust/unit"
export LLVM_PROFILE_FILE="$E2E_ARTIFACTS/coverage/rust/unit/%m-%p%c.profraw"
cargo test --locked -p sequent-core --lib -p step-cli --bin step-cli --no-fail-fast -- --test-threads=2
cd voting-portal
yarn jest --config jest.config.cjs --runInBand --coverage --coverageProvider=babel \
  --coverageReporters=json --coverageDirectory="$E2E_ARTIFACTS/coverage/frontend/unit/voting-portal"
