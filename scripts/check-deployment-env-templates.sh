#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Fails when a tracked file other than the development env file mentions
# Hasura's query-log, which records every GraphQL request with its variables.
# Any query-log token outside a # or // comment line counts, whatever the
# syntax around it, so a value split over lines or built from an expansion is
# caught.

set -uo pipefail

LOG_TYPE="query-log"
DEVELOPMENT_ENV_FILE=".devcontainer/.env.development"

repo_root=$(git rev-parse --show-toplevel) || exit 2
cd "$repo_root" || exit 2

matches=$(git grep -nIE \
    -e "(^|[^A-Za-z0-9_])${LOG_TYPE}([^A-Za-z0-9_-]|$)" \
    --and --not -e '^[[:space:]]*(#|//)' \
    -- . \
    ":(exclude)${DEVELOPMENT_ENV_FILE}" \
    ":(exclude)scripts/check-deployment-env-templates.sh" \
    ":(exclude)scripts/test-check-deployment-env-templates.sh")
case $? in
    0)
        echo "${LOG_TYPE} may only be enabled in ${DEVELOPMENT_ENV_FILE}; outside it, mention it only on comment lines:" >&2
        echo "$matches" >&2
        exit 1
        ;;
    1)
        echo "No file outside ${DEVELOPMENT_ENV_FILE} enables ${LOG_TYPE}."
        ;;
    *)
        echo "git grep failed" >&2
        exit 2
        ;;
esac
