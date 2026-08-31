#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Stage 2 of the telephone (IVR) load test: takes the outputs of Stage 1
# (setup-telephone-load-test.sh: a summary.json + voters CSV), generates a
# local phone_config.json and one DTMF input script per voter from a captured
# call template, then fans out N parallel `ivr-cli --bundle dev` processes —
# each an independent simulated phone call against the dev-container's real
# Keycloak/Hasura. See IVR_LOAD_TEST_DESIGN.md at the repo root.
#
# The DTMF template is captured empirically: run one interactive call by hand
# against the Stage-1 event and note every keystroke (see
# dtmf-template.example.txt next to this script for the procedure), replacing
# the voter-id/PIN entries with {{VOTER_ID}} / {{PIN}}.
#
# Requires the `ivr-cli` binary (cd beyond/packages && cargo build --release
# -p ivr-cli) and a Redis-compatible session store; if none is reachable this
# script starts a local `valkey` docker container (disable with
# --no-start-valkey).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

DEFAULT_SYSTEM_NUMBER="+111111111111"
VALKEY_CONTAINER_NAME="ivr-load-test-valkey"
VALKEY_IMAGE="valkey/valkey:8-alpine"
VALKEY_PORT=6379
# Printed by the BallotReceipt prompt only after a ballot has actually been
# cast (builtin_prompts.rs: "Your ballot locator for {election_name} is, ...").
DEFAULT_SUCCESS_REGEX="ballot locator"

usage() {
  cat <<'USAGE'
Usage: run-telephone-load-test.sh --run-dir <stage1-out-dir> --dtmf-template <file> [options]

Required:
  --run-dir <dir>              Stage 1 output directory (must contain
                                summary.json and the voters CSV it references)
  --dtmf-template <file>       DTMF input template with {{VOTER_ID}} and
                                {{PIN}} placeholders. Capture it with one
                                manual ivr-cli call — see
                                dtmf-template.example.txt for the procedure

Options:
  --concurrency <n>            Parallel calls. Default: 10
  --max-calls <n>              Cap on total calls. Default: every voter in the CSV
  --call-timeout <secs>        Per-call timeout. Default: 300
  --system-number <num>        Number the simulated callers dial (phone_config
                                key). Default: +111111111111
  --ivr-cli-bin <path>         Path to the ivr-cli binary. Default: ivr-cli on
                                PATH, else beyond/packages/{rust-local-target,
                                target}/release/ivr-cli
  --valkey-url <url>           Session store URL. Default: $VALKEY_URL, else
                                redis://127.0.0.1:6379
  --no-start-valkey            Never docker-run a local valkey; fail instead
                                if the session store is unreachable
  --success-regex <re>         Per-call log regex counted as a cast ballot.
                                Default: "ballot locator" (the receipt prompt)
  --out-dir <dir>              Where to write phone_config.json, per-call
                                inputs/logs and results.csv. Default: a fresh
                                temp dir
  -h, --help                   Show this help
USAGE
}

RUN_DIR=""
DTMF_TEMPLATE=""
CONCURRENCY=10
MAX_CALLS=""
CALL_TIMEOUT=300
SYSTEM_NUMBER="$DEFAULT_SYSTEM_NUMBER"
IVR_CLI_BIN=""
VALKEY_URL="${VALKEY_URL:-}"
START_VALKEY=1
SUCCESS_REGEX="$DEFAULT_SUCCESS_REGEX"
OUT_DIR=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --run-dir) RUN_DIR="$2"; shift 2 ;;
    --dtmf-template) DTMF_TEMPLATE="$2"; shift 2 ;;
    --concurrency) CONCURRENCY="$2"; shift 2 ;;
    --max-calls) MAX_CALLS="$2"; shift 2 ;;
    --call-timeout) CALL_TIMEOUT="$2"; shift 2 ;;
    --system-number) SYSTEM_NUMBER="$2"; shift 2 ;;
    --ivr-cli-bin) IVR_CLI_BIN="$2"; shift 2 ;;
    --valkey-url) VALKEY_URL="$2"; shift 2 ;;
    --no-start-valkey) START_VALKEY=0; shift ;;
    --success-regex) SUCCESS_REGEX="$2"; shift 2 ;;
    --out-dir) OUT_DIR="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $1" >&2; usage; exit 1 ;;
  esac
