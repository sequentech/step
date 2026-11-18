#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"
# A running listener belongs to the existing session; never replace it.
exec 9>/tmp/step-architect-start.lock
flock -n 9 || exit 0
if curl --silent --max-time 2 --output /dev/null http://127.0.0.1:5176/; then
  echo "Architect is already available on port 5176."
  exit 0
fi
if ss -ltnH 'sport = :5176' | grep -q .; then
  echo "Port 5176 is already in use; leaving its process untouched."
  exit 0
fi
if [[ ! -d beyond/packages/node_modules || ! -f beyond/packages/election-architect/vendor/sequent-election-config/index_bg.wasm ]]; then
  echo "Architect dependencies or vendored core are missing; prepare them before running start.architect."
  exit 0
fi
nohup devenv shell bash -- -c 'cd beyond/packages/election-architect && exec yarn start' > /tmp/step-architect.log 2>&1 < /dev/null 9>&- &
echo "Starting Architect on port 5176; log: /tmp/step-architect.log"
