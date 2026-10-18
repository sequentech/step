#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
#
# Runs after the upstream entrypoint applied everything (it exits on any
# error), records the applied checksum, then starts the server.
if [ -n "${SEQUENT_APPLY_DB_URL:-}" ]; then
    psql "$SEQUENT_APPLY_DB_URL" -q -v ON_ERROR_STOP=1 -v checksum="$SEQUENT_APPLY_CHECKSUM" <<'SQL' \
        || echo "sequent-record-apply: could not record the applied checksum" >&2
INSERT INTO sequent_ops.hasura_apply_state (id, checksum) VALUES (1, :'checksum')
ON CONFLICT (id) DO UPDATE SET checksum = EXCLUDED.checksum, applied_at = now();
SQL
fi
unset SEQUENT_APPLY_CHECKSUM SEQUENT_APPLY_DB_URL
exec "$@"
