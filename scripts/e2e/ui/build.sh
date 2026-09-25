#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -euo pipefail

ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)
# Admin's production webpack configuration embeds process.env. Give every
# build the same small environment, containing no CI or developer credentials.
for package in ui-core ui-essentials voting-portal admin-portal results-portal ballot-verifier; do
    echo "Building $package production bundle"
    env -i PATH="$PATH" HOME="$HOME" TMPDIR="${TMPDIR:-/tmp}" \
        NODE_OPTIONS=--max-old-space-size=6144 CI=true \
        yarn --cwd "$ROOT/packages/$package" build
done
