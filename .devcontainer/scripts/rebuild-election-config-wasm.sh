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
# `--locked`, and it is load-bearing rather than tidiness.
#
# Without it cargo is free to re-resolve, and the set it chooses is not the one
# `Cargo.lock` records: `curve25519-dalek 5.0.0` instead of the pinned
# `5.0.0-pre.1`, which drags in `getrandom 0.4.3` beside the `0.3.4` this
# repository pins with `wasm_js` on. 0.4 has no such feature enabled here, and it
# refuses to compile for `wasm32-unknown-unknown` at all:
#
#     error: The wasm32/64-unknown-unknown are not supported by default; you may
#     need to enable the "wasm_js" crate feature
#
# So the pins in `strand/Cargo.toml` — each one carrying a comment about exactly
# this — only hold while the lockfile is honoured. This broke step's own WASM job
# and the three `beyond` jobs that build the core from source, all at once, and
# nothing in the error names the lockfile.
wasm-pack build \
    --mode no-install \
    --out-name index \
    --out-dir "${OUT_DIR}" \
    --release \
    --target web \
    --features=wasmtest,default_features,election_config_xlsx,election_config_templates,election_config_archive \
    -- --locked

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
