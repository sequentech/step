#!/bin/bash

# SPDX-FileCopyrightText: 2024-2025 Sequent Legal <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Usage:
#   ./run_bg_voting.sh
#     [--batches N]
#     [--instances N]
#     [--voting-url URL]
#     [--tenant-id ID]
#     [--password-pattern PATTERN]
#     [--username-pattern PATTERN]
#     [--number-of-voters N]
#     [--save-screenshots true|false]
#
# All arguments are optional. Defaults are shown below.

# Defaults:
NUM_BATCHES=10
INSTANCES_PER_BATCH=4
VOTING_URL="https://voting-test.sequent.vote/tenant/90505c8a-23a9-4cdf-a26b-4e19f6a097d5/event/7d7f840a-4e75-4ba4-b431-633196da1a2c/login"
PASSWORD_PATTERN="user{n}"
USERNAME_PATTERN="user{n}"
NUMBER_OF_VOTERS=4096
SAVE_SCREENSHOTS="false"

# Parse arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --batches)
      NUM_BATCHES="$2"; shift 2;;
    --instances)
      INSTANCES_PER_BATCH="$2"; shift 2;;
    --voting-url)
      VOTING_URL="$2"; shift 2;;
    --password-pattern)
      PASSWORD_PATTERN="$2"; shift 2;;
    --username-pattern)
      USERNAME_PATTERN="$2"; shift 2;;
    --number-of-voters)
      NUMBER_OF_VOTERS="$2"; shift 2;;
    --username-pattern)
      USERNAME_PATTERN="$2"; shift 2;;
    --save-screenshots)
      SAVE_SCREENSHOTS="$2"; shift 2;;
    *)
      echo "Unknown argument: $1"; exit 1;;
  esac
done

mkdir -p logs

kill_chrome_helpers() {
  echo "🔪 Killing Chrome-related processes..."

  # Kill Chrome processes (Linux)
  pkill -f 'chrome' || echo "No chrome processes found."
  pkill -f 'chromium' || echo "No chromium processes found."
  pkill -f 'chromedriver' || echo "No chromedriver processes found."

  # Wait until they’re fully dead (max 15s)
  local WAIT_TIME=0
  local MAX_WAIT=15
  while [[ $WAIT_TIME -lt $MAX_WAIT ]]; do
    local still_chrome=$(pgrep -f 'chrome')
    local still_chromium=$(pgrep -f 'chromium')
    local still_driver=$(pgrep -f 'chromedriver')

    if [[ -z "$still_chrome" && -z "$still_chromium" && -z "$still_driver" ]]; then
      echo "✅ All Chrome-related processes terminated."
      break
    fi

    sleep 1
    WAIT_TIME=$((WAIT_TIME+1))
  done

  if [[ $WAIT_TIME -eq $MAX_WAIT ]]; then
    echo "⚠️ Some Chrome or chromedriver processes may still be running after $MAX_WAIT seconds."
  fi
}

for batch in $(seq 1 $NUM_BATCHES); do
  echo "🚀 Starting batch #$batch..."

  for i in $(seq 1 $INSTANCES_PER_BATCH); do
    run_id=$(( (batch - 1) * INSTANCES_PER_BATCH + i ))
    echo "  🧪 Starting test instance #$run_id"
    export NUMBER_OF_ITERATIONS="1"
    export VOTING_URL="$VOTING_URL"
    export NUMBER_OF_VOTERS="$NUMBER_OF_VOTERS"
    export USERNAME_PATTERN="$USERNAME_PATTERN"
    export PASSWORD_PATTERN="$PASSWORD_PATTERN"
    export SAVE_SCREENSHOTS="$SAVE_SCREENSHOTS"
    npx nightwatch src/voting1.js --env chrome > logs/test_$run_id.log 2>&1 &
  done

  # Wait for tests to start and run briefly
  sleep 15

  echo "⏹ Stopping Chrome processes for batch #$batch..."
  kill_chrome_helpers
  echo "✅ Batch #$batch complete. Chrome processes force-killed."

  sleep 2

done

echo "🏁 All $NUM_BATCHES batches completed."
