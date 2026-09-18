#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -euo pipefail
cd /workspaces/step/packages
# The PDF unit test uses rust-headless-chrome's executable discovery, while
# Playwright keeps the pinned browser outside the system PATH.
export CHROME="$(node -p "require('@playwright/test').chromium.executablePath()")"
export CARGO_TARGET_DIR=/workspaces/step/.e2e/cargo/coverage
source <(cargo llvm-cov show-env --sh)
export RUSTFLAGS="${RUSTFLAGS:-} -C llvm-args=-runtime-counter-relocation"
mkdir -p "$E2E_ARTIFACTS/coverage/rust/unit"
export LLVM_PROFILE_FILE="$E2E_ARTIFACTS/coverage/rust/unit/%m-%p%c.profraw"
# Cargo identifies the current test executables even when compilation is cached.
# Enumerating debug/deps would also include old feature/toolchain variants.
cargo test --locked -p sequent-core --lib -p step-cli --bin step-cli --no-run \
  --message-format=json > "$E2E_ARTIFACTS/private/unit-artifacts.jsonl"
python3 - <<'PY'
import json, os
from pathlib import Path
artifacts = Path(os.environ["E2E_ARTIFACTS"])
objects = {}
for line in (artifacts / "private/unit-artifacts.jsonl").read_text().splitlines():
    item = json.loads(line)
    if item.get("reason") == "compiler-artifact" and item.get("profile", {}).get("test") and item.get("executable"):
        objects[item["target"]["name"]] = item["executable"]
if set(objects) != {"sequent_core", "step-cli"}:
    raise RuntimeError("Expected current Cargo unit executables for sequent-core and step-cli")
(artifacts / "coverage/rust/unit/objects.json").write_text(json.dumps(sorted(objects.values())))
PY
cargo test --locked -p sequent-core --lib -p step-cli --bin step-cli --no-fail-fast -- --test-threads=2
cd voting-portal
yarn jest --config jest.config.cjs --runInBand --coverage --coverageProvider=babel \
  --coverageReporters=json --coverageDirectory="$E2E_ARTIFACTS/coverage/frontend/unit/voting-portal"
