#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
# Same isolated native backend and voting oracle, with real Chromium trustees.
set -euo pipefail
ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$ROOT"
export STEP_E2E_PROJECT=${STEP_E2E_PROJECT:-step-e2e-browser-${USER:-local}}
controller="$STEP_E2E_PROJECT-browser-trustees"
controller_started=0
for argument in "$@"; do
    case "$argument" in
        --skip-images|--skip-build) ;;
        *) echo "Supported options: --skip-images --skip-build" >&2; exit 2 ;;
    esac
done
overlay=$(mktemp)
export STEP_E2E_COMPOSE_OVERLAY="$overlay"
cat > "$overlay" <<'YAML'
services:
  harvest:
    environment:
      HARVEST_TRUSTEE_ALLOWED_ORIGINS: http://localhost:9229
YAML
cleanup() {
    local status=$?
    if [[ "$controller_started" == 1 ]]; then
        docker logs "$controller" > "$output/browser-controller.log" 2>&1 || true
        docker stop "$controller" >/dev/null 2>&1 || true
    fi
    scripts/e2e/run.sh --down >/dev/null 2>&1 || true
    rm -f "$overlay"
    exit "$status"
}
trap cleanup EXIT
scripts/e2e/run.sh --keep --bootstrap-only "$@"
output="$ROOT/.cache/backend-e2e/$STEP_E2E_PROJECT"
compose=(docker compose --project-name "$STEP_E2E_PROJECT"
    --env-file .devcontainer/.env.development --env-file "$output/compose.env"
    --profile full --profile e2e -f .devcontainer/docker-compose.yml
    -f .devcontainer/docker-compose-ci.yml -f "$overlay")
"${compose[@]}" stop trustee1 trustee2
# No published port: the controller receives only synthetic fixture identities.
docker run --detach --rm --init --ipc=host --name "$controller" \
    --network "${STEP_E2E_PROJECT}_default" --network-alias browser-trustees \
    --user "$(id -u):$(id -g)" --volume "$ROOT:/workspaces/step:ro" \
    --workdir /workspaces/step mcr.microsoft.com/playwright:v1.62.1-noble \
    node scripts/e2e/browser-trustees.mjs >/dev/null
controller_started=1
ready=0
for attempt in {1..30}; do
    if docker exec "$controller" node -e 'fetch("http://localhost:9229/health").then(r => process.exit(r.ok ? 0 : 1)).catch(() => process.exit(1))' >/dev/null 2>&1; then
        ready=1
        break
    fi
    sleep 1
done
[[ "$ready" == 1 ]] || { docker logs "$controller"; exit 1; }
"${compose[@]}" run --rm --no-deps --entrypoint python3 \
    --env STEP_E2E_BROWSER_TRUSTEES_URL=http://browser-trustees:9229 \
    driver -m scripts.e2e.journeys.browser_journey
