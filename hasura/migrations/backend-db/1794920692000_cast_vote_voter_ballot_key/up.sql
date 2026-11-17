-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- A voter's ballot is stored once, on every write path, including one that
-- bypasses check_revote_limit. Adding ballot_id to the participation index
-- keeps its access path and one index write per cast; the included columns
-- let check_revote_limit read only the index. A NULL key column would let a
-- row past the key, so those columns become NOT NULL. On large tables, run
-- scripts/postgres/cast_vote_covering_index.sql first: it builds the key and
-- proves the columns hold no NULLs without blocking votes.

-- An interrupted concurrent build leaves an invalid index that IF NOT EXISTS
-- would accept without enforcing anything.
DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM pg_index
    WHERE indexrelid = to_regclass('sequent_backend.cast_vote_voter_ballot_key')
      AND NOT indisvalid
  ) THEN
    RAISE EXCEPTION 'cast_vote_voter_ballot_key is invalid; drop it and rerun';
  END IF;
END $$;

CREATE UNIQUE INDEX IF NOT EXISTS cast_vote_voter_ballot_key
ON sequent_backend.cast_vote
  (tenant_id, election_event_id, election_id, voter_id_string, ballot_id)
INCLUDE (status, area_id);

DROP INDEX IF EXISTS sequent_backend.cast_vote_participation_election_idx;

-- A validated cast_vote_voter_ballot_not_null check lets SET NOT NULL skip
-- its table scan; it is redundant afterwards.
ALTER TABLE sequent_backend.cast_vote
  ALTER COLUMN election_id SET NOT NULL,
  ALTER COLUMN voter_id_string SET NOT NULL,
  ALTER COLUMN ballot_id SET NOT NULL;

ALTER TABLE sequent_backend.cast_vote
  DROP CONSTRAINT IF EXISTS cast_vote_voter_ballot_not_null;
