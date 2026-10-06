#!/usr/bin/env bash
set -euo pipefail

# SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Builds the WASM package the election-configuration SPAs use.
#
# This is a *second* package from the same crate, and it exists so the four front
# ends that already vendor sequent-core do not have to carry what they will never
# use. They build with `wasmtest,default_features`, which gives them the bundle
# schema and the validator. This one adds election_config_xlsx, _templates,
# _archive and _signing — a spreadsheet parser, a template engine, a zip writer and
# the certificate checks behind `verifyConfigurationPackage`, none of which belong
# in the voting portal. That export exists in this package only.
#
# wasm-pack takes the npm package name from the crate name, so both builds would
# otherwise produce sequent-core-0.1.0.tgz. The rename below is what keeps them
# apart; everything else is the same pipeline as rebuild-sequent-core-full.sh.
#
# Run from inside packages/sequent-core via nix develop:
#   cd packages/sequent-core && nix develop --command ../../.devcontainer/scripts/rebuild-election-config-wasm.sh

PACKAGE_NAME="sequent-election-config"
OUT_DIR="pkg-election-config"

echo "==> Checking versions..."
rustc --version
wasm-pack --version

echo "==> Building ${PACKAGE_NAME} WASM..."
# `--locked`, and it is load-bearing rather than tidiness.
#
# Without it cargo is free to re-resolve, and the set it chooses need not be the
# one `Cargo.lock` records. The WASM build depends on that set: curve25519-dalek
# 5.0 pulls `getrandom 0.4` beside the `0.3.4` this repository pins, and each line
# compiles for `wasm32-unknown-unknown` only because `strand/Cargo.toml` names it
# directly with `wasm_js` on. A resolution that brings in another getrandom line
# fails with
#
#     error: The wasm32/64-unknown-unknown are not supported by default; you may
#     need to enable the "wasm_js" crate feature
#
# and nothing in that error names the lockfile. This once broke step's own WASM
# job and the three `beyond` jobs that build the core from source, all at once.
wasm-pack build \
    --mode no-install \
    --out-name index \
    --out-dir "${OUT_DIR}" \
    --release \
    --target web \
    --features=wasmtest,default_features,election_config_xlsx,election_config_templates,election_config_archive,election_config_signing \
    -- --locked

echo "==> Checking that the package exports what only its features provide..."
# An export behind a feature disappears without a build error when the feature
# list above loses that feature.
for exported in verifyConfigurationPackage openConfiguration; do
    if ! grep -q "export function ${exported}(" "${OUT_DIR}/index.d.ts"; then
        echo "error: ${OUT_DIR}/index.d.ts does not export ${exported}" >&2
        exit 1
    fi
done

echo "==> Renaming the package so it does not collide with sequent-core..."
# node rather than sed: package.json is JSON, and a regex over it is how a build
# script starts corrupting files that happen to contain the same string twice.
node -e '
  const fs = require("fs");
  const path = process.argv[1] + "/package.json";
  const manifest = JSON.parse(fs.readFileSync(path, "utf8"));
  manifest.name = process.argv[2];
  manifest.description =
    "Reading, building and validating a Sequent election event import, in the browser";
  fs.writeFileSync(path, JSON.stringify(manifest, null, 2) + "\n");
' "${OUT_DIR}" "${PACKAGE_NAME}"

echo "==> Packing..."
# `npm pack` rather than `wasm-pack pack`, which takes the *crate* directory and
# looks for a `pkg` child inside it — so with a custom --out-dir it goes hunting
# for pkg-election-config/pkg and fails. `wasm-pack pack` is a wrapper around
# `npm pack` in the output directory, which is exactly this, and it drops the
# tarball in the directory it runs from.
(cd "${OUT_DIR}" && npm pack)

echo
echo "==> Built ${OUT_DIR}/${PACKAGE_NAME}-0.1.0.tgz"
echo
echo "    Vendor it into whichever package consumes it, the way the four front ends"
echo "    vendor sequent-core, and update that package's lockfile hash."
