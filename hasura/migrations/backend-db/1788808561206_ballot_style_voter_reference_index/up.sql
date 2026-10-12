-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- On populated deployments, prebuild with scripts/postgres/ballot_style_voter_reference_index.sql.
-- IF NOT EXISTS then lets transactional migration bookkeeping reuse that index.
CREATE INDEX IF NOT EXISTS ballot_style_voter_reference_idx
    ON sequent_backend.ballot_style (tenant_id, election_event_id, area_id, election_id)
    INCLUDE (id, ballot_publication_id)
    WHERE deleted_at IS NULL;

-- An interrupted concurrent build leaves an unusable index under the same name.
DO $$
BEGIN
    IF NOT (SELECT indisvalid FROM pg_index
            WHERE indexrelid = 'sequent_backend.ballot_style_voter_reference_idx'::regclass) THEN
        RAISE EXCEPTION 'ballot_style_voter_reference_idx is invalid; drop it concurrently and rebuild before retrying';
    END IF;
END;
$$;
