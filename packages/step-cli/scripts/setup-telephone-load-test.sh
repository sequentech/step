#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Stage 1 of the telephone (IVR) load test: provisions an election event for
# a tenant, bulk-creates DTMF-safe voters, runs the keys ceremony, publishes,
# and opens the TELEPHONE voting channel. Writes a summary.json + voters CSV
# that Stage 2 (driving `ivr-cli` calls) consumes. See
# docs/docusaurus/docs/07-developers/12-ivr/telephone-load-testing-design.md
# for the full design and telephone-load-testing-guide.md for a walkthrough.
#
# Requires the `step-cli` binary on PATH (cd packages/step-cli && cargo build
# --release).

set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: setup-telephone-load-test.sh --election-event-json <path> [options]

Required:
  --election-event-json <path>   Election event JSON to import

Options (default to this repo's devcontainer dev tenant/Keycloak):
  --tenant-id <id>                Default: $SUPER_ADMIN_TENANT_ID
  --num-voters <n>                Default: 20
  --voter-pin-digits <n>          Numeric PIN length, max 8 (DTMF limit). Default: 6
  --voter-username-start <n>      First voter username (usernames increment from
                                  here). Default: 100, so every username is at
                                  least 3 digits
  --voter-area-name <name>        Every generated voter is placed in this single
                                  area, so all voters get the same contest count
                                  and one DTMF template works for every call.
                                  Default: the first area in the election event
  --threshold <n>                 Key ceremony trustee threshold. Default: 2
  --endpoint-url <url>            Hasura GraphQL endpoint. Default: $HASURA_ENDPOINT
  --keycloak-url <url>            Default: $KEYCLOAK_URL
  --keycloak-admin-user <user>    Default: $KEYCLOAK_ADMIN
  --keycloak-admin-password <pw>  Default: $KEYCLOAK_ADMIN
  --keycloak-client-id <id>       Default: api-key-client (needs "gold" acr
                                  for publish/voting-status; the devcontainer's
                                  $KEYCLOAK_CLI_CLIENT_ID is a lower tier and
                                  will 403 on those two steps)
  --keycloak-client-secret <s>    Default: this repo's devcontainer secret for
                                  api-key-client
  --trustee1-user <user>          Default: trustee1
  --trustee1-password <pw>        Default: trustee1
  --trustee2-user <user>          Default: trustee2
  --trustee2-password <pw>        Default: trustee2
  --out-dir <dir>                 Default: a fresh temp dir
  -h, --help                      Show this help
USAGE
}

