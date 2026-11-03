#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
# Real administration portal download/upload with owned native trustees and public verification.
set -euo pipefail
ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$ROOT"
# Docker bind sources use the host checkout alias when it is visible here.
ROOT=$(python3 -c 'from scripts.dev.mode.checkout import load_checkout; c=load_checkout(); print(c.host_root if c.host_root.exists() and c.host_root.samefile(c.root) else c.root)')
cd "$ROOT"
DOCKER=${DOCKER:-docker}
export STEP_E2E_PROJECT=${STEP_E2E_PROJECT:-step-e2e-server-custody-$(id -u)-$$-$RANDOM}
controller="$STEP_E2E_PROJECT-server-custody"
controller_started=0
backend_args=()
build_ui=true
for argument in "$@"; do
    case "$argument" in
        --skip-images|--skip-build) backend_args+=("$argument") ;;
        --skip-ui-build) build_ui=false ;;
        *) echo "Supported options: --skip-images --skip-build --skip-ui-build" >&2; exit 2 ;;
    esac
done
# Hold the project claim through browser execution and cleanup.
if ! python3 "$ROOT/scripts/e2e/project_lock.py" --check "$STEP_E2E_PROJECT"; then
    exec python3 "$ROOT/scripts/e2e/project_lock.py" "$STEP_E2E_PROJECT" "$ROOT/scripts/e2e/run-server-custody.sh" "$@"
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
        STEP_E2E_RUN_TOKEN=$RUN_TOKEN "$ROOT/scripts/e2e/run.sh" --down >/dev/null 2>&1 || true
    fi
    exit "$status"
}
trap cleanup EXIT
if $build_ui; then
    yarn --cwd packages build:ui-core
    yarn --cwd packages build:ui-essentials
    # Parent devenv settings must not replace this run's isolated endpoints.
    env -i PATH="$PATH" HOME="$HOME" NODE_OPTIONS=--max-old-space-size=6144 CI=true yarn --cwd packages build:admin-portal
fi
[[ -f packages/admin-portal/dist/index.html && -f packages/admin-portal/dist/wbraid-trustee/index_bg.wasm ]] || {
    echo 'Build the administration portal and browser bindings before --skip-ui-build' >&2; exit 2;
}
STEP_E2E_RUN_TOKEN=$RUN_TOKEN "$ROOT/scripts/e2e/run.sh" --keep --bootstrap-only "${backend_args[@]}"
compose=($DOCKER compose --project-name "$STEP_E2E_PROJECT"
    --env-file .devcontainer/.env.development --env-file "$output/compose.env"
    --profile full --profile e2e -f .devcontainer/docker-compose.yml
    -f .devcontainer/docker-compose-ci.yml)
[[ -n "$overlay" ]] && compose+=(-f "$overlay")
# No published port: the controller receives only synthetic fixture identities.
fixture_tenant=$("${compose[@]}" exec -T harvest printenv SUPER_ADMIN_TENANT_ID)
$DOCKER run --detach --rm --init --ipc=host --name "$controller" \
    --network "${STEP_E2E_PROJECT}_default" --network-alias server-custody \
    --user "$(id -u):$(id -g)" --volume "$ROOT:/workspaces/step:ro" \
    --volume "$output:/out" --env STEP_E2E_OUTPUT_DIR=/out \
    --env "SUPER_ADMIN_TENANT_ID=$fixture_tenant" \
    --workdir /workspaces/step mcr.microsoft.com/playwright:v1.62.1-noble \
    node --experimental-strip-types scripts/e2e/server-custody.mts >/dev/null
controller_started=1
ready=0
for attempt in {1..30}; do
    if $DOCKER exec "$controller" node -e 'fetch("http://localhost:9230/health").then(r => process.exit(r.ok ? 0 : 1)).catch(() => process.exit(1))' >/dev/null 2>&1; then
        ready=1
        break
    fi
    sleep 1
done
[[ "$ready" == 1 ]] || { $DOCKER logs "$controller"; exit 1; }
"${compose[@]}" run --rm --no-deps \
    --env STEP_E2E_SERVER_CUSTODY_BROWSER_URL=http://server-custody:9230 \
    driver test
