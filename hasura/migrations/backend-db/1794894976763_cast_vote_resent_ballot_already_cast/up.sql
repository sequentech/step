-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

CREATE OR REPLACE FUNCTION check_revote_limit()
RETURNS TRIGGER AS $$
DECLARE
  allowed_revotes integer;
  previous_votes bigint;
  voted_in_another_area boolean;
  already_cast boolean;
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

  -- Count, cross-area eligibility and resent ballots share one scan under the
  -- existing lock. Discarded votes count only as resent ballots.
  SELECT
    count(*) FILTER (WHERE cv.status IN ('valid', 'in-progress')),
    coalesce(bool_or(
      cv.status IN ('valid', 'in-progress')
      AND cv.area_id IS NOT NULL
      AND cv.area_id IS DISTINCT FROM NEW.area_id
    ), false),
    coalesce(bool_or(cv.ballot_id = NEW.ballot_id), false)
  INTO previous_votes, voted_in_another_area, already_cast
  FROM sequent_backend.cast_vote cv
  WHERE cv.tenant_id = NEW.tenant_id
    AND cv.election_event_id = NEW.election_event_id
    AND cv.election_id = NEW.election_id
    AND cv.voter_id_string = NEW.voter_id_string;

  -- A resent ballot is answered with the stored vote, so it must not be
  -- stored again or use up a revote, whatever the status of the stored vote.
  IF already_cast THEN
    RAISE EXCEPTION 'ballot_already_cast';
  END IF;

  -- Cross-area exclusivity is an integrity rule, including unlimited revotes.
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
