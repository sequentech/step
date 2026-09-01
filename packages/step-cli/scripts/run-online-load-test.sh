#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Stage 2 of the ONLINE (voting portal) load test: takes the outputs of
# Stage 1 (setup-telephone-load-test.sh --voting-channel ONLINE: a
# summary.json + voters CSV), renders a voter manifest, then drives one
# Playwright run in packages/voting-portal where each voter is a real
# headless-browser session going through login, election selection, candidate
# selection, review, cast and confirmation — so all portal overhead (Keycloak
# auth, GraphQL, ballot styles, WASM ballot encryption) is exercised for
# real. Concurrency is delegated to Playwright workers (one process, one
# reused browser per worker) rather than fanning out separate processes,
# because browsers are expensive. See
# docs/docusaurus/docs/07-developers/02-cli/02-tutorials/load-testing/online-load-testing-design.md.
#
# Requires Node dependencies installed (`yarn` from packages/) and the
# Playwright Chromium browser (`yarn --cwd packages/voting-portal playwright
# install chromium`), plus `jq`.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
VOTING_PORTAL_DIR="$REPO_ROOT/packages/voting-portal"

usage() {
  cat <<'USAGE'
Usage: run-online-load-test.sh --run-dir <stage1-out-dir> [options]

Required:
  --run-dir <dir>              Stage 1 output directory (must contain
                                summary.json and the voters CSV it references),
                                provisioned with --voting-channel ONLINE

Options:
  --concurrency <n>            Parallel voting clients (Playwright workers).
                               Each is a real Chromium page doing WASM ballot
                               encryption — budget roughly 0.5GB RAM and one
                               core per client, on top of the compose stack.
                               Default: 4
  --max-votes <n>              Cap on total votes. Default: every voter in the CSV
  --voter-offset <n>           Skip the first N voters in the CSV. For
                               distributed runs: give each load machine a
                               disjoint slice of one Stage-1 voter set via
                               --voter-offset/--max-votes, so no two machines
                               re-use (and duplicate-vote) the same voters.
                               Default: 0
  --vote-timeout <secs>        Per-voter end-to-end timeout. Default: 180
  --voting-portal-url <url>    Override summary.json's voting_portal_url —
                               needed when Stage 1 recorded a URL (e.g. the
                               devcontainer's 127.0.0.1) that this machine
                               cannot reach
  --keycloak-url <url>         Override summary.json's keycloak_url (same
                               distributed-run reason; only used for the
                               preflight reachability check — the browser
                               follows the portal's own Keycloak redirects)
  --hasura-url <url>           Override summary.json's hasura_url (preflight
                               check only, same reason)
  --candidates-pattern <re>    Regular expression filtering which candidates
                               may be selected, by visible name
  --headed                     Run browsers headed for debugging (forces
                               --concurrency 1)
  --out-dir <dir>              Where to write the voter manifest, Playwright
                               report/traces, results.csv and summary.json.
                               Default: a fresh temp dir
  -h, --help                   Show this help
USAGE
}

RUN_DIR=""
CONCURRENCY=4
MAX_VOTES=""
VOTER_OFFSET=0
VOTE_TIMEOUT=180
VOTING_PORTAL_URL_OVERRIDE=""
KEYCLOAK_URL_OVERRIDE=""
HASURA_URL_OVERRIDE=""
CANDIDATES_PATTERN=""
HEADED=0
OUT_DIR=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --run-dir) RUN_DIR="$2"; shift 2 ;;
    --concurrency) CONCURRENCY="$2"; shift 2 ;;
    --max-votes) MAX_VOTES="$2"; shift 2 ;;
    --voter-offset) VOTER_OFFSET="$2"; shift 2 ;;
    --vote-timeout) VOTE_TIMEOUT="$2"; shift 2 ;;
    --voting-portal-url) VOTING_PORTAL_URL_OVERRIDE="$2"; shift 2 ;;
    --keycloak-url) KEYCLOAK_URL_OVERRIDE="$2"; shift 2 ;;
    --hasura-url) HASURA_URL_OVERRIDE="$2"; shift 2 ;;
    --candidates-pattern) CANDIDATES_PATTERN="$2"; shift 2 ;;
    --headed) HEADED=1; shift ;;
    --out-dir) OUT_DIR="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $1" >&2; usage; exit 1 ;;
  esac
