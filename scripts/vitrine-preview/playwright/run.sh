#!/bin/sh
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Runs a Playwright script in the pinned Chromium image, on the host's
# network, so that it reaches the preview on 127.0.0.1. The script imports
# playwright-core from /pw and may write to /out.
#   run.sh NAME SCRIPT.mjs OUTDIR [args...]
set -eu
name=$1; script=$(cd "$(dirname "$2")" && pwd)/$(basename "$2"); out=$3; shift 3
mkdir -p "$out"
exec docker run --rm -e TRACE="${TRACE:-}" --name "vitrine-ui-harness-$name" --network host \
  --user "$(id -u):$(id -g)" -e HOME=/tmp -e NODE_PATH=/pw \
  -v "${VITRINE_DIR:-$HOME/vitrine}/scripts/video/node_modules:/pw:ro" \
  -v "$(dirname "$script"):/work:ro" -v "$(cd "$out" && pwd):/out" \
  mcr.microsoft.com/playwright:v1.64.1-noble \
  node "/work/$(basename "$script")" "$@"
