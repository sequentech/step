#!/bin/bash -i
# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -ex -o pipefail

source .devcontainer/.env

cd packages/sequent-core 
# wasm-pack build --mode no-install --out-name index --release --target web --features=wasmtest
# wasm-pack -v pack . 2>&1 | tee output.log

cd ..
hash=$(grep "shasum:" sequent-core/output.log | awk '{printf $4}')
hash="${hash}\\\""
echo "Hash: ${hash}"
awk -v hash="${hash}" '
  /sequent-core-0.1.0.tgz#/ {
    sub(/#.*/, "#"hash"")
  }
  { print }
' yarn.lock > yarn.lock.tmp

mv yarn.lock.tmp yarn.lock

# rm ./ui-core/rust/sequent-core-0.1.0.tgz ./admin-portal/rust/sequent-core-0.1.0.tgz ./voting-portal/rust/sequent-core-0.1.0.tgz ./ballot-verifier/rust/sequent-core-0.1.0.tgz
# cp sequent-core/pkg/sequent-core-0.1.0.tgz ./ui-core/rust/sequent-core-0.1.0.tgz
# cp sequent-core/pkg/sequent-core-0.1.0.tgz ./admin-portal/rust/sequent-core-0.1.0.tgz
# cp sequent-core/pkg/sequent-core-0.1.0.tgz ./voting-portal/rust/sequent-core-0.1.0.tgz
# cp sequent-core/pkg/sequent-core-0.1.0.tgz ./ballot-verifier/rust/sequent-core-0.1.0.tgz

# rm -rf node_modules ui-core/node_modules voting-portal/node_modules ballot-verifier/node_modules admin-portal/node_modules

# yarn && yarn build:ui-core && yarn build:ui-essentials && yarn build:voting-portal && yarn build:admin-portal