done

log() { echo "==> $*" >&2; }

[[ -n "$RUN_DIR" ]] || { echo "Error: --run-dir is required" >&2; usage; exit 1; }
SUMMARY_JSON="$RUN_DIR/summary.json"
[[ -f "$SUMMARY_JSON" ]] || { echo "Error: no summary.json in $RUN_DIR — run setup-telephone-load-test.sh --voting-channel ONLINE first" >&2; exit 1; }
[[ "$VOTER_OFFSET" =~ ^[0-9]+$ ]] || { echo "Error: --voter-offset must be a non-negative integer" >&2; exit 1; }
[[ "$CONCURRENCY" =~ ^[1-9][0-9]*$ ]] || { echo "Error: --concurrency must be a positive integer" >&2; exit 1; }
command -v jq >/dev/null 2>&1 || { echo "Error: jq is required (used to render the voter manifest and parse the Playwright report)" >&2; exit 1; }

if (( HEADED )) && (( CONCURRENCY != 1 )); then
  log "--headed is a debugging mode; forcing --concurrency 1"
  CONCURRENCY=1
fi

# --- Read Stage 1's summary.json ---------------------------------------------

# summary.json is written by setup-telephone-load-test.sh with a known flat
# shape, so plain sed extraction is enough — no jq needed for this part.
json_get() {
  local key="$1"
  sed -n 's/.*"'"$key"'": *"\([^"]*\)".*/\1/p' "$SUMMARY_JSON" | head -1
}

TENANT_ID="$(json_get tenant_id)"
ELECTION_EVENT_ID="$(json_get election_event_id)"
KEYCLOAK_REALM="$(json_get keycloak_realm)"
KEYCLOAK_URL="${KEYCLOAK_URL_OVERRIDE:-$(json_get keycloak_url)}"
HASURA_URL="${HASURA_URL_OVERRIDE:-$(json_get hasura_url)}"
HASURA_URL="${HASURA_URL%/v1/graphql}"
VOTERS_CSV="$(json_get voters_csv)"
VOTING_CHANNEL="$(json_get voting_channel)"
VOTING_PORTAL_URL="${VOTING_PORTAL_URL_OVERRIDE:-$(json_get voting_portal_url)}"
for var in TENANT_ID ELECTION_EVENT_ID KEYCLOAK_REALM KEYCLOAK_URL HASURA_URL VOTERS_CSV; do
  [[ -n "${!var}" ]] || { echo "Error: could not read $var from $SUMMARY_JSON" >&2; exit 1; }
done
if [[ "$VOTING_CHANNEL" != "ONLINE" ]]; then
  echo "Error: $SUMMARY_JSON was provisioned for the ${VOTING_CHANNEL:-TELEPHONE} channel — re-run setup-telephone-load-test.sh with --voting-channel ONLINE (the portal's eligibility check gates on the ONLINE channel being open)" >&2
  exit 1
fi
[[ -n "$VOTING_PORTAL_URL" ]] || { echo "Error: could not read voting_portal_url from $SUMMARY_JSON (pass --voting-portal-url)" >&2; exit 1; }
VOTING_PORTAL_URL="${VOTING_PORTAL_URL%/}"
LOGIN_URL="$VOTING_PORTAL_URL/tenant/$TENANT_ID/event/$ELECTION_EVENT_ID/login"
# Stage 1 writes voters_csv as an absolute path; tolerate a moved/copied run
# dir (e.g. rsync'ed to another load machine).
[[ -f "$VOTERS_CSV" ]] || VOTERS_CSV="$RUN_DIR/$(basename "$VOTERS_CSV")"
[[ -f "$VOTERS_CSV" ]] || { echo "Error: voters CSV not found: $(json_get voters_csv)" >&2; exit 1; }
log "Election event $ELECTION_EVENT_ID (tenant $TENANT_ID), voters: $VOTERS_CSV"
log "Login URL: $LOGIN_URL"