TENANT_ID="${SUPER_ADMIN_TENANT_ID:-}"
ELECTION_EVENT_JSON=""
NUM_VOTERS=20
VOTER_PIN_DIGITS=6
VOTER_USERNAME_START=100
VOTER_AREA_NAME=""
THRESHOLD=2
ENDPOINT_URL="${HASURA_ENDPOINT:-}"
KEYCLOAK_URL="${KEYCLOAK_URL:-}"
KEYCLOAK_ADMIN_USER="${KEYCLOAK_ADMIN:-}"
KEYCLOAK_ADMIN_PASSWORD="${KEYCLOAK_ADMIN:-}"
# NOT $KEYCLOAK_CLI_CLIENT_ID: that client (admin-portal in this devcontainer)
# gets Keycloak's default "silver" acr on direct-grant login, and `publish` /
# `update-event-voting-status` require "gold" (sequent-core's
# has_gold_permission checks claims.acr == "gold"). api-key-client is the one
# client configured with `default.acr.values: gold`, matching what every CLI
# tutorial in docs/docusaurus hardcodes for this same reason.
KEYCLOAK_CLIENT_ID="api-key-client"
# Left empty by default: looked up live from Keycloak's admin API below,
# rather than hardcoded, since a client secret is per-environment and can be
# rotated independently of this script. NOT $KEYCLOAK_CLI_CLIENT_SECRET - that
# devcontainer env var is admin-portal's secret, a different client.
KEYCLOAK_CLIENT_SECRET=""
TRUSTEE1_USER="trustee1"
TRUSTEE1_PASSWORD="trustee1"
TRUSTEE2_USER="trustee2"
TRUSTEE2_PASSWORD="trustee2"
OUT_DIR=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --tenant-id) TENANT_ID="$2"; shift 2 ;;
    --election-event-json) ELECTION_EVENT_JSON="$2"; shift 2 ;;
    --num-voters) NUM_VOTERS="$2"; shift 2 ;;
    --voter-pin-digits) VOTER_PIN_DIGITS="$2"; shift 2 ;;
    --voter-username-start) VOTER_USERNAME_START="$2"; shift 2 ;;
    --voter-area-name) VOTER_AREA_NAME="$2"; shift 2 ;;
    --threshold) THRESHOLD="$2"; shift 2 ;;
    --endpoint-url) ENDPOINT_URL="$2"; shift 2 ;;
    --keycloak-url) KEYCLOAK_URL="$2"; shift 2 ;;
    --keycloak-admin-user) KEYCLOAK_ADMIN_USER="$2"; shift 2 ;;
    --keycloak-admin-password) KEYCLOAK_ADMIN_PASSWORD="$2"; shift 2 ;;
    --keycloak-client-id) KEYCLOAK_CLIENT_ID="$2"; shift 2 ;;
    --keycloak-client-secret) KEYCLOAK_CLIENT_SECRET="$2"; shift 2 ;;
    --trustee1-user) TRUSTEE1_USER="$2"; shift 2 ;;
    --trustee1-password) TRUSTEE1_PASSWORD="$2"; shift 2 ;;
    --trustee2-user) TRUSTEE2_USER="$2"; shift 2 ;;
    --trustee2-password) TRUSTEE2_PASSWORD="$2"; shift 2 ;;
    --out-dir) OUT_DIR="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $1" >&2; usage; exit 1 ;;
  esac
done

[[ -n "$ELECTION_EVENT_JSON" ]] || { echo "Error: --election-event-json is required" >&2; usage; exit 1; }
[[ -f "$ELECTION_EVENT_JSON" ]] || { echo "Error: no such file: $ELECTION_EVENT_JSON" >&2; exit 1; }
[[ -n "$TENANT_ID" ]] || { echo "Error: --tenant-id is required (or set \$SUPER_ADMIN_TENANT_ID)" >&2; exit 1; }
[[ -n "$ENDPOINT_URL" ]] || { echo "Error: --endpoint-url is required (or set \$HASURA_ENDPOINT)" >&2; exit 1; }
[[ -n "$KEYCLOAK_URL" ]] || { echo "Error: --keycloak-url is required (or set \$KEYCLOAK_URL)" >&2; exit 1; }
[[ -n "$KEYCLOAK_ADMIN_USER" ]] || { echo "Error: --keycloak-admin-user is required (or set \$KEYCLOAK_ADMIN)" >&2; exit 1; }
[[ -n "$KEYCLOAK_CLIENT_ID" ]] || { echo "Error: --keycloak-client-id is required (or set \$KEYCLOAK_CLI_CLIENT_ID)" >&2; exit 1; }
(( VOTER_PIN_DIGITS >= 1 && VOTER_PIN_DIGITS <= 8 )) || { echo "Error: --voter-pin-digits must be between 1 and 8 (DTMF voter auth limit)" >&2; exit 1; }
(( VOTER_USERNAME_START >= 0 )) || { echo "Error: --voter-username-start must be >= 0" >&2; exit 1; }
command -v step-cli >/dev/null 2>&1 || { echo "Error: step-cli not found on PATH. Build it: (cd packages/step-cli && cargo build --release)" >&2; exit 1; }
command -v jq >/dev/null 2>&1 || { echo "Error: jq is required (used to restrict voter generation to a single area)" >&2; exit 1; }

export NO_COLOR=1

log() { echo "==> $*" >&2; }

