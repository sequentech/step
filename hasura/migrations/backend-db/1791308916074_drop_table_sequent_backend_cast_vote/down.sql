-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Recreates the empty table and its revote trigger; the ballots in the ballot
-- boxes are not copied back.
CREATE OR REPLACE FUNCTION public.check_revote_limit()
RETURNS TRIGGER AS $$
DECLARE
  allowed_revotes integer;
  previous_votes bigint;
  voted_in_another_area boolean;
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

  -- Cross-area exclusivity is an integrity rule, including unlimited revotes.
  -- Check after acquiring the existing lock and before the unlimited shortcut.
  SELECT count(*), coalesce(bool_or(
      cv.area_id IS NOT NULL AND cv.area_id IS DISTINCT FROM NEW.area_id
  ), false)
  INTO previous_votes, voted_in_another_area
  FROM sequent_backend.cast_vote cv
  WHERE cv.tenant_id = NEW.tenant_id
    AND cv.election_event_id = NEW.election_event_id
    AND cv.election_id = NEW.election_id
    AND cv.voter_id_string = NEW.voter_id_string
    AND cv.status IN ('valid', 'in-progress');

  -- Count and cross-area eligibility share one scan under the existing lock.
  IF voted_in_another_area THEN
    RAISE EXCEPTION 'check_votes_in_other_areas_failed';
  END IF;

  SELECT num_allowed_revotes INTO allowed_revotes
  FROM "sequent_backend"."election"
  WHERE id = NEW.election_id
  AND tenant_id = NEW.tenant_id
  AND election_event_id = NEW.election_event_id;

  allowed_revotes := COALESCE(allowed_revotes, 1);

  IF allowed_revotes = 0 THEN
    RETURN NEW;
  ELSIF previous_votes >= allowed_revotes THEN
    RAISE EXCEPTION 'insert_failed_exceeds_allowed_revotes';
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TABLE "sequent_backend"."cast_vote" (
    "id" uuid DEFAULT gen_random_uuid() NOT NULL,
    "tenant_id" uuid NOT NULL,
    "election_id" uuid,
    "area_id" uuid,
    "created_at" timestamptz DEFAULT now(),
    "last_updated_at" timestamptz DEFAULT now(),
    "labels" jsonb,
    "annotations" jsonb,
    "content" text,
    "cast_ballot_signature" bytea,
    "voter_id_string" varchar,
    "election_event_id" uuid NOT NULL,
    "ballot_id" text,
    "status" text DEFAULT 'valid' NOT NULL,
    CONSTRAINT "cast_vote_status_check" CHECK (status IN ('in-progress', 'valid', 'discarded')),
    CONSTRAINT "cast_vote_pkey" PRIMARY KEY ("id", "tenant_id", "election_event_id"),
    CONSTRAINT "cast_vote_election_event_id_fkey" FOREIGN KEY ("election_event_id")
        REFERENCES "sequent_backend"."election_event" ("id") ON UPDATE RESTRICT ON DELETE RESTRICT,
    CONSTRAINT "cast_vote_election_event_id_tenant_id_election_id_fkey"
        FOREIGN KEY ("election_event_id", "tenant_id", "election_id")
        REFERENCES "sequent_backend"."election" ("election_event_id", "tenant_id", "id")
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    CONSTRAINT "cast_vote_tenant_id_fkey" FOREIGN KEY ("tenant_id")
        REFERENCES "sequent_backend"."tenant" ("id") ON UPDATE RESTRICT ON DELETE RESTRICT
);
ALTER TABLE "sequent_backend"."cast_vote" ALTER COLUMN "content" SET STORAGE EXTERNAL;

CREATE INDEX "cast_vote_participation_election_idx" ON "sequent_backend"."cast_vote"
    ("tenant_id", "election_event_id", "election_id", "voter_id_string");
CREATE INDEX "cast_vote_participation_event_idx" ON "sequent_backend"."cast_vote"
    ("tenant_id", "election_event_id", "voter_id_string");
CREATE INDEX "idx_cast_vote_in_progress" ON "sequent_backend"."cast_vote"
    ("tenant_id", "election_event_id", "election_id", "voter_id_string", "created_at" DESC)
    WHERE status = 'in-progress';
CREATE INDEX "idx_cast_vote_optimized" ON "sequent_backend"."cast_vote"
    ("tenant_id", "election_event_id", "area_id", "status", "election_id", "voter_id_string", "created_at" DESC);

CREATE TRIGGER "check_revote_trigger" BEFORE INSERT ON "sequent_backend"."cast_vote"
    FOR EACH ROW EXECUTE FUNCTION public.check_revote_limit();
