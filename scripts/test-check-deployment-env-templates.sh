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

# Assignments that enable query-log.
expect 1 .env "${VAR}=startup, http-log, query-log"
expect 1 .env "${VAR}=\"startup, query-log\""
expect 1 .env "${VAR}='query-log,startup'"
expect 1 .env "export ${VAR}=query-log"
expect 1 env.sh "export ${VAR}=query-log;"
expect 1 env.sh "${VAR}=query-log&& run"
expect 1 env.sh "${VAR}=query-log|cat"
expect 1 docker-compose.yml "      ${VAR}: startup, query-log"
expect 1 docker-compose.yml "      ${VAR}: \"query-log\""
expect 1 docker-compose.yml "      - ${VAR}=startup,query-log"
expect 1 docker-compose.yml "      ${VAR}: \${${VAR}:-query-log}"
expect 1 docker-compose.yml "      ${VAR}: \${${VAR}:-startup, query-log}"
expect 1 docker-compose.yml "      ${VAR}: \${${VAR}-query-log}"
expect 1 env.sh "${VAR}=\${${VAR}:=query-log}"
expect 1 docker-compose.yml "      ${VAR}: \${ENABLE_QUERY_LOG:+query-log}"
expect 1 config.json "  \"${VAR}\": \"startup, query-log\""
expect 1 packages/Dockerfile.hasura "ENV ${VAR}=\"startup, query-log\""
expect 1 packages/Dockerfile.hasura "ENV ${VAR} query-log"

# Lines that do not enable it.
expect 0 .devcontainer/.env.development "${VAR}=startup, query-log"
expect 0 .env "# ${VAR}=startup, query-log"
expect 0 .env "${VAR}=\"startup\" # query-log is disabled"
expect 0 env.sh "${VAR}=startup; echo query-log"
expect 0 .env "${VAR}=not-query-log"
expect 0 .env "${VAR}=query-logs"
expect 0 .env "OTHER_LOG_TYPES=query-log"
expect 0 packages/Dockerfile.hasura "ENV ${VAR}=\"startup, http-log\""
expect 0 packages/Dockerfile.hasura "ENV ${VAR}_COPY query-log"

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