# --- Preflight: every dependency checked before any load is generated --------

if ! curl -sf -o /dev/null --max-time 10 "$VOTING_PORTAL_URL/"; then
  echo "Error: voting portal not reachable at $VOTING_PORTAL_URL — start it with 'yarn start:voting-portal' (or pass --voting-portal-url if it runs elsewhere)" >&2
  exit 1
fi
if ! curl -sf -o /dev/null --max-time 10 "$KEYCLOAK_URL/realms/$KEYCLOAK_REALM/.well-known/openid-configuration"; then
  if curl -sf -o /dev/null --max-time 10 "$KEYCLOAK_URL/realms/master/.well-known/openid-configuration"; then
    echo "Error: Keycloak is up at $KEYCLOAK_URL but realm $KEYCLOAK_REALM does not exist — was the election event deleted, or is $SUMMARY_JSON stale?" >&2
  else
    echo "Error: Keycloak not reachable at $KEYCLOAK_URL — is the stack running? (pass --keycloak-url to override summary.json's URL on another machine)" >&2
  fi
  exit 1
fi
if ! curl -sf -o /dev/null --max-time 10 "$HASURA_URL/healthz"; then
  echo "Error: Hasura not reachable at $HASURA_URL — is the stack running? (pass --hasura-url to override summary.json's URL on another machine)" >&2
  exit 1
fi

PLAYWRIGHT_BIN=""
for candidate in \
  "$VOTING_PORTAL_DIR/node_modules/.bin/playwright" \
  "$REPO_ROOT/packages/node_modules/.bin/playwright"; do
  if [[ -x "$candidate" ]]; then
    PLAYWRIGHT_BIN="$candidate"
    break
  fi
done
[[ -n "$PLAYWRIGHT_BIN" ]] || {
  echo "Error: Playwright not installed. Install JS dependencies first: (cd packages && yarn)" >&2
  exit 1
}
if [[ -n "${IN_NIX_SHELL:-}" && -z "${PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS:-}" ]]; then
  # Playwright's own ldd-based dependency check runs against devenv's nix
  # glibc, which doesn't see this system's actual runtime libraries — a false
  # positive that blocks every launch even though Chromium runs fine. Skip
  # it; the chromium-install check just below still catches a genuinely
  # missing browser.
  export PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS=1
