#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -euo pipefail

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

# Generate new local credentials only when missing. Reopening the development
# container must preserve credentials already used by its persistent databases.
python3 "$SCRIPT_DIR/initialize-local-secrets.py"
# Record the host workspace path for Docker bind mounts. The Dev Containers CLI
# provides localWorkspaceFolder; direct invocations fall back to the repository
# root derived from this script's location.
workspace_folder="${LOCAL_WORKSPACE_FOLDER:-${localWorkspaceFolder:-$(cd "${SCRIPT_DIR}/../.." && pwd)}}"
python3 - "$SCRIPT_DIR/../.env" "$workspace_folder" <<'PY'
from pathlib import Path
import re, shlex, sys
path = Path(sys.argv[1])
text = re.sub(r'^LOCAL_WORKSPACE_FOLDER=.*\n?', '', path.read_text(), flags=re.MULTILINE)
path.write_text(text + '\nLOCAL_WORKSPACE_FOLDER=' + shlex.quote(sys.argv[2]) + '\n')
PY

echo "$(pwd)/.devcontainer/.env file initialized successfully"
