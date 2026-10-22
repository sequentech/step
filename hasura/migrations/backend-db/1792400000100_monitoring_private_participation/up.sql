-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
ALTER TABLE sequent_backend.monitoring_voter ADD COLUMN has_voted boolean NOT NULL DEFAULT false;
COMMENT ON COLUMN sequent_backend.monitoring_voter.has_voted IS 'Participation state without a ballot timestamp; private events never set first_voted_at.';

-- A participation refresh must not encode the cast time in the row's update timestamp.
CREATE FUNCTION sequent_backend.monitoring_voter_activity_timestamp() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF (to_jsonb(NEW) - ARRAY['has_voted', 'first_voted_at', 'updated_at'])
       IS NOT DISTINCT FROM
       (to_jsonb(OLD) - ARRAY['has_voted', 'first_voted_at', 'updated_at']) THEN
        NEW.updated_at := OLD.updated_at;
    ELSE
        NEW.updated_at := now();
    END IF;
    RETURN NEW;
END;
$$;
DROP TRIGGER set_sequent_backend_monitoring_voter_updated_at ON sequent_backend.monitoring_voter;
CREATE TRIGGER set_sequent_backend_monitoring_voter_updated_at
BEFORE UPDATE ON sequent_backend.monitoring_voter FOR EACH ROW
EXECUTE FUNCTION sequent_backend.monitoring_voter_activity_timestamp();
