-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

\set ON_ERROR_STOP on

-- Run with psql -X -v ON_ERROR_STOP=1 -f (never --single-transaction), before
-- or after the cast_vote_voter_ballot_key migration; run first, it leaves that
-- migration no table scan and nothing to build. The old index keeps serving
-- queries until the key is built. If it stops, fix the cause and run it again.

SELECT EXISTS (
  SELECT 1 FROM pg_attribute
  WHERE attrelid = 'sequent_backend.cast_vote'::regclass
    AND attname IN ('election_id', 'voter_id_string', 'ballot_id')
    AND NOT attnotnull
) AS nullable_key \gset

-- The NOT VALID check refuses new NULLs at once; VALIDATE then scans existing
-- rows without blocking votes, and lets the migration's SET NOT NULL skip it.
-- Adding the check needs a brief exclusive lock; give up rather than queue
-- votes behind a long-running query.
\if :nullable_key
DO $$
BEGIN
  SET LOCAL lock_timeout = '10s';
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conrelid = 'sequent_backend.cast_vote'::regclass
      AND conname = 'cast_vote_voter_ballot_not_null'
  ) THEN
    ALTER TABLE sequent_backend.cast_vote
      ADD CONSTRAINT cast_vote_voter_ballot_not_null CHECK (
        election_id IS NOT NULL
        AND voter_id_string IS NOT NULL
        AND ballot_id IS NOT NULL
      ) NOT VALID;
  END IF;
END $$;
ALTER TABLE sequent_backend.cast_vote
  VALIDATE CONSTRAINT cast_vote_voter_ballot_not_null;
\endif

-- An interrupted concurrent build leaves an invalid index that IF NOT EXISTS
-- would accept without enforcing anything.
DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM pg_index
    WHERE indexrelid = to_regclass('sequent_backend.cast_vote_voter_ballot_key')
      AND NOT indisvalid
  ) THEN
    RAISE EXCEPTION 'cast_vote_voter_ballot_key is invalid; drop it concurrently and rerun';
  END IF;
END $$;

CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS cast_vote_voter_ballot_key
ON sequent_backend.cast_vote
    (tenant_id, election_event_id, election_id, voter_id_string, ballot_id)
INCLUDE (status, area_id);

-- The key leads with the same columns, so no access path is removed.
DROP INDEX CONCURRENTLY IF EXISTS sequent_backend.cast_vote_participation_election_idx;
