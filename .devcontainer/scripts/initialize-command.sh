#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -ex -o pipefail

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

# Create .devcontainer/.env if it does not already exists
[ -e .devcontainer/.env ] || touch .devcontainer/.env
cp .devcontainer/.env.development .devcontainer/.env
# Load .devcontainer/.env environment variables
source .devcontainer/.env
# Record the host workspace path for Docker bind mounts. The Dev Containers CLI
# provides localWorkspaceFolder; direct invocations fall back to the repository
# root derived from this script's location.
workspace_folder="${LOCAL_WORKSPACE_FOLDER:-${localWorkspaceFolder:-$(cd "${SCRIPT_DIR}/../.." && pwd)}}"
printf "\nLOCAL_WORKSPACE_FOLDER=%s\n" "$workspace_folder" >> .devcontainer/.env

echo "$(pwd)/.devcontainer/.env file initialized successfully"
