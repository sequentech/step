-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- One fingerprint per contest ciphertext of the ballot. Rows cast before this
-- column existed keep it NULL.
ALTER TABLE sequent_backend.cast_vote
  ADD COLUMN IF NOT EXISTS ciphertext_fingerprints text[];

CREATE INDEX IF NOT EXISTS cast_vote_ciphertext_fingerprints_idx
ON sequent_backend.cast_vote USING gin (ciphertext_fingerprints);

-- A ciphertext belongs to the voter who cast it first in the election event.
CREATE OR REPLACE FUNCTION check_ciphertext_not_cast_by_other_voter()
RETURNS TRIGGER AS $$
DECLARE
  fingerprint text;
BEGIN
  IF NEW.ciphertext_fingerprints IS NULL
     OR cardinality(NEW.ciphertext_fingerprints) = 0 THEN
    RETURN NEW;
  END IF;

  -- Serialize inserts of the same ciphertext. Without this lock two voters
  -- casting it concurrently can both find no earlier row. Locks are taken in
  -- sorted order so that two ballots sharing several ciphertexts cannot
  -- deadlock.
  FOREACH fingerprint IN ARRAY (
    SELECT array_agg(DISTINCT f ORDER BY f)
    FROM unnest(NEW.ciphertext_fingerprints) AS f
  ) LOOP
    PERFORM pg_advisory_xact_lock(
      hashtextextended(
        NEW.tenant_id::text || ':' || NEW.election_event_id::text || ':' ||
        fingerprint,
        0
      )
    );
  END LOOP;

  IF EXISTS (
    SELECT 1
    FROM sequent_backend.cast_vote cv
    WHERE cv.ciphertext_fingerprints && NEW.ciphertext_fingerprints
      AND cv.tenant_id = NEW.tenant_id
      AND cv.election_event_id = NEW.election_event_id
      AND cv.voter_id_string IS DISTINCT FROM NEW.voter_id_string
  ) THEN
    RAISE EXCEPTION 'ciphertext_already_cast';
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS check_ciphertext_reuse_trigger ON sequent_backend.cast_vote;
CREATE TRIGGER check_ciphertext_reuse_trigger
BEFORE INSERT ON sequent_backend.cast_vote
FOR EACH ROW EXECUTE FUNCTION check_ciphertext_not_cast_by_other_voter();
