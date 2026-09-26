#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -euo pipefail

# The base entrypoint starts nix-daemon asynchronously. In a fresh socket
# directory, automatic store selection can otherwise open the root-owned DB.
export NIX_REMOTE=daemon
connection_log=$(mktemp)
trap 'rm -f "$connection_log" "$connection_log.attempt"' EXIT

# The retry shell receives the log path as its own positional argument.
# shellcheck disable=SC2016
if ! timeout 30s bash -c '
    until nix --extra-experimental-features nix-command store ping --store daemon >"$1.attempt" 2>&1; do
        mv "$1.attempt" "$1"
        sleep 0.1
    done
' bash "$connection_log"; then
    echo 'Nix daemon did not become ready within 30 seconds.' >&2
    cat "$connection_log" >&2
    if ! pidof nix-daemon >&2; then
        echo 'No nix-daemon process is running.' >&2
    fi
    if test -r /tmp/nix-daemon.log; then
        tail -n 40 /tmp/nix-daemon.log >&2
    fi
    exit 1
fi

rm -f "$connection_log" "$connection_log.attempt"
trap - EXIT
exec "$@"
