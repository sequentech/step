#!/bin/bash
set -e # Optional: exit immediately if a command exits with a non-zero status.

echo "---- Running initializeCommand as User ----"
echo "User: $(whoami)"
echo "ID: $(id)"
echo "-------------------------------------------"

# --- Rest of your original initialize-command.sh script below ---
# Example:
# echo "Doing other setup steps..."
# ...


#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# set -ex -o pipefail

# SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )

# # Create .devcontainer/.env if it does not already exists
# [ -e .devcontainer/.env ] || touch .devcontainer/.env
# cp .devcontainer/.env.development .devcontainer/.env
# # Load .devcontainer/.env environment variables
# source .devcontainer/.env
# # Set LOCAL_WORKSPACE_FOLDER environment variable if not already set
# [ ! -z "${localWorkspaceFolder}" ] || printf "\nLOCAL_WORKSPACE_FOLDER=${SCRIPT_DIR}/..\n" >> .devcontainer/.env

# echo "$(pwd)/.devcontainer/.env file initialized successfully"
