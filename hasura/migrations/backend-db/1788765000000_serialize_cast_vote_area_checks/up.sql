-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

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

  -- Cross-area exclusivity is an integrity rule, including unlimited revotes.
  -- Check after acquiring the existing lock and before the unlimited shortcut.
  IF EXISTS (
    SELECT 1 FROM sequent_backend.cast_vote cv
    WHERE cv.tenant_id = NEW.tenant_id
      AND cv.election_event_id = NEW.election_event_id
      AND cv.election_id = NEW.election_id
      AND cv.voter_id_string = NEW.voter_id_string
      AND cv.status IN ('valid', 'in-progress')
      AND cv.area_id IS NOT NULL
      AND cv.area_id IS DISTINCT FROM NEW.area_id
  ) THEN
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