done

log() { echo "==> $*" >&2; }

[[ -n "$RUN_DIR" ]] || { echo "Error: --run-dir is required" >&2; usage; exit 1; }
SUMMARY_JSON="$RUN_DIR/summary.json"
[[ -f "$SUMMARY_JSON" ]] || { echo "Error: no summary.json in $RUN_DIR — run setup-telephone-load-test.sh first" >&2; exit 1; }
[[ -n "$DTMF_TEMPLATE" ]] || { echo "Error: --dtmf-template is required (see $SCRIPT_DIR/dtmf-template.example.txt for how to capture one)" >&2; usage; exit 1; }
[[ -f "$DTMF_TEMPLATE" ]] || { echo "Error: no such file: $DTMF_TEMPLATE" >&2; exit 1; }
grep -q '{{VOTER_ID}}' "$DTMF_TEMPLATE" || { echo "Error: template $DTMF_TEMPLATE has no {{VOTER_ID}} placeholder" >&2; exit 1; }
grep -q '{{PIN}}' "$DTMF_TEMPLATE" || { echo "Error: template $DTMF_TEMPLATE has no {{PIN}} placeholder" >&2; exit 1; }

# --- Locate the ivr-cli binary -----------------------------------------------

if [[ -z "$IVR_CLI_BIN" ]]; then
  for candidate in \
    "$(command -v ivr-cli 2>/dev/null || true)" \
    "$REPO_ROOT/beyond/packages/rust-local-target/release/ivr-cli" \
    "$REPO_ROOT/beyond/packages/target/release/ivr-cli"; do
    if [[ -n "$candidate" && -x "$candidate" ]]; then
      IVR_CLI_BIN="$candidate"
      break
    fi
  done
fi
[[ -n "$IVR_CLI_BIN" && -x "$IVR_CLI_BIN" ]] || {
  echo "Error: ivr-cli binary not found. Build it: (cd beyond/packages && cargo build --release -p ivr-cli), or pass --ivr-cli-bin" >&2
  exit 1
}
log "Using ivr-cli: $IVR_CLI_BIN"

# --- Read Stage 1's summary.json ---------------------------------------------

# summary.json is written by setup-telephone-load-test.sh with a known flat
# shape, so plain sed extraction is enough — no jq dependency.
json_get() {
  local key="$1"
  sed -n 's/.*"'"$key"'": *"\([^"]*\)".*/\1/p' "$SUMMARY_JSON" | head -1
}

TENANT_ID="$(json_get tenant_id)"
ELECTION_EVENT_ID="$(json_get election_event_id)"
KEYCLOAK_REALM="$(json_get keycloak_realm)"
KEYCLOAK_URL="$(json_get keycloak_url)"
HASURA_URL="$(json_get hasura_url)"
VOTERS_CSV="$(json_get voters_csv)"
for var in TENANT_ID ELECTION_EVENT_ID KEYCLOAK_REALM KEYCLOAK_URL HASURA_URL VOTERS_CSV; do
  [[ -n "${!var}" ]] || { echo "Error: could not read $var from $SUMMARY_JSON" >&2; exit 1; }
done
# Stage 1 writes voters_csv as an absolute path; tolerate a moved run dir.
[[ -f "$VOTERS_CSV" ]] || VOTERS_CSV="$RUN_DIR/$(basename "$VOTERS_CSV")"
[[ -f "$VOTERS_CSV" ]] || { echo "Error: voters CSV not found: $(json_get voters_csv)" >&2; exit 1; }
log "Election event $ELECTION_EVENT_ID (tenant $TENANT_ID), voters: $VOTERS_CSV"

# --- Keycloak IVR client secrets ---------------------------------------------

