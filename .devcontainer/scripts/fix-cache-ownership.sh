#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

# Hands the dependency cache volumes of docker-compose-base.yml to the container
# user. A new volume is mounted root-owned because the image lacks those
# directories, and a volume filled under another UID keeps that owner when
# updateRemoteUserUID maps the user to a different host UID. Only the cache
# volumes change owner.

set -euo pipefail

uid="$(id -u)"
gid="$(id -g)"
for cache in /home/vscode/.cache /home/vscode/.cargo; do
    if [ -d "${cache}" ] && [ "$(stat -c %u "${cache}")" != "${uid}" ]; then
        sudo chown -R "${uid}:${gid}" "${cache}"
    fi
done
