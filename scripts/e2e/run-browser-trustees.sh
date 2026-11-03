#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
# Same isolated native backend and voting oracle, with real Chromium trustees.
set -euo pipefail
ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$ROOT"
DOCKER=${DOCKER:-docker}
export STEP_E2E_PROJECT=${STEP_E2E_PROJECT:-step-e2e-browser-${USER:-local}}
controller="$STEP_E2E_PROJECT-browser-trustees"
controller_started=0
for argument in "$@"; do
    case "$argument" in
        --skip-images|--skip-build) ;;
        *) echo "Supported options: --skip-images --skip-build" >&2; exit 2 ;;
    esac
done
# Hold the project claim through browser execution and cleanup.
if ! python3 "$ROOT/scripts/e2e/project_lock.py" --check "$STEP_E2E_PROJECT"; then
    exec python3 "$ROOT/scripts/e2e/project_lock.py" "$STEP_E2E_PROJECT" "$ROOT/scripts/e2e/run-browser-trustees.sh" "$@"
fi
export STEP_E2E_OUTPUT_DIR=${STEP_E2E_OUTPUT_DIR:-$ROOT/.cache/backend-e2e/$STEP_E2E_PROJECT}
mkdir -p "$STEP_E2E_OUTPUT_DIR"
output=$(cd -- "$STEP_E2E_OUTPUT_DIR" && pwd)
export STEP_E2E_OUTPUT_DIR="$output"
RUN_TOKEN=$$-$RANDOM-$RANDOM
OWNERSHIP_FILE=$output/.owned-$RUN_TOKEN
overlay=${STEP_E2E_COMPOSE_OVERLAY:-}
if [[ -n "$overlay" ]]; then
    overlay=$(realpath "$overlay")
    export STEP_E2E_COMPOSE_OVERLAY="$overlay"
fi
cleanup() {
    local status=$?
    if [[ "$controller_started" == 1 ]]; then
        $DOCKER logs "$controller" > "$output/browser-controller.log" 2>&1 || true
        $DOCKER stop "$controller" >/dev/null 2>&1 || true
    fi
    # Bootstrap refuses preexisting resources before creating this run's claim.
    if [[ -f "$OWNERSHIP_FILE" && "$(<"$OWNERSHIP_FILE")" == "$STEP_E2E_PROJECT" ]]; then
        STEP_E2E_RUN_TOKEN=$RUN_TOKEN scripts/e2e/run.sh --down >/dev/null 2>&1 || true
    fi
    exit "$status"
}
trap cleanup EXIT
STEP_E2E_RUN_TOKEN=$RUN_TOKEN scripts/e2e/run.sh --keep --bootstrap-only "$@"
compose=($DOCKER compose --project-name "$STEP_E2E_PROJECT"
    --env-file .devcontainer/.env.development --env-file "$output/compose.env"
    --profile full --profile e2e -f .devcontainer/docker-compose.yml
    -f .devcontainer/docker-compose-ci.yml)
[[ -n "$overlay" ]] && compose+=(-f "$overlay")
"${compose[@]}" stop trustee1 trustee2
# No published port: the controller receives only synthetic fixture identities.
$DOCKER run --detach --rm --init --ipc=host --name "$controller" \
    --network "${STEP_E2E_PROJECT}_default" --network-alias browser-trustees \
    --user "$(id -u):$(id -g)" --volume "$ROOT:/workspaces/step:ro" \
    --workdir /workspaces/step mcr.microsoft.com/playwright:v1.62.1-noble \
    node scripts/e2e/browser-trustees.mjs >/dev/null
controller_started=1
ready=0
for attempt in {1..30}; do
    if $DOCKER exec "$controller" node -e 'fetch("http://localhost:9229/health").then(r => process.exit(r.ok ? 0 : 1)).catch(() => process.exit(1))' >/dev/null 2>&1; then
        ready=1
        break
    fi
    sleep 1
done
[[ "$ready" == 1 ]] || { $DOCKER logs "$controller"; exit 1; }
"${compose[@]}" run --rm --no-deps --entrypoint python3 \
    --env STEP_E2E_BROWSER_TRUSTEES_URL=http://browser-trustees:9229 \
    driver -m scripts.e2e.journeys.browser_journey
