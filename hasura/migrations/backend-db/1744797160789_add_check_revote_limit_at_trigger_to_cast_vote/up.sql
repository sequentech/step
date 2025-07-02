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
  ) >= allowed_revotes THEN
    RAISE EXCEPTION 'insert_failed_exceeds_allowed_revotes';
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER check_revote_trigger
BEFORE INSERT ON "sequent_backend"."cast_vote"
FOR EACH ROW
EXECUTE PROCEDURE check_revote_limit();
