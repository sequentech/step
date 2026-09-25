#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -euo pipefail

ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)
DOCKER=${DOCKER:-docker}
export STEP_E2E_BIN_DIR=${STEP_E2E_BIN_DIR:-$ROOT/.cache/backend-e2e/bin}
keep=false ui_build=true down_only=false backend_args=(--keep --bootstrap-only)
for argument in "$@"; do
    case "$argument" in
        --keep) keep=true ;;
        --skip-ui-build) ui_build=false ;;
        --skip-images|--skip-build) backend_args+=("$argument") ;;
        --down) down_only=true ;;
        *) echo "Unknown option: $argument" >&2; exit 2 ;;
    esac
done

if $down_only && [[ -z "${STEP_E2E_PROJECT:-}" ]]; then
    echo '--down requires an explicit STEP_E2E_PROJECT; no stack was removed' >&2
    exit 2
fi
export STEP_E2E_PROJECT=${STEP_E2E_PROJECT:-step-e2e-ui-$(id -u)-$$-$RANDOM}
# Keep the claim through backend startup, browser execution and outer cleanup.
# The backend inherits and validates the same locked descriptor.
if ! python3 "$ROOT/scripts/e2e/project_lock.py" --check "$STEP_E2E_PROJECT"; then
    exec python3 "$ROOT/scripts/e2e/project_lock.py" "$STEP_E2E_PROJECT" "$0" "$@"
fi
export STEP_E2E_OUTPUT_DIR=${STEP_E2E_OUTPUT_DIR:-$ROOT/.cache/backend-e2e/$STEP_E2E_PROJECT}
OUTPUT=$STEP_E2E_OUTPUT_DIR
# Always create a fresh token; an old output directory cannot confer ownership.
RUN_TOKEN=$$-$RANDOM-$RANDOM
OWNERSHIP_FILE=$OUTPUT/.owned-$RUN_TOKEN

compose() {
    $DOCKER compose --project-name "$STEP_E2E_PROJECT" \
        --env-file "$ROOT/.devcontainer/.env.development" --env-file "$OUTPUT/compose.env" \
        --profile full --profile e2e --profile ui-e2e \
        -f "$ROOT/.devcontainer/docker-compose.yml" \
        -f "$ROOT/.devcontainer/docker-compose-ci.yml" \
        -f "$ROOT/.devcontainer/docker-compose-ui-e2e.yml" "$@"
}

if $down_only; then
    compose down --volumes --remove-orphans --timeout 20
    exit 0
fi

mkdir -p "$OUTPUT/logs"
$ui_build && "$ROOT/scripts/e2e/ui/build.sh"

cleanup() {
    # The backend creates this marker only after refusing preexisting resources.
    [[ -f "$OWNERSHIP_FILE" && "$(<"$OWNERSHIP_FILE")" == "$STEP_E2E_PROJECT" ]] || return 0
    compose logs --no-color --timestamps > "$OUTPUT/logs/ui-stack.log" 2>&1 || true
    if ! $keep; then
        compose down --volumes --remove-orphans --timeout 20 > "$OUTPUT/logs/ui-down.log" 2>&1 && rm -f "$OWNERSHIP_FILE" || true
    fi
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

DOCKER=$DOCKER STEP_E2E_RUN_TOKEN=$RUN_TOKEN "$ROOT/scripts/e2e/run.sh" "${backend_args[@]}"
rm -f "$OUTPUT/origins.json" "$OUTPUT/fixture.json"
compose up --detach --wait --wait-timeout 120 ui-browser
compose run --rm --no-deps --entrypoint python3 driver -m scripts.e2e.ui.seed
compose exec -T ui-browser yarn --cwd packages/ui-e2e test
