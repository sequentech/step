#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -euo pipefail
if [[ $# != 1 ]]; then
    echo 'Usage: worker.sh ABSOLUTE_PRIVATE_SHARD_DIRECTORY' >&2
    exit 2
fi
runner_root="$(cd -- "$(dirname -- "$0")/../.." && pwd)"
cd "$runner_root"
exec devenv shell python3 scripts/voting_e2e/load.py worker "$1"
