#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"
# A running listener belongs to the existing session; never replace it.
exec 9>/tmp/step-admin-portal-start.lock
flock -n 9 || exit 0
if curl --silent --max-time 2 --output /dev/null http://127.0.0.1:3002/; then
  echo "Admin Portal is already available on port 3002."
  exit 0
fi
if ss -ltnH 'sport = :3002' | grep -q .; then
  echo "Port 3002 is already in use; leaving its process untouched."
  exit 0
fi
if [[ ! -d packages/node_modules ]]; then
  echo "Admin Portal dependencies are missing; prepare them before running start.admin-portal."
  exit 0
fi
nohup devenv shell bash -- -c 'cd packages && exec yarn start:admin-portal' > /tmp/step-admin-portal.log 2>&1 < /dev/null 9>&- &
echo "Starting Admin Portal on port 3002; log: /tmp/step-admin-portal.log"
