#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequent.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -euo pipefail
[[ $# -eq 2 && ( $1 = check || $1 = up ) ]] || {
    echo "usage: .devcontainer/scripts/comelec-receiver.sh {check|up} COMPOSE_PROJECT" >&2
    exit 2
}
ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$ROOT"
base=.devcontainer
site="$(pwd -P)/$base/.comelec-receiver.env"
helper=.devcontainer/comelec-receiver/compose-receiver.sh
[[ -f $base/.env && -f $site && ! -L $site && -x $helper ]] || {
    echo "Requires approved fenced Step bootstrap, a verified local site file and the checked-in Step receiver helper." >&2
    exit 1
}
export COMPOSE_PROFILES=full
exec "$helper" "$1" "$2" "$ROOT/$base" "$base/.env" "$site" "$base/docker-compose.yml"
