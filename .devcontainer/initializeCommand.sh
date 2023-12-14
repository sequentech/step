#!/bin/bash -i

set -ex -o pipefail

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

# Create .devcontainer/.env if it does not already exists
[ -e .devcontainer/.env ] || touch .devcontainer/.env
cp .devcontainer/.env.development .devcontainer/.env
# Load .devcontainer/.env environment variables
source .devcontainer/.env
# Set LOCAL_WORKSPACE_FOLDER environment variable if not already set
[ ! -z "${localWorkspaceFolder}" ] || printf "\nLOCAL_WORKSPACE_FOLDER=${SCRIPT_DIR}/..\n" >> .devcontainer/.env

echo "$(pwd)/.devcontainer/.env file initialized successfully"