fi
# Best-effort browser check against Playwright's default cache location; a
# custom PLAYWRIGHT_BROWSERS_PATH is honored, and a false negative still
# fails later with Playwright's own (equally actionable) error.
BROWSERS_PATH="${PLAYWRIGHT_BROWSERS_PATH:-$HOME/.cache/ms-playwright}"
shopt -s nullglob
chromium_dirs=("$BROWSERS_PATH"/chromium*)
shopt -u nullglob
if [[ ${#chromium_dirs[@]} -eq 0 ]]; then
  echo "Error: no Playwright Chromium found under $BROWSERS_PATH — install it: (cd packages/voting-portal && yarn playwright install chromium)" >&2
  exit 1
fi
log "Using Playwright: $PLAYWRIGHT_BIN"

if command -v nproc >/dev/null 2>&1; then
  cores="$(nproc)"
  if (( CONCURRENCY > cores - 2 )); then
    log "WARNING: --concurrency $CONCURRENCY exceeds $((cores > 2 ? cores - 2 : 1)) (cores - 2 on this machine); ballot encryption is CPU-bound and the compose stack shares these cores — expect queueing to skew latency numbers"
  fi
fi

# --- Local port bridging (devcontainer-only) ----------------------------------
#
# The voting portal's login flow sends the BROWSER to Keycloak/MinIO URLs
# baked in for the developer's own OS browser (reachable via VS Code's
# forwardPorts, e.g. http://localhost:8090) — a headless browser launched
# from *inside* the devcontainer can't reach those the same way, since
# "localhost" there is the devcontainer's own loopback, not the host's.
# Bridge the ports this flow is known to need with socat for the run,
# skipping any that already resolve (e.g. a distributed run, or a
# devcontainer with host networking where this is unnecessary). Content the
# browser may also fetch straight from MinIO (e.g. candidate photos) is not
# covered here.
FORWARD_PIDS=()
maybe_forward_local_port() {
  local port="$1" target="$2"
  if curl -sf -o /dev/null --max-time 2 "http://127.0.0.1:$port/" 2>/dev/null; then
    return
  fi
  command -v socat >/dev/null 2>&1 || {
    log "WARNING: 127.0.0.1:$port is not reachable from inside this container, and socat is not installed to bridge it to $target — the browser may fail to reach it"
    return
  }
  log "Bridging 127.0.0.1:$port -> $target for the browser (socat)"
  socat "TCP-LISTEN:$port,fork,reuseaddr" "TCP:$target" &
  FORWARD_PIDS+=("$!")
}
if [[ "$VOTING_PORTAL_URL" == "http://localhost:"* || "$VOTING_PORTAL_URL" == "http://127.0.0.1:"* ]]; then
  keycloak_hostport="${KEYCLOAK_URL#http://}"
  hasura_hostport="${HASURA_URL#http://}"
  maybe_forward_local_port "${keycloak_hostport##*:}" "$keycloak_hostport"
  maybe_forward_local_port "${hasura_hostport##*:}" "$hasura_hostport"
  maybe_forward_local_port 9002 "minio-proxy:9002"
fi
trap '(( ${#FORWARD_PIDS[@]} )) && kill "${FORWARD_PIDS[@]}" 2>/dev/null; true' EXIT

# --- Output layout -----------------------------------------------------------

if [[ -z "$OUT_DIR" ]]; then
  OUT_DIR="$(mktemp -d "${TMPDIR:-/tmp}/online-load-test-votes-XXXXXX")"
fi
mkdir -p "$OUT_DIR"
OUT_DIR="$(cd "$OUT_DIR" && pwd)"
log "Writing manifest/report/results to $OUT_DIR"

# --- Voter manifest ----------------------------------------------------------

header="$(head -1 "$VOTERS_CSV")"
username_col="$(awk -F, -v RS='' '{for (i=1;i<=NF;i++) if ($i=="username") {print i; exit}}' <<<"$header")"
password_col="$(awk -F, -v RS='' '{for (i=1;i<=NF;i++) if ($i=="password") {print i; exit}}' <<<"$header")"
[[ -n "$username_col" && -n "$password_col" ]] || { echo "Error: $VOTERS_CSV has no username/password columns (header: $header)" >&2; exit 1; }

MANIFEST_JSON="$OUT_DIR/voters.json"
# Every CSV column is passed through (not just username/password): the login
# form's fields depend on the realm's configured match-attributes (e.g.
# dateOfBirth alongside — or instead of — a voter-id username), and the CSV
# columns are named to match those fields, so the whole row is what the
# browser flow needs to fill in.
{ echo "$header";
  tail -n +2 "$VOTERS_CSV" \
    | tail -n +"$((VOTER_OFFSET + 1))" \
    | { if [[ -n "$MAX_VOTES" ]]; then head -n "$MAX_VOTES"; else cat; fi; }; } \
  | jq -R -s --arg u "$username_col" --arg p "$password_col" '
      split("\n") | map(select(length > 0) | split(","))
      | .[0] as $header
      | .[1:]
      | map(select((.[($u|tonumber)-1] // "") != "" and (.[($p|tonumber)-1] // "") != ""))
      | map([$header, .] | transpose | map({(.[0]): .[1]}) | add)
    ' \
  >"$MANIFEST_JSON"
count="$(jq 'length' "$MANIFEST_JSON")"
(( count > 0 )) || { echo "Error: no voter rows found in $VOTERS_CSV after skipping --voter-offset $VOTER_OFFSET" >&2; exit 1; }
log "Rendered voter manifest with $count voters"

# --- Drive the browsers ------------------------------------------------------

REPORT_JSON="$OUT_DIR/playwright-report.json"
CAST_CSV="$OUT_DIR/cast_ballots.csv"
: >"$CAST_CSV"

export LOGIN_URL
export LOAD_TEST_MANIFEST="$MANIFEST_JSON"
export LOAD_TEST_CAST_CSV="$CAST_CSV"
export LOAD_TEST_OUT_DIR="$OUT_DIR"
export LOAD_TEST_VOTE_TIMEOUT_MS="$((VOTE_TIMEOUT * 1000))"
export CANDIDATES_PATTERN
(( HEADED )) && export LOAD_TEST_HEADED=true
export PLAYWRIGHT_JSON_OUTPUT_NAME="$REPORT_JSON"

log "Casting $count votes with concurrency $CONCURRENCY (timeout ${VOTE_TIMEOUT}s each)"
started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
start_ts="$(date +%s)"
set +e
(cd "$VOTING_PORTAL_DIR" && "$PLAYWRIGHT_BIN" test \
  --config playwright.load.config.ts \
  --workers "$CONCURRENCY" \
  --reporter=json,line)
pw_exit=$?
set -e
elapsed=$(( $(date +%s) - start_ts ))
finished_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
[[ -f "$REPORT_JSON" ]] || {
  echo "Error: Playwright exited with $pw_exit without writing $REPORT_JSON — the run crashed before any result was produced" >&2
  exit 1
}

# --- Results -----------------------------------------------------------------

# Per-voter status/duration come from the Playwright JSON report (which also
# covers voters killed by the per-test timeout); ballot ids come from the
# rows the spec appends to cast_ballots.csv on success.
declare -A ballot_ids
while IFS=, read -r voter_id _duration ids; do
  [[ -n "$voter_id" ]] && ballot_ids["$voter_id"]="$ids"
done <"$CAST_CSV"

RESULTS_CSV="$OUT_DIR/results.csv"
echo "voter_id,status,duration_ms,ballot_id" >"$RESULTS_CSV"
cast=0
failed=0
while IFS=, read -r voter_id status duration; do
  [[ -n "$voter_id" ]] || continue
  if [[ "$status" == "passed" ]]; then
    cast=$((cast + 1))
  else
    failed=$((failed + 1))
  fi
  echo "$voter_id,$status,$duration,${ballot_ids[$voter_id]:-}" >>"$RESULTS_CSV"
done < <(jq -r '
  [.suites[]? | recurse(.suites[]?) | .specs[]?]
  | .[]
  | [ (.title | sub("^voter "; "")),
      (.tests[0].results[0].status // "unknown"),
      ((.tests[0].results[0].duration // 0) | round) ]
  | @csv' "$REPORT_JSON" | tr -d '"')
(( cast + failed > 0 )) || {
  echo "Error: the Playwright run (exit $pw_exit) produced no per-voter results — it failed before any test ran; check the output above and $REPORT_JSON" >&2
  exit 1
}
if (( cast + failed != count )); then
  log "WARNING: expected $count voters but the report covers $((cast + failed))"
fi

SUMMARY_OUT="$OUT_DIR/summary.json"
cat >"$SUMMARY_OUT" <<SUMMARY
{
  "run_dir": "${RUN_DIR}",
  "election_event_id": "${ELECTION_EVENT_ID}",
  "tenant_id": "${TENANT_ID}",
  "login_url": "${LOGIN_URL}",
  "concurrency": ${CONCURRENCY},
  "voter_offset": ${VOTER_OFFSET},
  "vote_timeout_secs": ${VOTE_TIMEOUT},
  "started_at": "${started_at}",
  "finished_at": "${finished_at}",
  "elapsed_secs": ${elapsed},
  "total_votes": ${count},
  "cast": ${cast},
  "failed": ${failed},
  "results_csv": "${RESULTS_CSV}",
  "traces_dir": "${OUT_DIR}/traces"
}
SUMMARY

log "Done in ${elapsed}s: $cast/$count voters cast a ballot ($failed did not)"
log "Per-voter results: $RESULTS_CSV"
log "Failure traces:    $OUT_DIR/traces/ (open with: yarn --cwd packages/voting-portal playwright show-trace <trace.zip>)"
log "Run summary:       $SUMMARY_OUT"
if (( failed > 0 )); then
  exit 1
fi
