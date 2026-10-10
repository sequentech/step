#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Fails when a tracked file other than the development env file enables
# Hasura's query-log, which records every GraphQL request with its variables.

set -uo pipefail

LOG_TYPES_VAR="HASURA_GRAPHQL_ENABLED_LOG_TYPES"
DEVELOPMENT_ONLY_LOG_TYPE="query-log"
DEVELOPMENT_ENV_FILE=".devcontainer/.env.development"

repo_root=$(git rev-parse --show-toplevel) || exit 2
cd "$repo_root" || exit 2

assignment="^[[:space:]]*(export[[:space:]]+|-[[:space:]]*)?[\"']?${LOG_TYPES_VAR}[\"']?[[:space:]]*[:=]"
value_before_token="([^#]*([\"',[:space:]]|:-))?"
token_end="([\"',}[:space:]]|$)"
pattern="${assignment}${value_before_token}${DEVELOPMENT_ONLY_LOG_TYPE}${token_end}"

matches=$(git grep -nE "$pattern" -- . ":(exclude)${DEVELOPMENT_ENV_FILE}")
case $? in
    0)
        echo "${LOG_TYPES_VAR} must not include ${DEVELOPMENT_ONLY_LOG_TYPE} outside ${DEVELOPMENT_ENV_FILE}:" >&2
        echo "$matches" >&2
        exit 1
        ;;
    1)
        echo "No file outside ${DEVELOPMENT_ENV_FILE} enables ${DEVELOPMENT_ONLY_LOG_TYPE}."
        ;;
    *)
        echo "git grep failed" >&2
        exit 2
        ;;
esac
