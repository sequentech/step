#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

disk-usage() {
    echo "$1"
    echo "Disk usage:"
    df -h
}

STEP_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=/dev/null
source "${STEP_ROOT}/.devcontainer/.env"
export COMPOSE_FILE="${STEP_ROOT}/.devcontainer/docker-compose.yml"

df -h

echo "Removing unused programs..."
rm -rf /workspaces/.codespaces/shared/editors/ &> /dev/null
disk-usage "Unused programs removed"

echo "Pruning Docker..."
docker system prune --all --force &> /dev/null
disk-usage "Docker pruned"

echo "Collecting Nix garbage..."
# The devcontainers of other checkouts share the Nix store; a collection here
# sees neither their roots nor their running builds.
sharing=$(docker ps --filter "volume=${DEVCONTAINER_NIX_VOLUME}" \
    --filter label=devcontainer.local_folder --format '{{.Names}}' |
    grep -vx "${DEVCONTAINER_NAME_PREFIX}devcontainer" || true)
if [ -n "${sharing}" ]; then
    echo "Skipped: the Nix store is in use by ${sharing}"
else
    nix-collect-garbage -d &> /dev/null
fi
disk-usage "Nix garbage collected"

echo "Cleaning ImmuDB database..."
docker compose rm -fs immudb &> /dev/null
docker volume rm -f "${COMPOSE_PROJECT_NAME}_immudb_data" &> /dev/null
docker compose up -d --no-recreate immudb &> /dev/null
disk-usage "ImmuDB cleaned up"
