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
# schema and the validator. This one adds election_config_xlsx, _templates and
# _archive — a spreadsheet parser, a template engine and a zip writer, none of which
# belong in the voting portal.
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
wasm-pack build \
    --mode no-install \
    --out-name index \
    --out-dir "${OUT_DIR}" \
    --release \
    --target web \
    --features=wasmtest,default_features,election_config_xlsx,election_config_templates,election_config_archive

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
wasm-pack -v pack "${OUT_DIR}"

echo
echo "==> Built ${OUT_DIR}/${PACKAGE_NAME}-0.1.0.tgz"
echo
echo "    Vendor it into whichever package consumes it, the way the four front ends"
echo "    vendor sequent-core, and update that package's lockfile hash."