if [[ -z "$KEYCLOAK_CLIENT_SECRET" ]]; then
  log "Looking up $KEYCLOAK_CLIENT_ID's client secret from Keycloak (master realm admin API)"
  master_token="$(curl -sf -X POST "$KEYCLOAK_URL/realms/master/protocol/openid-connect/token" \
    -d grant_type=password -d client_id=admin-cli \
    -d "username=$KEYCLOAK_ADMIN_USER" -d "password=$KEYCLOAK_ADMIN_PASSWORD" \
    | jq -r '.access_token // empty')"
  [[ -n "$master_token" ]] || { echo "Error: could not obtain a master-realm admin token to look up $KEYCLOAK_CLIENT_ID's secret (pass --keycloak-client-secret explicitly)" >&2; exit 1; }
  KEYCLOAK_CLIENT_SECRET="$(curl -sf -H "Authorization: Bearer $master_token" \
    "$KEYCLOAK_URL/admin/realms/tenant-$TENANT_ID/clients?clientId=$KEYCLOAK_CLIENT_ID" \
    | jq -r '.[0].secret // empty')"
  [[ -n "$KEYCLOAK_CLIENT_SECRET" ]] || { echo "Error: could not look up $KEYCLOAK_CLIENT_ID's secret in tenant-$TENANT_ID (pass --keycloak-client-secret explicitly)" >&2; exit 1; }
fi

# step-cli always exits 0, even on failure (commands eprintln "Error! ..."
# and return); detect failure by scanning the captured output instead of $?.
run_step() {
  local out
  out="$(step-cli step "$@" 2>&1 | sed -E 's/\x1b\[[0-9;]*[a-zA-Z]//g')"
  echo "$out" >&2
  if grep -q '^Error!' <<<"$out"; then
    echo "==> step-cli step $* failed" >&2
    return 1
  fi
  printf '%s' "$out"
}

# The trustee containers complete the key ceremony asynchronously (polling
# the bulletin board on their own schedule, running an actual DKG protocol
# round), so `complete-key-ceremony` can legitimately 500 if called before a
# trustee has caught up to a just-started ceremony. Retry with backoff
# instead of treating the first failure as fatal.
retry_step() {
  local attempts="$1" delay="$2"
  shift 2
  local i=1
  while true; do
    if run_step "$@" >/dev/null; then
      return 0
    fi
    if (( i >= attempts )); then
      echo "==> step-cli step $* did not succeed after $attempts attempts" >&2
      return 1
    fi
    log "Retrying in ${delay}s (attempt $((i + 1))/$attempts)..."
    sleep "$delay"
    i=$((i + 1))
  done
}

# step-cli prints "Success! ... ID: <uuid>" (or, inconsistently, "ID <uuid>"
# with no colon) on the last line of a successful run; take the last ID-like
# token on the last such line.
extract_id() {
  grep -oE 'ID:? +[A-Za-z0-9._-]+' <<<"$1" | tail -1 | awk '{print $NF}'
}

configure_as() {
  local user="$1" password="$2"
  run_step config \
    --tenant-id "$TENANT_ID" \
    --endpoint-url "$ENDPOINT_URL" \
    --keycloak-url "$KEYCLOAK_URL" \
    --keycloak-user "$user" \
    --keycloak-password "$password" \
    --keycloak-client-id "$KEYCLOAK_CLIENT_ID" \
    --keycloak-client-secret "$KEYCLOAK_CLIENT_SECRET" >/dev/null
}

if [[ -z "$OUT_DIR" ]]; then
  OUT_DIR="$(mktemp -d "${TMPDIR:-/tmp}/telephone-load-test-XXXXXX")"
fi
mkdir -p "$OUT_DIR"
log "Writing outputs to $OUT_DIR"

log "[1/7] Authenticating as admin ($KEYCLOAK_ADMIN_USER)"
configure_as "$KEYCLOAK_ADMIN_USER" "$KEYCLOAK_ADMIN_PASSWORD"

