-- Combined cast_vote `status` migration (meta-6409):
--   1. Add the `status` column.
--   2. Index optimized for find_area_ballots (includes `status`).
--   3. Partial index for the review_cast_votes `in-progress` scan.
--   4. Revote limit trigger counts every unresolved or accepted vote
--      ('discarded' votes do not consume a revote slot).

ALTER TABLE "sequent_backend"."cast_vote"
  ADD COLUMN "status" text NOT NULL DEFAULT 'valid';

ALTER TABLE "sequent_backend"."cast_vote"
  ADD CONSTRAINT "cast_vote_status_check"
  CHECK (status IN ('in-progress', 'indeterminate', 'valid', 'discarded'));

CREATE INDEX "idx_cast_vote_optimized" ON
  "sequent_backend"."cast_vote" USING btree ("tenant_id", "election_event_id", "area_id", "status", "election_id", "voter_id_string", "created_at" DESC);

CREATE INDEX IF NOT EXISTS idx_cast_vote_in_progress
ON sequent_backend.cast_vote (tenant_id, election_event_id, election_id, voter_id_string, created_at DESC)
WHERE status = 'in-progress';

CREATE OR REPLACE FUNCTION check_revote_limit()
RETURNS TRIGGER AS $$
DECLARE
  allowed_revotes integer;
BEGIN
  -- Serialize the count-and-insert decision for one voter and election. Without
  -- this lock two concurrent inserts can both observe the same count.
  PERFORM pg_advisory_xact_lock(
    hashtextextended(
      NEW.tenant_id::text || ':' || NEW.election_event_id::text || ':' ||
      NEW.election_id::text || ':' || NEW.voter_id_string,
      0
    )
  );

  SELECT num_allowed_revotes INTO allowed_revotes
  FROM "sequent_backend"."election"
  WHERE id = NEW.election_id
  AND tenant_id = NEW.tenant_id
  AND election_event_id = NEW.election_event_id;

  allowed_revotes := COALESCE(allowed_revotes, 1);

  IF allowed_revotes = 0 THEN
    RETURN NEW;
  ELSIF (
    SELECT COUNT(*)
    FROM "sequent_backend"."cast_vote" cv
    WHERE cv.election_id = NEW.election_id
    AND cv.voter_id_string = NEW.voter_id_string
    AND cv.tenant_id = NEW.tenant_id
    AND cv.election_event_id = NEW.election_event_id
    AND cv.status IN ('valid', 'in-progress', 'indeterminate')
  ) >= allowed_revotes THEN
    RAISE EXCEPTION 'insert_failed_exceeds_allowed_revotes';
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;
