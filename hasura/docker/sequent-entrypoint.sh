#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
#
# Wraps the upstream cli-migrations entrypoint. It applies metadata, migrations
# and a metadata reload on every start, and each of those makes every running
# replica rebuild its schema cache. Skip them when this image's metadata and
# migrations are the ones last applied to the database.
set -uo pipefail

APPLY_STATE=sequent_ops.hasura_apply_state
METADATA_DIR=${HASURA_GRAPHQL_METADATA_DIR:-/hasura-metadata}
MIGRATIONS_DIR=${HASURA_GRAPHQL_MIGRATIONS_DIR:-/hasura-migrations}

log() {
    printf '{"timestamp":"%s","level":"info","type":"startup","detail":{"kind":"sequent-apply-state","info":"%s"}}\n' \
        "$(date -u '+%Y-%m-%dT%H:%M:%S.000+0000')" "$1"
}

tree_checksum() {
    [ -d "$1" ] || { echo none; return; }
    (cd "$1" && find . -type f -print0 | LC_ALL=C sort -z | xargs -0 -r sha256sum) | sha256sum | cut -c1-64
}

CHECKSUM="$(tree_checksum "$METADATA_DIR")-$(tree_checksum "$MIGRATIONS_DIR")"
DB_URL=${PG_DATABASE_URL:-${HASURA_GRAPHQL_DATABASE_URL:-}}
STORED=""
if [ -n "$DB_URL" ]; then
    STORED=$(psql "$DB_URL" -tAc "SELECT checksum FROM $APPLY_STATE WHERE id = 1" 2>/dev/null || true)
fi

if [ -n "$STORED" ] && [ "$STORED" = "$CHECKSUM" ]; then
    log "metadata and migrations already applied ($CHECKSUM), skipping"
    export HASURA_GRAPHQL_METADATA_DIR=/nonexistent HASURA_GRAPHQL_MIGRATIONS_DIR=/nonexistent
    exec /bin/docker-entrypoint.sh "$@"
fi

log "applying metadata and migrations ($CHECKSUM, last applied: ${STORED:-none})"
export SEQUENT_APPLY_CHECKSUM=$CHECKSUM SEQUENT_APPLY_DB_URL=$DB_URL
exec /bin/docker-entrypoint.sh /bin/sequent-record-apply.sh "$@"
