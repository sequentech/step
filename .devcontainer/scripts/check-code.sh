#!/bin/bash -i
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -e -o pipefail

source .devcontainer/.env

# check hasura formatting (prettier --check)
cd hasura/
yarn && yarn prettify

# check rust formatting
cd ../packages/
cargo fmt -- --check

# clippy for sequent-core (all features; warnings allowed)
export CARGO_TARGET_DIR="$(pwd)/rust-local-target"
cd ./sequent-core/
cargo clippy --no-deps --all-features -- -A warnings

# check Typescript lint & formatting
cd ..
yarn
yarn lint
yarn prettify

# check java formatting (spotless)
cd ./keycloak-extensions
mvn invoker:run@run-spotless-check
