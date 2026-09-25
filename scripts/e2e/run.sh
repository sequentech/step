#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

# Run the backend E2E journeys on a fresh, isolated compose stack.
#
#   scripts/e2e/run.sh [--keep] [--skip-images] [--skip-build] [-k PATTERN]
#
#   --keep          leave the stack running (tear it down with --down)
#   --down          remove the explicitly named STEP_E2E_PROJECT and its volumes
#   --skip-images   reuse the service images already built
#   --skip-build    reuse the binaries already in STEP_E2E_BIN_DIR
#   --bootstrap-only prepare services and the administrator for another driver
#   -k PATTERN      run through the last matching journey, including prerequisites
#
# Environment:
#   DOCKER                 docker command (default "docker", e.g. "sudo docker")
#   STEP_E2E_PROJECT       compose project name (default unique per invocation)
#   STEP_E2E_OUTPUT_DIR    logs/results (default .cache/backend-e2e/<project>)
#   STEP_E2E_BIN_DIR       binaries (default .cache/backend-e2e/bin)
#   STEP_E2E_PORTS=1       also publish Hasura, Keycloak and MinIO on 127.0.0.1
#   and the build settings documented in scripts/e2e/build.sh
set -euo pipefail

ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
DOCKER=${DOCKER:-docker}
export STEP_E2E_BIN_DIR=${STEP_E2E_BIN_DIR:-$ROOT/.cache/backend-e2e/bin}
DEVCONTAINER=$ROOT/.devcontainer
SERVICES=(
    devcontainer postgres postgres-keycloak postgres-b4 minio configure-minio rabbitmq
    immudb immudb-init data-connector-agent graphql-engine keycloak harvest windmill beat b4
    trustee1 trustee2
)
IMAGES=(postgres postgres-b4 minio configure-minio keycloak harvest)

keep=false images=true build=true down_only=false bootstrap_only=false pattern=()
while (($#)); do
    case "$1" in
        --keep) keep=true ;;
        --down) down_only=true ;;
        --skip-images) images=false ;;
        --skip-build) build=false ;;
        --bootstrap-only) bootstrap_only=true ;;
        -k)
            [[ $# -ge 2 && -n "$2" ]] || { echo '-k requires a pattern' >&2; exit 2; }
            pattern=(-k "$2"); shift ;;
        -h | --help) sed -n '5,22p' "$0"; exit 0 ;;
        *) echo "Unknown option $1" >&2; exit 2 ;;
    esac
    shift
done

if $down_only && [[ -z "${STEP_E2E_PROJECT:-}" ]]; then
    echo '--down requires an explicit STEP_E2E_PROJECT; no stack was removed' >&2
    exit 2
fi
PROJECT=${STEP_E2E_PROJECT:-step-e2e-$(id -u)-$$-$RANDOM}
[[ "$PROJECT" =~ ^[a-z0-9][a-z0-9_-]*$ ]] || { echo "Invalid Compose project: $PROJECT" >&2; exit 2; }
OUTPUT=${STEP_E2E_OUTPUT_DIR:-$ROOT/.cache/backend-e2e/$PROJECT}
# The UI wrapper supplies a fresh token to recognize this invocation's claim,
# even if its caller selected an output directory containing old run markers.
RUN_TOKEN=${STEP_E2E_RUN_TOKEN:-$$-$RANDOM-$RANDOM}
[[ "$RUN_TOKEN" =~ ^[a-zA-Z0-9_-]+$ ]] || { echo 'Invalid run token' >&2; exit 2; }
OWNERSHIP_FILE=$OUTPUT/.owned-$RUN_TOKEN

assert_unused_project() {
    local resources kind
    resources=$($DOCKER ps --all --quiet --filter "label=com.docker.compose.project=$PROJECT") || return
    if [[ -n "$resources" ]]; then
        echo "Project $PROJECT already has containers; choose another STEP_E2E_PROJECT" >&2
        return 1
    fi
    for kind in network volume; do
        resources=$($DOCKER "$kind" ls --quiet --filter "label=com.docker.compose.project=$PROJECT") || return
        if [[ -n "$resources" ]]; then
            echo "Project $PROJECT already has ${kind}s; choose another STEP_E2E_PROJECT" >&2
            return 1
        fi
    done
}
$down_only || assert_unused_project

mkdir -p "$OUTPUT/logs" "$STEP_E2E_BIN_DIR"
# The base compose file reads .devcontainer/.env; never replace a developer's own.
[[ -f "$DEVCONTAINER/.env" ]] || cp "$DEVCONTAINER/.env.development" "$DEVCONTAINER/.env"
cat > "$OUTPUT/compose.env" <<EOF
STEP_E2E_BIN_DIR=$STEP_E2E_BIN_DIR
STEP_E2E_OUTPUT_DIR=$OUTPUT
STEP_E2E_UID=$(id -u)
STEP_E2E_GID=$(id -g)
EOF

files=(-f "$DEVCONTAINER/docker-compose.yml" -f "$DEVCONTAINER/docker-compose-ci.yml")
[[ "${STEP_E2E_PORTS:-}" == 1 ]] && files+=(-f "$DEVCONTAINER/docker-compose-ci-ports.yml")
compose() {
    $DOCKER compose --project-name "$PROJECT" \
        --env-file "$DEVCONTAINER/.env.development" --env-file "$OUTPUT/compose.env" \
        --profile full --profile e2e "${files[@]}" "$@"
}

collect_logs() {
    compose ps --all > "$OUTPUT/logs/services.txt" 2>&1 || true
    for service in "${SERVICES[@]}"; do
        compose logs --no-color --timestamps "$service" > "$OUTPUT/logs/$service.log" 2>&1 || true
    done
}

teardown() {
    compose down --volumes --remove-orphans --timeout 20 > "$OUTPUT/logs/down.log" 2>&1 || return
    rm -f "$OWNERSHIP_FILE"
}

if $down_only; then
    teardown
    exit 0
fi

started=$SECONDS
phase() { printf '\n==> %s (%ss)\n' "$1" "$((SECONDS - started))"; }

if $images; then
    phase "Building service images"
    compose build "${IMAGES[@]}"
fi
if $build; then
    phase "Building backend binaries"
    STEP_E2E_RUNTIME_IMAGE=${STEP_E2E_RUNTIME_IMAGE:-step-backend-e2e-cargo-packages:local} DOCKER=$DOCKER \
        "$ROOT/scripts/e2e/build.sh"
fi

# Builds may take several minutes: check again immediately before taking ownership.
assert_unused_project
printf '%s\n' "$PROJECT" > "$OWNERSHIP_FILE"
status=1
trap 'collect_logs; $keep || teardown || true' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

phase "Starting project $PROJECT; logs in $OUTPUT"
compose up --detach --wait --wait-timeout 900 "${SERVICES[@]}"

phase "Waiting for the super tenant"
set +e
compose run --rm --no-deps driver bootstrap
bootstrapped=$?
set -e
if ((bootstrapped == 3)); then
    phase "Restarting Hasura to reload the JWKS"
    compose restart graphql-engine
    compose up --detach --wait --wait-timeout 300 graphql-engine
    compose run --rm --no-deps driver bootstrap
elif ((bootstrapped != 0)); then
    exit "$bootstrapped"
fi

if $bootstrap_only; then
    phase "Bootstrap complete"
    exit 0
fi

phase "Running the journeys"
set +e
compose run --rm --no-deps driver test ${pattern[@]+"${pattern[@]}"}
status=$?
set -e
phase "Finished with status $status; logs in $OUTPUT"
exit "$status"
