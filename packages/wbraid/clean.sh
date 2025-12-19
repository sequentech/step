# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

#!/usr/bin/env bash
set -euo pipefail

# Colors
YELLOW="\e[33m"
GREEN="\e[32m"
CYAN="\e[36m"
GRAY="\e[90m"
RESET="\e[0m"

echo -e "${YELLOW}Cleaning up WBraid POC...${RESET}"

# --- Remove SQLite databases ---
for file in \
    "crates/service/b4.db" \
    "crates/service/b4.db-shm" \
    "crates/service/b4.db-wal"
do
    if [ -f "$file" ]; then
        rm -f "$file"
        echo -e "${GREEN}✓ Removed $(basename "$file")${RESET}"
    fi
done

# --- Remove WASM build artifacts ---
if [ -d "crates/client/pkg" ]; then
    rm -rf "crates/client/pkg"
    echo -e "${GREEN}✓ Removed WASM build artifacts${RESET}"
fi

# --- Stop and remove LocalStack containers ---
containers=$(docker ps -q --filter ancestor=localstack/localstack || true)
if [ -n "$containers" ]; then
    echo "$containers" | xargs -r docker stop >/dev/null
    echo -e "${GREEN}✓ Stopped LocalStack containers${RESET}"
fi

containers=$(docker ps -aq --filter ancestor=localstack/localstack || true)
if [ -n "$containers" ]; then
    echo "$containers" | xargs -r docker rm >/dev/null
    echo -e "${GREEN}✓ Removed LocalStack containers${RESET}"
fi

echo -e "\n${CYAN}Cleanup complete! You can now run:${RESET}"
echo -e "  ${GRAY}./localstack.sh   - Start LocalStack${RESET}"
echo -e "  ${GRAY}./bb.sh           - Start bulletin board service${RESET}"
echo -e "  ${GRAY}./serve.sh        - Build WASM and serve demo${RESET}"
