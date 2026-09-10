-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

\set ON_ERROR_STOP on

-- Run before the Hasura migration, without --single-transaction.
-- An interrupted build must be dropped and rebuilt; never accept an invalid index.
CREATE INDEX CONCURRENTLY IF NOT EXISTS ballot_style_voter_reference_idx
    ON sequent_backend.ballot_style (tenant_id, election_event_id, area_id, election_id)
    INCLUDE (id, ballot_publication_id)
    WHERE deleted_at IS NULL;
DO $$
BEGIN
    IF NOT (SELECT indisvalid FROM pg_index
            WHERE indexrelid = 'sequent_backend.ballot_style_voter_reference_idx'::regclass) THEN
        RAISE EXCEPTION 'ballot_style_voter_reference_idx is invalid; drop it concurrently and rerun this script';
    END IF;
END;
$$;
