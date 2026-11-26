-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Hasura's table mutations do not pass through manage-election-dates.
-- The scheduler uses its own connection to stop rows and record outcomes.
CREATE FUNCTION sequent_backend.guard_hasura_schedule_mutation() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    parent record;
BEGIN
    IF NULLIF(current_setting('hasura.user', true), '') IS NOT NULL THEN
        IF (TG_OP <> 'INSERT' AND OLD.event_processor IN ('START_LOCKDOWN_PERIOD', 'END_LOCKDOWN_PERIOD'))
           OR (TG_OP <> 'DELETE' AND NEW.event_processor IN ('START_LOCKDOWN_PERIOD', 'END_LOCKDOWN_PERIOD')) THEN
            RAISE EXCEPTION 'Use the event-wide schedule action to change a lockdown schedule.'
                USING ERRCODE = '42501';
        END IF;

        -- A retargeting write affects both parents. Lock by event identity
        -- even if a caller also changes tenant_id. The scheduler locks the
        -- parent first, so never wait here while holding the child row.
        FOR parent IN
            SELECT presentation FROM sequent_backend.election_event
            WHERE id IN (CASE WHEN TG_OP <> 'INSERT' THEN OLD.election_event_id END,
                         CASE WHEN TG_OP <> 'DELETE' THEN NEW.election_event_id END)
            ORDER BY id FOR SHARE NOWAIT
        LOOP
            IF parent.presentation ->> 'locked_down' = 'locked-down' THEN
                RAISE EXCEPTION 'The election event is locked down. Its schedule is read-only.'
                    USING ERRCODE = '42501';
            END IF;
        END LOOP;
    END IF;
    IF TG_OP = 'DELETE' THEN RETURN OLD; END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER guard_hasura_schedule_mutation
BEFORE INSERT OR UPDATE OR DELETE ON sequent_backend.scheduled_event
FOR EACH ROW EXECUTE FUNCTION sequent_backend.guard_hasura_schedule_mutation();
