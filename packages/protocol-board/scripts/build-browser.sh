#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -euo pipefail
cd "$(dirname "$0")/../.."
export CARGO_TARGET_DIR="$PWD/rust-local-target/browser-trustee"
unset RUSTFLAGS
expected_bindgen=$(sed -n '/^name = "wasm-bindgen"$/{n;s/^version = "\(.*\)"$/\1/p}' Cargo.lock)
installed_bindgen=$(wasm-bindgen --version | awk '{print $2}')
[[ "$installed_bindgen" == "$expected_bindgen" ]] || {
    echo "wasm-bindgen CLI must match Cargo.lock ($expected_bindgen)" >&2
    exit 1
}
RUSTC_BOOTSTRAP=1 cargo --config wbraid/crates/braid/.cargo/config.toml build \
  -p protocol-board --lib --target wasm32-unknown-unknown --release \
  --no-default-features --features browser --locked
wasm-bindgen "$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/protocol_board.wasm" \
  --out-dir admin-portal/public/wbraid-trustee --target web --out-name index

# Portals build in node-only images, so vendor this package like sequent-core.
node - <<'JS'
const fs = require("fs")
fs.writeFileSync("admin-portal/public/wbraid-trustee/package.json", JSON.stringify({
    name: "wbraid-trustee", version: "0.1.0", type: "module", license: "AGPL-3.0-only",
    files: ["index.js", "index.d.ts", "index_bg.wasm", "index_bg.wasm.d.ts", "snippets"],
}, null, 2))
JS
(cd admin-portal/public/wbraid-trustee && npm pack --pack-destination ../../rust)
# Refresh only this vendored package's integrity; preserve all other resolutions.
node - <<'JS'
const fs = require("node:fs")
const crypto = require("node:crypto")
const hash = crypto.createHash("sha1").update(fs.readFileSync("admin-portal/rust/wbraid-trustee-0.1.0.tgz")).digest("hex")
const lock = fs.readFileSync("yarn.lock", "utf8")
const updated = lock.replace(/(  resolved "file:\.\/admin-portal\/rust\/wbraid-trustee-0\.1\.0\.tgz#)[a-f0-9]+(")/, (_, prefix, suffix) => prefix + hash + suffix)
if (updated !== lock) fs.writeFileSync("yarn.lock", updated)
JS
