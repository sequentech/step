#!/bin/bash -i
# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -ex -o pipefail

source .devcontainer/.env
# devenv prepends its Nix OpenSSL to LD_LIBRARY_PATH, but the devcontainer's
# system Docker CLI must load the Ubuntu-compatible OpenSSL libraries.
env -u LD_LIBRARY_PATH docker compose restart graphql-engine

# Compose feeds graphql-engine HASURA_GRAPHQL_ADMIN_SECRET from
# KEYCLOAK_ADMIN_CLIENT_SECRET locally, but remote deployments set it
# independently.
HASURA_ADMIN_SECRET="${HASURA_GRAPHQL_ADMIN_SECRET:-${KEYCLOAK_ADMIN_CLIENT_SECRET:?neither HASURA_GRAPHQL_ADMIN_SECRET nor KEYCLOAK_ADMIN_CLIENT_SECRET is set; expected one from .devcontainer/.env}}" \
    .devcontainer/scripts/generate-graphql.sh

# Format the generated hasura files
cd hasura && yarn && yarn prettify:fix
