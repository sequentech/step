#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Writes .devcontainer/.env: .env.development plus the values derived from where
# this checkout lives, so that two checkouts never share containers. Creates the
# dependency cache volumes.

set -e -o pipefail

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
ROOT=$(cd "${SCRIPT_DIR}/../.." && pwd)
cd "${ROOT}"

# shellcheck source=/dev/null
source .devcontainer/.env.development

# Record the host workspace path for Docker bind mounts. The Dev Containers CLI
# provides localWorkspaceFolder; direct invocations fall back to the repository
# root derived from this script's location.
workspace_folder="${LOCAL_WORKSPACE_FOLDER:-${localWorkspaceFolder:-${ROOT}}}"
folder_name="$(basename "${workspace_folder}")"

# The Dev Containers CLI's default project name for the folder. The checkout in
# a folder named `step` keeps the canonical project and container names; any
# other prefixes its container names with the folder name.
checkout_name="$(printf '%s' "${folder_name}" | tr '[:upper:]' '[:lower:]' | tr -cd 'a-z0-9_-')"
if [ -z "${checkout_name}" ]; then
    echo "Cannot derive a Compose project name from the folder name '${folder_name}'" >&2
    exit 1
fi
project_name="${checkout_name}_devcontainer"
name_prefix=""
[ "${project_name}" = "${COMPOSE_PROJECT_NAME}" ] || name_prefix="${checkout_name}-"

# docker-compose-base.yml also mounts the checkout's parent at its host path,
# unless that path would hide a directory the container itself needs.
host_parent="$(dirname "${workspace_folder}")"
host_parent_mount=/mnt/checkout-parent
case "${host_parent}" in
    /workspaces* | /home/vscode | /nix/* | /usr/* | /etc/* | /bin/* | /sbin/* | \
        /lib/* | /lib32/* | /lib64/* | /libx32/* | /proc/* | /sys/* | /dev/* | \
        /boot/* | /run/*) ;;
    /*/*) host_parent_mount="${host_parent}" ;;
esac

sed "s|^COMPOSE_PROJECT_NAME=.*|COMPOSE_PROJECT_NAME=${project_name}|" \
    .devcontainer/.env.development > .devcontainer/.env
cat >> .devcontainer/.env <<EOF

# Derived from this checkout by .devcontainer/scripts/initialize-command.sh
LOCAL_WORKSPACE_FOLDER='${workspace_folder}'
DEVCONTAINER_WORKSPACE_FOLDER='/workspaces/${folder_name}'
DEVCONTAINER_NAME_PREFIX=${name_prefix}
DEVCONTAINER_HOST_PARENT='${host_parent_mount}'
EOF

if command -v docker &> /dev/null; then
    for volume in "${DEVCONTAINER_NIX_VOLUME}" "${DEVCONTAINER_CACHE_VOLUME}" \
        "${DEVCONTAINER_CARGO_VOLUME}"; do
        docker volume create "${volume}" > /dev/null
    done
else
    echo "docker not found: the dependency cache volumes were not created" >&2
fi

echo "${ROOT}/.devcontainer/.env initialized for Compose project ${project_name}"
