#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
VENV_DIR="$SCRIPT_DIR/.venv"
OUTPUT_DIR="$SCRIPT_DIR/output"
PACKAGES_DIR="$PROJECT_ROOT/packages"

# Ensure output directory exists
mkdir -p "$OUTPUT_DIR"

# Create virtualenv if it doesn't exist
if [ ! -d "$VENV_DIR" ]; then
    python3 -m venv "$VENV_DIR"
fi

# Activate virtualenv
source "$VENV_DIR/bin/activate"

# Install dependencies
pip install --quiet --disable-pip-version-check -r "$SCRIPT_DIR/requirements.txt"

# Run the dependency listing script
python "$SCRIPT_DIR/list_deps.py" "$PACKAGES_DIR" -o "$OUTPUT_DIR/dependencies.csv"
