#!/bin/bash -i
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -e -o pipefail

source .devcontainer/.env
# devenv prepends its Nix OpenSSL to LD_LIBRARY_PATH, but the devcontainer's
# system Docker CLI must load the Ubuntu-compatible OpenSSL libraries.
env -u LD_LIBRARY_PATH docker compose restart graphql-engine

# Sourced from .devcontainer/.env, and the same value compose feeds
# graphql-engine as HASURA_GRAPHQL_ADMIN_SECRET. Required, so a missing
# configuration fails immediately instead of looking like a graphql-engine that
# never becomes ready.
HASURA_ADMIN_SECRET="${KEYCLOAK_ADMIN_CLIENT_SECRET:?KEYCLOAK_ADMIN_CLIENT_SECRET is not set; expected it from .devcontainer/.env}" \
    .devcontainer/scripts/generate-graphql.sh

# Format the generated hasura files
cd hasura && yarn && yarn prettify:fix
