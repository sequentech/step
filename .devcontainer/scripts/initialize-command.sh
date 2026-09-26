#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Writes .devcontainer/.env: .env.development plus the values derived from where
# this checkout lives, so that two checkouts never share containers.

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

sed "s|^COMPOSE_PROJECT_NAME=.*|COMPOSE_PROJECT_NAME=${project_name}|" \
    .devcontainer/.env.development > .devcontainer/.env
cat >> .devcontainer/.env <<EOF

# Derived from this checkout by .devcontainer/scripts/initialize-command.sh
LOCAL_WORKSPACE_FOLDER='${workspace_folder}'
DEVCONTAINER_WORKSPACE_FOLDER='/workspaces/${folder_name}'
DEVCONTAINER_NAME_PREFIX=${name_prefix}
EOF

echo "${ROOT}/.devcontainer/.env initialized for Compose project ${project_name}"
