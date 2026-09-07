-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Run with psql -X -v ON_ERROR_STOP=1 -f (never --single-transaction).
-- Keep the old index serving queries throughout the concurrent build. Stop on
-- any error; inspect pg_index.indisvalid before recovering an interrupted build.
CREATE INDEX CONCURRENTLY cast_vote_participation_election_covering_idx
ON sequent_backend.cast_vote (tenant_id, election_event_id, election_id, voter_id_string)
INCLUDE (status);

-- This replaces the same access path with a covering version; no reporting
-- access path is removed. CREATE must have succeeded before retiring the old one.
DROP INDEX CONCURRENTLY sequent_backend.cast_vote_participation_election_idx;
ALTER INDEX sequent_backend.cast_vote_participation_election_covering_idx
RENAME TO cast_vote_participation_election_idx;
