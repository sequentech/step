#!/usr/bin/env bash

# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
#
# run_bg_voting.sh
# ------------------
# Purpose:
#   Orchestrate Nightwatch voting tests with configurable concurrency and
#   iterations, using Nightwatch's built-in parallel test workers.
#
# Key ideas:
#   - NUM_BATCHES maps directly to NUMBER_OF_ITERATIONS used by src/voting.js.
#     The test script loops internally, so we don't need an outer loop here.
#   - INSTANCES_PER_BATCH controls the parallelism (number of concurrent test
#     instances). To guarantee parallel runs, we generate N short-lived copies
#     of the base test file and run Nightwatch in parallel against that folder.
#
# Usage examples:
#   ./run_bg_voting.sh \
#     --batches 200 \
#     --instances 8 \
#     --voting-url "https://voting-test.sequent.vote/tenant/.../login" \
#     --number-of-voters 4096 \
#     --username-pattern 'user{n}' \
#     --password-pattern 'user{n}' \
#     --save-screenshots false
#
# Notes:
#   - This script does NOT kill Chrome processes mid-run. Nightwatch manages
#     its own webdriver sessions; forcibly killing them can corrupt runs.
#   - We keep a per-run temporary folder with duplicated tests and remove it
#     on exit.
#   - Nightwatch parallel mode is enabled via --parallel and --workers.
#     The config is explicitly provided so relative paths resolve.

set -Eeuo pipefail

# Defaults
NUM_BATCHES=10                    # -> NUMBER_OF_ITERATIONS for voting.js
INSTANCES_PER_BATCH=4             # number of parallel test instances
VOTING_URL="${VOTING_URL:-https://voting-test.sequent.vote/tenant/90505c8a-23a9-4cdf-a26b-4e19f6a097d5/event/7d7f840a-4e75-4ba4-b431-633196da1a2c/login}"
PASSWORD_PATTERN="user{n}"
USERNAME_PATTERN="user{n}"
NUMBER_OF_VOTERS=4096
SAVE_SCREENSHOTS="false"
BASE_TEST_REL="nightwatch/src/voting.js"  # base test to duplicate
KEEP_PARALLEL_FILES="false"               # set true to keep generated tests
NW_ENV="default"                          # Nightwatch env (default=headless, chrome=non-headless)
ENABLE_VOTER_TRACKING="true"              # enable anti-double-voting (default: true)
PREVIOUS_VOTERS_FILE=""                   # path to previous run's used_voters.txt to exclude
VOTER_MIN_INDEX=1                         # voter indexes will range from [VOTER_MIN_INDEX, VOTER_MIN_INDEX + NUMBER_OF_VOTERS)
CANDIDATES_PATTERN=""                     # regex to filter out candidates which name doesn't match the expression (for example "/^(?!.*Ungültig wählen).*$/" to match any candidate that doesn't include the text "Ungültig wählen")


# Parse arguments
while [[ $# -gt 0 ]]; do
  case "$1" in
    --batches)
      NUM_BATCHES="$2"; shift 2 ;;
    --instances)
      INSTANCES_PER_BATCH="$2"; shift 2 ;;
    --voting-url)
      VOTING_URL="$2"; shift 2 ;;
    --password-pattern)
      PASSWORD_PATTERN="$2"; shift 2 ;;
    --username-pattern)
      USERNAME_PATTERN="$2"; shift 2 ;;
    --candidates-pattern)
      CANDIDATES_PATTERN="$2"; shift 2 ;;
    --number-of-voters)
      NUMBER_OF_VOTERS="$2"; shift 2 ;;
    --voter-min-index)
      VOTER_MIN_INDEX="$2"; shift 2 ;;
    --save-screenshots)
      SAVE_SCREENSHOTS="$2"; shift 2 ;;
    --base-test)
      BASE_TEST_REL="$2"; shift 2 ;;
    --keep-parallel-files)
      KEEP_PARALLEL_FILES="true"; shift 1 ;;
    --env)
      NW_ENV="$2"; shift 2 ;;
    --disable-voter-tracking)
      ENABLE_VOTER_TRACKING="false"; shift 1 ;;
    --previous-voters-file)
      PREVIOUS_VOTERS_FILE="$2"; shift 2 ;;
    *)
      echo "Unknown argument: $1" >&2; exit 1 ;;
  esac
done

# Resolve paths relative to this script so it works from anywhere
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NIGHTWATCH_DIR="$SCRIPT_DIR/nightwatch"
TESTS_DIR="$NIGHTWATCH_DIR/src"
BASE_TEST_PATH="$SCRIPT_DIR/$BASE_TEST_REL"
CONFIG_PATH="$NIGHTWATCH_DIR/nightwatch.conf.js"
LOG_DIR="$SCRIPT_DIR/logs"
mkdir -p "$LOG_DIR"

# Validation helpers
is_positive_int() { [[ "$1" =~ ^[1-9][0-9]*$ ]]; }

if ! is_positive_int "$NUM_BATCHES"; then
  echo "--batches must be a positive integer" >&2; exit 2
