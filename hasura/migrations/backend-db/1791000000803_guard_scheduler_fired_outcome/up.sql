-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Execution evidence and cached predictions must not be editable as ordinary annotations.
-- Prediction cache integrity preserves change-log baselines and edit attribution.
CREATE OR REPLACE FUNCTION sequent_backend.guard_scheduler_fired_outcome()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    previous jsonb;
    generated_key text;
BEGIN
    FOREACH generated_key IN ARRAY ARRAY['fired_outcome', 'predicted_outcome'] LOOP
        previous := NULL;
        IF TG_OP = 'UPDATE' THEN
            previous := NULLIF(OLD.annotations -> generated_key, 'null'::jsonb);
        END IF;
        IF previous IS DISTINCT FROM NULLIF(NEW.annotations -> generated_key, 'null'::jsonb)
           AND NOT sequent_backend.trusted_write_allowed() THEN
            RAISE EXCEPTION 'Scheduled execution and prediction results can only be written by the scheduler'
                USING ERRCODE = '42501',
                      HINT = 'Reload the scheduled event before saving. Its execution results and predictions are read-only.';
        END IF;
    END LOOP;
    RETURN NEW;
END;
$$;

CREATE TRIGGER guard_scheduler_fired_outcome
BEFORE INSERT OR UPDATE OF annotations ON sequent_backend.scheduled_event
FOR EACH ROW EXECUTE FUNCTION sequent_backend.guard_scheduler_fired_outcome();
