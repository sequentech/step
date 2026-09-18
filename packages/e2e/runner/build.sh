#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -euo pipefail
cd /workspaces/step
mode="${1:-normal}"
stage="${2:-all}"
case "$mode" in normal|coverage) ;; *) exit 2;; esac
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0
export YARN_CACHE_FOLDER=/workspaces/step/.e2e/yarn-cache
mkdir -p ".e2e/bin/$mode" ".e2e/web/$mode" .e2e/cargo
install_binary() {
  # An earlier retained stack may still execute the previous inode.
  cp "$1" "$2.tmp"
  mv -f "$2.tmp" "$2"
}
if [[ "$stage" == all || "$stage" == native ]]; then
  # Build trustees outside the instrumented environment; they have a separate
  # slow cryptographic test tier and are not part of the backend coverage scope.
  if [[ "$mode" == normal || ! -f .e2e/bin/normal/trustee ]]; then
    (
      cd packages
      export CARGO_TARGET_DIR=/workspaces/step/.e2e/cargo/normal
      mkdir -p ../.e2e/bin/normal
      cargo build --locked -p braid --bin main --features native
      install_binary "$CARGO_TARGET_DIR/debug/main" ../.e2e/bin/normal/trustee
    )
  fi
  (
    cd packages
    export CARGO_TARGET_DIR="/workspaces/step/.e2e/cargo/$mode"
    if [[ "$mode" == coverage ]]; then
      source <(cargo llvm-cov show-env --sh)
      export RUSTFLAGS="${RUSTFLAGS:-} -C llvm-args=-runtime-counter-relocation"
    fi
    cargo build --locked -p harvest --bin harvest -p step-cli --bin step-cli -p b4 --bin b4 --features b4/native
    for binary in harvest step-cli b4; do install_binary "$CARGO_TARGET_DIR/debug/$binary" "../.e2e/bin/$mode/$binary"; done
    cargo build --locked -p windmill --bin main --bin beat
    install_binary "$CARGO_TARGET_DIR/debug/main" "../.e2e/bin/$mode/windmill"
    install_binary "$CARGO_TARGET_DIR/debug/beat" "../.e2e/bin/$mode/beat"
  )
fi
if [[ "$stage" == all || "$stage" == web ]]; then
  (
    unset RUSTFLAGS LLVM_PROFILE_FILE
    export CARGO_TARGET_DIR=/workspaces/step/.e2e/cargo/wasm
    cd packages/sequent-core
    wasm-pack build --mode no-install --out-name index --release --target web \
      --out-dir /workspaces/step/.e2e/wasm/sequent-core --features wasmtest,default_features
  )
  cd packages
  yarn install --frozen-lockfile --non-interactive
  yarn build:ui-core
  yarn build:ui-essentials
  for portal in voting-portal admin-portal ballot-verifier results-portal; do
    E2E_BUILD_MODE="$mode" NODE_OPTIONS=--max-old-space-size=4096 \
      yarn webpack --config e2e/webpack.cjs --env "portal=$portal" --mode production
  done
fi
git rev-parse HEAD > "/workspaces/step/.e2e/bin/$mode/source-sha"
PYTHONPATH=/workspaces/step/packages python3 -c 'from e2e.runner.process import source_digest; print(source_digest())' \
  > "/workspaces/step/.e2e/bin/$mode/source-digest"