fi
if ! is_positive_int "$INSTANCES_PER_BATCH"; then
  echo "--instances must be a positive integer" >&2; exit 2
fi

# Sanity checks
if [[ ! -f "$BASE_TEST_PATH" ]]; then
  echo "Base test not found: $BASE_TEST_PATH" >&2
  echo "Tip: use --base-test to point to the correct test (default: $BASE_TEST_REL)" >&2
  exit 3
fi
if [[ ! -f "$CONFIG_PATH" ]]; then
  echo "Nightwatch config not found: $CONFIG_PATH" >&2; exit 3
fi

# Prepare a temporary folder with duplicated test files to enforce parallelism
RUN_ID="$(date +%Y%m%d_%H%M%S)_$$"
PARALLEL_DIR="$TESTS_DIR/_parallel_$RUN_ID"
mkdir -p "$PARALLEL_DIR"

cleanup() {
  if [[ "$KEEP_PARALLEL_FILES" != "true" ]]; then
    rm -rf "$PARALLEL_DIR" || true
  else
    echo "Keeping generated test files in: $PARALLEL_DIR"
  fi
}
trap cleanup EXIT

# Duplicate the base test N times
for i in $(seq 1 "$INSTANCES_PER_BATCH"); do
  cp -f "$BASE_TEST_PATH" "$PARALLEL_DIR/voting_instance_${i}.js"
done

# Export environment for the test
export NUMBER_OF_ITERATIONS="$NUM_BATCHES"
export VOTING_URL="$VOTING_URL"
export NUMBER_OF_VOTERS="$NUMBER_OF_VOTERS"
export USERNAME_PATTERN="$USERNAME_PATTERN"
export PASSWORD_PATTERN="$PASSWORD_PATTERN"
export SAVE_SCREENSHOTS="$SAVE_SCREENSHOTS"
export VOTER_MIN_INDEX="$VOTER_MIN_INDEX"

# Export PARALLEL_DIR so Node.js can access the shared voter tracking file
export PARALLEL_DIR="$PARALLEL_DIR"

# Export voter tracking configuration
export ENABLE_VOTER_TRACKING="$ENABLE_VOTER_TRACKING"
export CANDIDATES_PATTERN="$CANDIDATES_PATTERN"

if [[ "$ENABLE_VOTER_TRACKING" == "true" ]]; then
  # Initialize the shared used_voters file (defensive, though JS handles ENOENT)
  : > "$PARALLEL_DIR/used_voters.txt" || true
  
  # If previous voters file provided, import it to exclude those voters
  if [[ -n "$PREVIOUS_VOTERS_FILE" ]]; then
    if [[ -f "$PREVIOUS_VOTERS_FILE" ]]; then
      echo "Importing previously used voters from: $PREVIOUS_VOTERS_FILE"
      cat "$PREVIOUS_VOTERS_FILE" >> "$PARALLEL_DIR/used_voters.txt" || true
      IMPORTED_COUNT=$(wc -l < "$PREVIOUS_VOTERS_FILE" 2>/dev/null || echo "0")
      echo "Imported $IMPORTED_COUNT previously used voters"
    else
      echo "Warning: --previous-voters-file specified but file not found: $PREVIOUS_VOTERS_FILE" >&2
      echo "Continuing without importing previous voters..." >&2
    fi
  fi
else
  echo "Voter tracking disabled (--disable-voter-tracking)"
fi
# Log summary
LOG_FILE="$LOG_DIR/nightwatch_$RUN_ID.log"
echo "=== Nightwatch run $RUN_ID ===" | tee "$LOG_FILE"
echo "TEST_BASE:   $BASE_TEST_PATH" | tee -a "$LOG_FILE"
echo "INSTANCES:   $INSTANCES_PER_BATCH" | tee -a "$LOG_FILE"
echo "ITERATIONS:  $NUMBER_OF_ITERATIONS (from --batches)" | tee -a "$LOG_FILE"
echo "VOTING_URL:  $VOTING_URL" | tee -a "$LOG_FILE"
echo "NW_ENV:      $NW_ENV" | tee -a "$LOG_FILE"
echo "PARALLEL_DIR: $PARALLEL_DIR" | tee -a "$LOG_FILE"

# Run Nightwatch with explicit config and parallel workers.
# We "pushd" into the nightwatch directory so any relative paths (e.g., chromedriver)
# in the config resolve properly.
pushd "$NIGHTWATCH_DIR" >/dev/null
set +e
npx nightwatch --config "$CONFIG_PATH" \
  --env "$NW_ENV" \
  --parallel \
  --workers "$INSTANCES_PER_BATCH" \
  "$PARALLEL_DIR" | tee -a "$LOG_FILE"
EXIT_CODE=${PIPESTATUS[0]}
set -e
popd >/dev/null

if [[ $EXIT_CODE -ne 0 ]]; then
  echo "Nightwatch exited with code $EXIT_CODE" | tee -a "$LOG_FILE"
  exit $EXIT_CODE
fi

echo "✅ Completed Nightwatch run $RUN_ID" | tee -a "$LOG_FILE"
