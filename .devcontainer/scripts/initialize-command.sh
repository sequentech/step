#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Exit on error, print commands, handle pipe failures
set -ex -o pipefail

# Define paths relative to the workspace root where the script is expected to run
DEVCONTAINER_DIR=".devcontainer"
ENV_FILE="${DEVCONTAINER_DIR}/.env"
ENV_DEV_FILE="${DEVCONTAINER_DIR}/.env.development"

# --- Environment File Handling ---

# Ensure the .devcontainer directory exists
mkdir -p "$DEVCONTAINER_DIR"

# Create .env file if it doesn't exist
touch "$ENV_FILE"

# Copy development environment template if it exists
if [ -f "$ENV_DEV_FILE" ]; then
  cp "$ENV_DEV_FILE" "$ENV_FILE"
  echo "Copied content from $ENV_DEV_FILE to $ENV_FILE"
else
  # Decide if this is a fatal error or just a warning
  echo "Warning: Development environment template $ENV_DEV_FILE not found."
  # If this template MUST exist, uncomment the following lines:
  # echo "Error: $ENV_DEV_FILE is mandatory." >&2
  # exit 1
fi

# --- Source Environment Variables (Optional but was in original) ---
# Be cautious sourcing files that might change - only source if needed at this stage.
# Make sure the file exists before sourcing
if [ -f "$ENV_FILE" ]; then
    echo "Sourcing environment variables from $ENV_FILE"
    # Use grep to filter comments/empty lines for safer sourcing
    source <(grep -v -e '^#' -e '^$' "$ENV_FILE")
else
    echo "Warning: $ENV_FILE not found, cannot source variables."
fi

# --- Set LOCAL_WORKSPACE_FOLDER ---

# Determine the workspace folder path.
# Use the 'localWorkspaceFolder' variable if provided externally (common in dev containers),
# otherwise default to the current working directory (pwd).
# Note: DevPod *should* set the working directory to the workspace root.
CURRENT_WORKSPACE_FOLDER="${localWorkspaceFolder:-$(pwd)}"

# Check if LOCAL_WORKSPACE_FOLDER is already defined in the .env file
# Use grep -q for quiet check, check exit status
if ! grep -q -E "^LOCAL_WORKSPACE_FOLDER=" "$ENV_FILE"; then
  # If not defined, append it
  printf "\nLOCAL_WORKSPACE_FOLDER=%s\n" "$CURRENT_WORKSPACE_FOLDER" >> "$ENV_FILE"
  echo "Appended LOCAL_WORKSPACE_FOLDER to $ENV_FILE"
else
  echo "LOCAL_WORKSPACE_FOLDER already exists in $ENV_FILE"
fi

echo "$ENV_FILE processing complete."

# Explicitly exit with success status code 0
exit 0