#!/bin/bash

# SPDX-FileCopyrightText: 2024-2025 Sequent Legal <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Usage:
#   ./run_bg_voting.sh
#     [--batches N]
#     [--instances N]
#     [--env-name NAME]
#     [--tenant-id ID]
#     [--election-event-id ID]
#     [--voting-password PASS]
#     [--number-of-votes N]
#     [--username-pattern PATTERN]
#     [--save-screenshots true|false]
#
# All arguments are optional. Defaults are shown below.

# Defaults:
NUM_BATCHES=400
INSTANCES_PER_BATCH=10
ENV_NAME="dev"
TENANT_ID="90505c8a-23a9-4cdf-a26b-4e19f6a097d5"
ELECTION_EVENT_ID="e14a57a3-0c54-41d9-bceb-89a2c2c206f3"
VOTING_PASSWORD="User1234567!"
NUMBER_OF_VOTES=4096
USERNAME_PATTERN='testsequent2025+{n}@mailinator.com'
SAVE_SCREENSHOTS="false"

# Parse arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --batches)
      NUM_BATCHES="$2"; shift 2;;
    --instances)
      INSTANCES_PER_BATCH="$2"; shift 2;;
    --env-name)
      ENV_NAME="$2"; shift 2;;
    --tenant-id)
      TENANT_ID="$2"; shift 2;;
    --election-event-id)
      ELECTION_EVENT_ID="$2"; shift 2;;
    --voting-password)
      VOTING_PASSWORD="$2"; shift 2;;
    --number-of-votes)
      NUMBER_OF_VOTES="$2"; shift 2;;
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
    export ENV_NAME="$ENV_NAME"
    export TENANT_ID="$TENANT_ID"
    export ELECTION_EVENT_ID="$ELECTION_EVENT_ID"
    export VOTING_PASSWORD="$VOTING_PASSWORD"
    export NUMBER_OF_VOTES="$NUMBER_OF_VOTES"
    export USERNAME_PATTERN="$USERNAME_PATTERN"
    export SAVE_SCREENSHOTS="$SAVE_SCREENSHOTS"
    npx nightwatch tests/voting.js --env chrome > logs/test_$run_id.log 2>&1 &
  done

  # Wait for tests to start and run briefly
  sleep 15

  echo "⏹ Stopping Chrome processes for batch #$batch..."
  kill_chrome_helpers
  echo "✅ Batch #$batch complete. Chrome processes force-killed."

  sleep 2

done

echo "🏁 All $NUM_BATCHES batches completed."
