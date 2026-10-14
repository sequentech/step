-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

LOCK TABLE sequent_backend.scheduled_event IN SHARE ROW EXCLUSIVE MODE;

CREATE OR REPLACE FUNCTION sequent_backend.voting_window_election_id(
    schedule sequent_backend.scheduled_event
) RETURNS uuid LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE
    election_text text := schedule.event_payload ->> 'election_id';
    task_prefix text;
BEGIN
    IF schedule.tenant_id IS NULL OR schedule.election_event_id IS NULL
       OR election_text IS NULL
       OR election_text !~ '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
       OR schedule.event_payload <> jsonb_build_object('election_id', election_text)
    THEN
        RETURN NULL;
    END IF;

    task_prefix := format('tenant_%s_event_%s_election_%s_',
        schedule.tenant_id, schedule.election_event_id, election_text);
    IF schedule.task_id IN (
        task_prefix || 'START_VOTING_PERIOD', task_prefix || 'END_VOTING_PERIOD'
    ) THEN
        RETURN election_text::uuid;
    END IF;
    RETURN NULL;
END;
$$;

DO $$
DECLARE
    scope record;
BEGIN
    FOR scope IN
        SELECT tenant_id, election_event_id, election_id
        FROM sequent_backend.election_voting_window
        UNION
        SELECT tenant_id, election_event_id,
            sequent_backend.voting_window_election_id(schedule)
        FROM sequent_backend.scheduled_event schedule
        WHERE sequent_backend.voting_window_election_id(schedule) IS NOT NULL
    LOOP
        PERFORM sequent_backend.refresh_election_voting_window(
            scope.tenant_id, scope.election_event_id, scope.election_id
        );
    END LOOP;
END;
$$;
