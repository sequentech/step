#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Regression cases for check-deployment-env-templates.sh. Each case tracks one
# file in a scratch git repository and checks the script's exit code.

set -uo pipefail

CHECK="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/check-deployment-env-templates.sh"
VAR="HASURA_GRAPHQL_ENABLED_LOG_TYPES"

scratch=$(mktemp -d) || exit 2
trap 'rm -rf "$scratch"' EXIT
cases=0
failures=0

# expect <exit code> <tracked path> <file content>
expect() {
    local want=$1 path=$2 content=$3 repo got
    cases=$((cases + 1))
    repo=$(mktemp -d "$scratch/repo.XXXXXX")
    git -C "$repo" init -q 2>/dev/null
    mkdir -p "$repo/$(dirname "$path")"
    printf '%s\n' "$content" > "$repo/$path"
    git -C "$repo" add "$path"
    (cd "$repo" && "$CHECK" >/dev/null 2>&1)
    got=$?
    if [ "$got" -ne "$want" ]; then
        echo "FAIL: expected exit $want, got $got for $path: $content" >&2
        failures=$((failures + 1))
    fi
}

# Any non-comment query-log token outside the development env file fails.
expect 1 .env "${VAR}=startup, http-log, query-log"
expect 1 .env "${VAR}=\"startup, query-log\""
expect 1 .env "${VAR}=\"startup\" # query-log is disabled"
expect 1 env.sh "export ${VAR}=query-log;"
expect 1 env.sh "${VAR}=\${LOG_TYPES#*,},query-log"
expect 1 docker-compose.yml "      ${VAR}: \${${VAR}-query-log}"
expect 1 docker-compose.yml "      - ${VAR}=startup,query-log"
expect 1 config.json "  \"${VAR}\": \"startup, query-log\""
expect 1 Dockerfile "ENV OTHER=startup ${VAR}=query-log"
expect 1 Dockerfile $'ENV '"${VAR}"$'="startup, \\\n    query-log"'
expect 1 deployment.yaml $'- name: '"${VAR}"$'\n  value: startup, query-log'

# Comment lines, other log types and the development env file pass.
expect 0 .devcontainer/.env.development "${VAR}=startup, query-log"
expect 0 .env "# ${VAR}=startup, query-log"
expect 0 docker-compose.yml "      # query-log records every request"
expect 0 .env "${VAR}=startup, http-log, webhook-log, websocket-log"
expect 0 .env "${VAR}=query-logs"

# Outside a git repository the check cannot list tracked files.
cases=$((cases + 1))
outside=$(mktemp -d "$scratch/plain.XXXXXX")
(cd "$outside" && GIT_CEILING_DIRECTORIES="$scratch" "$CHECK" >/dev/null 2>&1)
got=$?
if [ "$got" -ne 2 ]; then
    echo "FAIL: expected exit 2 outside a git repository, got $got" >&2
    failures=$((failures + 1))
fi

if [ "$failures" -ne 0 ]; then
    echo "${failures} of ${cases} cases failed." >&2
    exit 1
fi
echo "All ${cases} cases pass."