# Prefer already-exported values; fall back to the devcontainer's dev env file.
DEV_ENV_FILE="$REPO_ROOT/.devcontainer/.env.development"
env_file_get() {
  sed -n "s/^$1=//p" "$DEV_ENV_FILE" 2>/dev/null | head -1
}
KEYCLOAK_IVR_SERVICE_CLIENT_ID="${KEYCLOAK_IVR_SERVICE_CLIENT_ID:-$(env_file_get KEYCLOAK_IVR_SERVICE_CLIENT_ID)}"
KEYCLOAK_IVR_SERVICE_CLIENT_ID="${KEYCLOAK_IVR_SERVICE_CLIENT_ID:-ivr-service}"
KEYCLOAK_IVR_VOTING_CLIENT_ID="${KEYCLOAK_IVR_VOTING_CLIENT_ID:-ivr-voting}"
KEYCLOAK_IVR_SERVICE_CLIENT_SECRET="${KEYCLOAK_IVR_SERVICE_CLIENT_SECRET:-$(env_file_get KEYCLOAK_IVR_SERVICE_CLIENT_SECRET)}"
KEYCLOAK_IVR_VOTING_CLIENT_SECRET="${KEYCLOAK_IVR_VOTING_CLIENT_SECRET:-$(env_file_get KEYCLOAK_IVR_VOTING_CLIENT_SECRET)}"
[[ -n "$KEYCLOAK_IVR_SERVICE_CLIENT_SECRET" ]] || { echo "Error: KEYCLOAK_IVR_SERVICE_CLIENT_SECRET not set and not found in $DEV_ENV_FILE" >&2; exit 1; }
[[ -n "$KEYCLOAK_IVR_VOTING_CLIENT_SECRET" ]] || { echo "Error: KEYCLOAK_IVR_VOTING_CLIENT_SECRET not set and not found in $DEV_ENV_FILE" >&2; exit 1; }

# --- Session store (valkey) --------------------------------------------------

port_open() { (exec 3<>"/dev/tcp/$1/$2") 2>/dev/null && exec 3>&- && exec 3<&-; }

if [[ -z "$VALKEY_URL" ]]; then
  VALKEY_URL="redis://127.0.0.1:$VALKEY_PORT"
  if ! port_open 127.0.0.1 "$VALKEY_PORT"; then
    if (( START_VALKEY )) && command -v docker >/dev/null 2>&1; then
      log "No session store on 127.0.0.1:$VALKEY_PORT — starting $VALKEY_CONTAINER_NAME ($VALKEY_IMAGE)"
      docker rm -f "$VALKEY_CONTAINER_NAME" >/dev/null 2>&1 || true
      docker run -d --name "$VALKEY_CONTAINER_NAME" -p "$VALKEY_PORT:6379" "$VALKEY_IMAGE" >/dev/null
      for _ in $(seq 1 30); do
        port_open 127.0.0.1 "$VALKEY_PORT" && break
        sleep 1
      done
    fi
    port_open 127.0.0.1 "$VALKEY_PORT" || {
      echo "Error: no Redis-compatible session store reachable at $VALKEY_URL (ivr-cli's dev bundle needs one; rerun without --no-start-valkey, or pass --valkey-url)" >&2
      exit 1
    }
  fi
fi
log "Session store: $VALKEY_URL"

# --- Output layout -----------------------------------------------------------

if [[ -z "$OUT_DIR" ]]; then
  OUT_DIR="$(mktemp -d "${TMPDIR:-/tmp}/telephone-load-test-calls-XXXXXX")"
fi
mkdir -p "$OUT_DIR/inputs" "$OUT_DIR/logs"
log "Writing call inputs/logs to $OUT_DIR"

# --- phone_config.json -------------------------------------------------------

# Same shape as ivr-core's ports/phone_config.rs (see the fixture at
# adapters/mock/fixtures/phone_config.json). cluster_id/region/environment are
# required by the deserializer but unused outside AWS routing.
PHONE_CONFIG_PATH="$OUT_DIR/phone_config.json"
cat >"$PHONE_CONFIG_PATH" <<PHONECFG
{
  "entries": {
    "${SYSTEM_NUMBER}": {
      "tenant_id": "${TENANT_ID}",
      "election_event_id": "${ELECTION_EVENT_ID}",
      "keycloak_realm": "${KEYCLOAK_REALM}",
      "cluster_id": "dev",
      "region": "local",
      "environment": "dev",
      "keycloak_url": "${KEYCLOAK_URL}",
      "hasura_url": "${HASURA_URL}",
      "default_language": "en",
      "enabled": true
    }
  }
}
PHONECFG
log "Generated $PHONE_CONFIG_PATH"

# --- Per-voter DTMF input files ----------------------------------------------

