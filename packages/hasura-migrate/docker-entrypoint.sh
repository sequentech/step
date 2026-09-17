#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
#
# Equivalent of gitops hasura_apply_migration.yml apply steps, for in-cluster use:
#   1) wait for Hasura /healthz
#   2) optional: ensure Postgres source backend-db via metadata API (first deploy)
#   3) substitute {{HARVEST_DOMAIN}} in metadata/actions.yaml
#   4) hasura migrate apply --up all
#   5) hasura metadata apply
#   6) hasura migrate status
set -euo pipefail

log() { echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] $*"; }

: "${HASURA_GRAPHQL_ENDPOINT:?HASURA_GRAPHQL_ENDPOINT is required (e.g. http://hasura:8080)}"
: "${HASURA_GRAPHQL_ADMIN_SECRET:?HASURA_GRAPHQL_ADMIN_SECRET is required}"

ENDPOINT="${HASURA_GRAPHQL_ENDPOINT%/}"
ADMIN_SECRET="${HASURA_GRAPHQL_ADMIN_SECRET}"
DB_NAME="${HASURA_DATABASE_NAME:-backend-db}"
HARVEST_DOMAIN="${HARVEST_DOMAIN:-harvest:8400}"
ENSURE_PG_SOURCE="${ENSURE_PG_SOURCE:-true}"
PG_DATABASE_URL_ENV="${PG_DATABASE_URL_ENV:-PG_DATABASE_URL}"
WAIT_TIMEOUT_SECONDS="${WAIT_TIMEOUT_SECONDS:-300}"

METADATA_URL="${ENDPOINT}/v1/metadata"
HEALTH_URL="${ENDPOINT}/healthz"

hasura_meta() {
  local payload="$1"
  curl -sf -m 60 -X POST "${METADATA_URL}" \
    -H "content-type: application/json" \
    -H "x-hasura-admin-secret: ${ADMIN_SECRET}" \
    --data-raw "${payload}"
}

wait_for_hasura() {
  local deadline=$((SECONDS + WAIT_TIMEOUT_SECONDS))
  local n=0
  log "Waiting for Hasura at ${HEALTH_URL} (timeout ${WAIT_TIMEOUT_SECONDS}s)..."
  while (( SECONDS < deadline )); do
    n=$((n + 1))
    code="$(curl -s -o /dev/null -w '%{http_code}' -m 5 "${HEALTH_URL}" || echo 000)"
    log "  health attempt ${n}: HTTP ${code}"
    if [[ "${code}" == "200" ]]; then
      log "Hasura is healthy"
      return 0
    fi
    sleep 5
  done
  log "ERROR: Hasura did not become healthy in time"
  return 1
}

ensure_pg_source() {
  log "Ensuring Postgres source '${DB_NAME}' (from_env=${PG_DATABASE_URL_ENV})..."
  local check_payload add_payload resp
  check_payload="$(jq -nc --arg s "$DB_NAME" '{type:"pg_get_source_tables",args:{source:$s}}')"
  resp="$(curl -s -m 30 -X POST "${METADATA_URL}" \
    -H "content-type: application/json" \
    -H "x-hasura-admin-secret: ${ADMIN_SECRET}" \
    --data-raw "${check_payload}" || true)"

  if echo "${resp}" | jq -e 'type == "array"' >/dev/null 2>&1; then
    log "Source '${DB_NAME}' already connected"
    return 0
  fi

  add_payload="$(jq -nc \
    --arg name "$DB_NAME" \
    --arg env "$PG_DATABASE_URL_ENV" \
    '{
      type: "pg_add_source",
      args: {
        name: $name,
        configuration: {
          connection_info: {
            database_url: { from_env: $env },
            pool_settings: { max_connections: 50, idle_timeout: 180 }
          }
        }
      }
    }')"

  local attempt
  for attempt in 1 2 3 4 5; do
    resp="$(curl -s -m 30 -X POST "${METADATA_URL}" \
      -H "content-type: application/json" \
      -H "x-hasura-admin-secret: ${ADMIN_SECRET}" \
      --data-raw "${add_payload}" || true)"
    if echo "${resp}" | grep -qi 'already exists'; then
      log "Source '${DB_NAME}' already exists"
      return 0
    fi
    if echo "${resp}" | jq -e '(.message? == "success") or (type == "object" and (has("error")|not) and (has("errors")|not))' >/dev/null 2>&1; then
      # empty object {} is success for many metadata APIs
      if echo "${resp}" | jq -e 'has("error") or has("errors")' >/dev/null 2>&1; then
        :
      else
        log "Connected source '${DB_NAME}'"
        return 0
      fi
    fi
    # success often returns {}
    if [[ "${resp}" == "{}" || "${resp}" == "" ]]; then
      log "Connected source '${DB_NAME}'"
      return 0
    fi
    log "  pg_add_source attempt ${attempt} response: ${resp:0:200}"
    sleep 5
  done
  log "ERROR: failed to add source '${DB_NAME}': ${resp:0:500}"
  return 1
}

prepare_metadata() {
  log "Substituting HARVEST_DOMAIN=${HARVEST_DOMAIN} in metadata/actions.yaml"
  # Match GHA: sed "s|{{HARVEST_DOMAIN}}|...|g"
  sed -i "s|{{HARVEST_DOMAIN}}|${HARVEST_DOMAIN}|g" /hasura/metadata/actions.yaml

  # Point CLI config at in-cluster endpoint (same as GHA yq on config.yaml)
  # Keep handler_webhook_baseurl as endpoint for consistency with workflow
  if command -v hasura >/dev/null; then
    :
  fi
  # config.yaml endpoint is overridden by --endpoint flags; still set for local tooling
  sed -i "s|^endpoint:.*|endpoint: ${ENDPOINT}|" /hasura/config.yaml || true
}

apply_migrations() {
  log "Applying migrations (database=${DB_NAME})..."
  cd /hasura
  hasura migrate apply \
    --endpoint "${ENDPOINT}" \
    --admin-secret "${ADMIN_SECRET}" \
    --database-name "${DB_NAME}" \
    --up all

  log "Applying metadata..."
  hasura metadata apply \
    --endpoint "${ENDPOINT}" \
    --admin-secret "${ADMIN_SECRET}"

  log "Migration status:"
  hasura migrate status \
    --endpoint "${ENDPOINT}" \
    --admin-secret "${ADMIN_SECRET}" \
    --database-name "${DB_NAME}" || true
}

main() {
  log "hasura-migrate start endpoint=${ENDPOINT} db=${DB_NAME}"
  wait_for_hasura
  if [[ "${ENSURE_PG_SOURCE}" == "true" ]]; then
    ensure_pg_source
  else
    log "ENSURE_PG_SOURCE=false — skipping pg_add_source"
  fi
  prepare_metadata
  apply_migrations
  log "hasura-migrate completed successfully"
}

main "$@"
