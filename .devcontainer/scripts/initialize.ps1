
# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

$SCRIPT_DIR = (Get-Location).Path

# Create .devcontainer/.env if it does not already exists
if (-Not (Test-Path .devcontainer\.env)) {
    New-Item -Path .devcontainer\.env -ItemType File -Force | Out-Null
}
Copy-Item .devcontainer\.env.development .devcontainer\.env
# Load .devcontainer/.env environment variables
. .devcontainer\.env
# Set LOCAL_WORKSPACE_FOLDER environment variable if not already set
if (-Not $localWorkspaceFolder) {
    Add-Content -Path .devcontainer\.env -Value "`nLOCAL_WORKSPACE_FOLDER=$SCRIPT_DIR\..\"
}

Write-Host "$(Get-Location)/.devcontainer/.env file initialized successfully"
