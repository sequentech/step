#!/bin/sh
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

# cargo-watch kills this entire process group on source changes or container stop.
# Keep that default: a worker holding old leases must exit before its replacement.
trap 'exit 0' INT TERM HUP

while true; do
    "$@"
    status=$?
    printf 'Worker exited (status %s); restarting in 5 seconds\n' "$status" >&2
    sleep 5
done