# The voters CSV comes from `step-cli step generate-voters` (header row, no
# quoting on the numeric username/password columns).
header="$(head -1 "$VOTERS_CSV")"
username_col="$(awk -F, -v RS='' '{for (i=1;i<=NF;i++) if ($i=="username") {print i; exit}}' <<<"$header")"
password_col="$(awk -F, -v RS='' '{for (i=1;i<=NF;i++) if ($i=="password") {print i; exit}}' <<<"$header")"
[[ -n "$username_col" && -n "$password_col" ]] || { echo "Error: $VOTERS_CSV has no username/password columns (header: $header)" >&2; exit 1; }

count=0
while IFS=, read -r -a row; do
  voter_id="${row[$((username_col - 1))]}"
  pin="${row[$((password_col - 1))]}"
  [[ -n "$voter_id" && -n "$pin" ]] || continue
  sed -e "s/{{VOTER_ID}}/$voter_id/g" -e "s/{{PIN}}/$pin/g" \
    "$DTMF_TEMPLATE" >"$OUT_DIR/inputs/call-$voter_id.txt"
  count=$((count + 1))
  if [[ -n "$MAX_CALLS" ]] && (( count >= MAX_CALLS )); then
    break
  fi
done < <(tail -n +2 "$VOTERS_CSV")
(( count > 0 )) || { echo "Error: no voter rows found in $VOTERS_CSV" >&2; exit 1; }
log "Rendered $count DTMF input files"

# --- Fan out the calls -------------------------------------------------------

export PHONE_CONFIG_PATH VALKEY_URL \
  KEYCLOAK_IVR_SERVICE_CLIENT_ID KEYCLOAK_IVR_SERVICE_CLIENT_SECRET \
  KEYCLOAK_IVR_VOTING_CLIENT_ID KEYCLOAK_IVR_VOTING_CLIENT_SECRET \
  IVR_CLI_BIN SYSTEM_NUMBER OUT_DIR CALL_TIMEOUT
export RUST_LOG="${RUST_LOG:-info}"

run_one_call() {
  local input_file="$1"
  local voter_id caller log_file rc
  voter_id="$(basename "$input_file" .txt)"
  voter_id="${voter_id#call-}"
  # Unique fake ANI per call; the caller's number is only blacklist-checked,
  # never used for auth, so any well-formed unique number works.
  caller="$(printf '+1555%07d' "$voter_id")"
  log_file="$OUT_DIR/logs/call-$voter_id.log"
  if timeout "$CALL_TIMEOUT" "$IVR_CLI_BIN" \
    --bundle dev \
    --system-number "$SYSTEM_NUMBER" \
    --number "$caller" \
    --input-file "$input_file" >"$log_file" 2>&1; then
    rc=0
  else
    rc=$?
  fi
  echo "$voter_id,$rc" >>"$OUT_DIR/exit_codes.csv"
}
export -f run_one_call

log "Placing $count calls with concurrency $CONCURRENCY (timeout ${CALL_TIMEOUT}s each)"
: >"$OUT_DIR/exit_codes.csv"
start_ts="$(date +%s)"
find "$OUT_DIR/inputs" -name 'call-*.txt' -print0 | sort -z |
  xargs -0 -P "$CONCURRENCY" -I{} bash -c 'run_one_call "$1"' _ {}
elapsed=$(( $(date +%s) - start_ts ))

# --- Results -----------------------------------------------------------------

RESULTS_CSV="$OUT_DIR/results.csv"
echo "voter_id,exit_code,ballot_cast" >"$RESULTS_CSV"
cast=0
failed=0
while IFS=, read -r voter_id rc; do
  if grep -qi "$SUCCESS_REGEX" "$OUT_DIR/logs/call-$voter_id.log" 2>/dev/null; then
    echo "$voter_id,$rc,true" >>"$RESULTS_CSV"
    cast=$((cast + 1))
  else
    echo "$voter_id,$rc,false" >>"$RESULTS_CSV"
    failed=$((failed + 1))
  fi
done <"$OUT_DIR/exit_codes.csv"

log "Done in ${elapsed}s: $cast/$count calls cast a ballot ($failed did not)"
log "Per-call results: $RESULTS_CSV"
log "Per-call logs:    $OUT_DIR/logs/"
if (( failed > 0 )); then
  log "Inspect a failed call's log for where the flow diverged from the template (searched for /$SUCCESS_REGEX/i as the cast marker)"
  exit 1
fi
