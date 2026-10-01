#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

disk-usage() {
    echo "$1"
    echo "Disk usage:"
    df -h
}

export COMPOSE_FILE=/workspaces/step/.devcontainer/docker-compose.yml

df -h

echo "Removing unused programs..."
rm -rf /workspaces/.codespaces/shared/editors/ &> /dev/null
disk-usage "Unused programs removed"

echo "Pruning Docker..."
docker system prune --all --force &> /dev/null
disk-usage "Docker pruned"

echo "Collecting Nix garbage..."
nix-collect-garbage -d &> /dev/null
disk-usage "Nix garbage collected"
