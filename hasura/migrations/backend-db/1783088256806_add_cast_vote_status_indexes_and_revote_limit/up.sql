-- Combined cast_vote `status` migration (meta-6409):
--   1. Add the `status` column (in-progress | valid | discarded).
--   2. Index optimized for find_area_ballots (includes `status`).
--   3. Partial index for the review_cast_votes `in-progress` scan.
--   4. Revote limit trigger counts only 'valid' and 'in-progress' votes
--      ('discarded' votes never became a recorded vote and must not consume a
--      revote slot).

ALTER TABLE "sequent_backend"."cast_vote"
  ADD COLUMN "status" text NOT NULL DEFAULT 'valid';

CREATE INDEX "idx_cast_vote_optimized" ON
  "sequent_backend"."cast_vote" USING btree ("tenant_id", "election_event_id", "area_id", "status", "election_id", "voter_id_string", "created_at");

CREATE INDEX IF NOT EXISTS idx_cast_vote_in_progress
ON sequent_backend.cast_vote (election_id, voter_id_string, created_at DESC)
WHERE status = 'in-progress';

CREATE OR REPLACE FUNCTION check_revote_limit()
RETURNS TRIGGER AS $$
DECLARE
  allowed_revotes integer;
BEGIN
  SELECT num_allowed_revotes INTO allowed_revotes
  FROM "sequent_backend"."election"
  WHERE id = NEW.election_id
  AND tenant_id = NEW.tenant_id
  AND election_event_id = NEW.election_event_id;

  IF allowed_revotes = 0 THEN
    RETURN NEW;
  ELSIF (
    SELECT COUNT(*)
    FROM "sequent_backend"."cast_vote" cv
    WHERE cv.election_id = NEW.election_id
    AND cv.voter_id_string = NEW.voter_id_string
    AND cv.tenant_id = NEW.tenant_id
    AND cv.election_event_id = NEW.election_event_id
    AND cv.status IN ('valid', 'in-progress')
  ) >= allowed_revotes THEN
    RAISE EXCEPTION 'insert_failed_exceeds_allowed_revotes';
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;
