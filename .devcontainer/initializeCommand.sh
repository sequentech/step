#!/bin/bash -i

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

# Create .devcontainer/.env if it does not already exists
[ -e .devcontainer/.env ] || touch .devcontainer/.env
cp .devcontainer/windmill.env .devcontainer/.env
# Load .devcontainer/.env environment variables
source .devcontainer/.env
# Set LOCAL_WORKSPACE_FOLDER environment variable if not already set
[ ! -z "${localWorkspaceFolder}" ] || printf "\nLOCAL_WORKSPACE_FOLDER=${SCRIPT_DIR}/..\n" >> .devcontainer/.env

# perform the build in parallel, it's faster. Later it will be called again by
# the devcontainer but then all images will have been built already
cd .devcontainer/
docker compose build --parallel
