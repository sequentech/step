-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- One bounded batch. Stop every Keycloak node and discard all session caches first.
-- Example: psql -v realm_name=tenant-<uuid>-event-<uuid> -v sessions_quiesced=yes -f this-file
\set ON_ERROR_STOP on
\if :{?realm_name}
\else
    DO $$ BEGIN RAISE EXCEPTION 'realm_name is required'; END $$;
\endif
\if :{?sessions_quiesced}
\else
    DO $$ BEGIN RAISE EXCEPTION 'Explicit acknowledgement that every Keycloak node is stopped is required'; END $$;
\endif
SELECT :'sessions_quiesced' = 'yes'
   AND :'realm_name' ~ '^tenant-[0-9a-fA-F]{8}(-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}-event-[0-9a-fA-F]{8}(-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}$'
    AS scope_confirmed \gset
\if :scope_confirmed
\else
    DO $$ BEGIN RAISE EXCEPTION 'Requires sessions_quiesced=yes and one explicit election-event realm'; END $$;
\endif
SELECT EXISTS (SELECT 1 FROM realm WHERE name = :'realm_name') AS realm_exists \gset
\if :realm_exists
\else
    DO $$ BEGIN RAISE EXCEPTION 'The election-event realm does not exist'; END $$;
\endif
BEGIN;
WITH selected AS MATERIALIZED (
    SELECT s.user_session_id, s.offline_flag
    FROM offline_user_session s JOIN realm r ON r.id = s.realm_id
    WHERE r.name = :'realm_name'
    ORDER BY s.user_session_id, s.offline_flag LIMIT 5000
    FOR UPDATE OF s
), clients AS (
    DELETE FROM offline_client_session c USING selected s
    WHERE c.user_session_id = s.user_session_id AND c.offline_flag = s.offline_flag
    RETURNING 1
), removed AS (
    DELETE FROM offline_user_session u USING selected s
    WHERE u.user_session_id = s.user_session_id AND u.offline_flag = s.offline_flag
      AND (SELECT count(*) FROM clients) >= 0
    RETURNING 1
)
SELECT count(*) AS removed_sessions FROM removed;
COMMIT;
