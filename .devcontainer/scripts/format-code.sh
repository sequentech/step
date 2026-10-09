#!/bin/bash -i
# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -ex -o pipefail

source .devcontainer/.env

# lint & prettify hasura
cd hasura/
yarn && yarn prettify:fix

# lint & prettify rust code
cd ../packages/
cargo fmt

# lint & prettify Typescript code
yarn
yarn lint:fix
yarn prettify:fix

# lint & prettify java code
cd ./keycloak-extensions
mvn clean install
mvn invoker:run@run-spotless-apply
