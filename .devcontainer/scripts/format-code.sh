#!/bin/bash -i
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -ex -o pipefail

STEP_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." > /dev/null 2>&1 && pwd)"

cd "${STEP_ROOT}"
source .devcontainer/.env

# lint & prettify hasura
cd "${STEP_ROOT}/hasura"
yarn && yarn prettify:fix

# lint & prettify rust code
cd "${STEP_ROOT}/packages"
cargo fmt

# lint & prettify Typescript code
yarn
yarn lint:fix
yarn prettify:fix

# lint & prettify java code
cd "${STEP_ROOT}/packages/keycloak-extensions"
mvn clean install
mvn invoker:run@run-spotless-apply

# lint & prettify private java code when the optional beyond checkout is available
if [ -e "${STEP_ROOT}/beyond/.git" ]; then
    "${STEP_ROOT}/beyond/scripts/format-keycloak-extensions.sh"
else
    echo "beyond not cloned; skipping beyond keycloak extension formatting"
fi