log "[2/7] Importing election event from $ELECTION_EVENT_JSON"
# Append a random 5-char suffix to the election event's name (every language,
# so it stays visible regardless of the admin portal's UI language) - makes
# this run's election event easy to pick out in the admin portal's list,
# especially when several load-test runs exist at once.
run_suffix_chars="ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
RUN_SUFFIX=""
for _ in 1 2 3 4 5; do
  RUN_SUFFIX+="${run_suffix_chars:$((RANDOM % ${#run_suffix_chars})):1}"
done
jq --arg suffix "$RUN_SUFFIX" '
  .election_event.presentation.i18n |= with_entries(
    if .value.name then .value.name += " - " + $suffix else . end
  )
' "$ELECTION_EVENT_JSON" >"$OUT_DIR/election-event-to-import.json"
out="$(run_step import-election --file-path "$OUT_DIR/election-event-to-import.json" --is-local)"
ELECTION_EVENT_ID="$(extract_id "$out")"
[[ -n "$ELECTION_EVENT_ID" ]] || { echo "Error: could not parse election_event_id from import-election output" >&2; exit 1; }
ELECTION_EVENT_NAME="$(jq -r '
  .election_event.presentation.i18n as $i18n
  | ($i18n.en.name // ($i18n | to_entries | map(select(.value.name)) | .[0].value.name) // "Unknown")
' "$OUT_DIR/election-event-to-import.json")"
log "    election_event_id=$ELECTION_EVENT_ID"
log "    election_event_name=$ELECTION_EVENT_NAME"

log "[3/7] Generating $NUM_VOTERS voters with numeric, ${VOTER_PIN_DIGITS}-digit DTMF-safe credentials"
# dateOfBirth is required: this realm's IVR auth flow (checked live via its
# {realm}/ivr-config endpoint, not assumed from a generic default) resolves
# voters by dateOfBirth + PIN rather than by username + PIN, so a voter with
# no dateOfBirth attribute can never authenticate over a call. generate-voters
# writes it in the realm's expected YYYY-MM-DD form already.
#
# generate-voters round-robins voters across every area in the election event
# file it's given, and different areas can have different contest counts —
# which would mean different voters need different DTMF scripts. This copy is
# trimmed down to a single area (and only that area's area_contests) so every
# generated voter lands in the same area, keeping one DTMF template valid for
# every simulated call. This only affects voter generation, not the election
# itself: $ELECTION_EVENT_JSON (with all its areas) is what was already
# imported in step [2/7] above.
if [[ -z "$VOTER_AREA_NAME" ]]; then
  VOTER_AREA_NAME="$(jq -r '.areas[0].name' "$ELECTION_EVENT_JSON")"
  [[ -n "$VOTER_AREA_NAME" && "$VOTER_AREA_NAME" != "null" ]] || { echo "Error: $ELECTION_EVENT_JSON has no areas" >&2; exit 1; }
fi
jq --arg area "$VOTER_AREA_NAME" '
  ([.areas[] | select(.name == $area)] | .[0].id) as $area_id
  | if $area_id == null then error("no area named \($area)") else . end
  | .areas = [.areas[] | select(.name == $area)]
  | .area_contests = [.area_contests[] | select(.area_id == $area_id)]
' "$ELECTION_EVENT_JSON" >"$OUT_DIR/election-event.json" \
  || { echo "Error: --voter-area-name '$VOTER_AREA_NAME' does not match any area in $ELECTION_EVENT_JSON" >&2; exit 1; }
log "    voter_area=$VOTER_AREA_NAME"
cat >"$OUT_DIR/external_config.json" <<EXTCFG
{
  "election_event_json_file": "election-event.json",
  "realm_name": "tenant-${TENANT_ID}-event-${ELECTION_EVENT_ID}",
  "tenant_id": "${TENANT_ID}",
  "election_event_id": "${ELECTION_EVENT_ID}",
  "area_id": "",
  "election_id": "",
  "generate_voters": {
    "csv_file_name": "voters",
    "fields": ["username", "area_name", "password", "email", "email_verified", "dateOfBirth"],
    "excluded_columns": [],
    "email_prefix": "telephone-load-test",
    "domain": "example.invalid",
    "sequence_email_number": true,
    "sequence_start_number": 0,
    "username_start_number": ${VOTER_USERNAME_START},
    "voter_password": "",
    "voter_password_policy": {"type": "random-numeric", "digits": ${VOTER_PIN_DIGITS}},
    "password_salt": "",
    "hashed_password": "",
    "overseas_reference": "",
    "min_age": 18,
    "max_age": 90,
    "authorized_elections_count": 0,
    "email_verified": true
  },
  "duplicate_votes": {"row_id_to_clone": ""},
  "generate_applications": {"applicant_data": {}, "annotations": {}}
}
EXTCFG
run_step generate-voters --working-directory "$OUT_DIR" --num-users "$NUM_VOTERS" >/dev/null
VOTERS_CSV="$OUT_DIR/voters_${NUM_VOTERS}.csv"
[[ -f "$VOTERS_CSV" ]] || { echo "Error: expected voters CSV at $VOTERS_CSV, not found" >&2; exit 1; }
log "    voters_csv=$VOTERS_CSV"

log "[4/7] Bulk-importing voters into the election event"
run_step import-voters --election-event-id "$ELECTION_EVENT_ID" --file-path "$VOTERS_CSV" --is-local >/dev/null

log "[5/7] Starting the keys ceremony (threshold=$THRESHOLD)"
out="$(run_step start-key-ceremony --election-event-id "$ELECTION_EVENT_ID" --threshold "$THRESHOLD")"
KEY_CEREMONY_ID="$(extract_id "$out")"
[[ -n "$KEY_CEREMONY_ID" ]] || { echo "Error: could not parse key_ceremony_id from start-key-ceremony output" >&2; exit 1; }
log "    key_ceremony_id=$KEY_CEREMONY_ID"

log "[6/7] Completing the keys ceremony as $TRUSTEE1_USER, then $TRUSTEE2_USER"
configure_as "$TRUSTEE1_USER" "$TRUSTEE1_PASSWORD"
retry_step 30 5 complete-key-ceremony --election-event-id "$ELECTION_EVENT_ID" --key-ceremony-id "$KEY_CEREMONY_ID"
configure_as "$TRUSTEE2_USER" "$TRUSTEE2_PASSWORD"
retry_step 30 5 complete-key-ceremony --election-event-id "$ELECTION_EVENT_ID" --key-ceremony-id "$KEY_CEREMONY_ID"

log "[7/7] Publishing and opening the TELEPHONE voting channel"
configure_as "$KEYCLOAK_ADMIN_USER" "$KEYCLOAK_ADMIN_PASSWORD"
run_step publish --election-event-id "$ELECTION_EVENT_ID" >/dev/null
run_step update-event-voting-status --election-event-id "$ELECTION_EVENT_ID" --voting-status OPEN --voting-channel TELEPHONE >/dev/null

REALM_NAME="tenant-${TENANT_ID}-event-${ELECTION_EVENT_ID}"
cat >"$OUT_DIR/summary.json" <<SUMMARY
{
  "tenant_id": "${TENANT_ID}",
  "election_event_id": "${ELECTION_EVENT_ID}",
  "election_event_name": "${ELECTION_EVENT_NAME}",
  "keycloak_realm": "${REALM_NAME}",
  "keycloak_url": "${KEYCLOAK_URL}",
  "hasura_url": "${ENDPOINT_URL}",
  "voters_csv": "${VOTERS_CSV}",
  "num_voters": ${NUM_VOTERS},
  "voter_area": "${VOTER_AREA_NAME}"
}
SUMMARY

log "Done. Election event \"$ELECTION_EVENT_NAME\" ($ELECTION_EVENT_ID) is open for TELEPHONE voting."
log "Summary: $OUT_DIR/summary.json"
log "Voters (username,password are the DTMF voter-id/PIN): $VOTERS_CSV"
