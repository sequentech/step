-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Back to election-scoped windows only (1788765000002, 1789420000001).
LOCK TABLE sequent_backend.scheduled_event IN SHARE ROW EXCLUSIVE MODE;

DROP TRIGGER add_election_voting_window ON sequent_backend.election;
DROP FUNCTION sequent_backend.add_election_voting_window();

CREATE OR REPLACE FUNCTION sequent_backend.update_election_voting_window()
RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    old_election uuid;
    new_election uuid;
    affected record;
BEGIN
    IF TG_OP = 'UPDATE' AND ROW(
        OLD.tenant_id, OLD.election_event_id, OLD.task_id,
        OLD.event_payload, OLD.cron_config, OLD.archived_at
    ) IS NOT DISTINCT FROM ROW(
        NEW.tenant_id, NEW.election_event_id, NEW.task_id,
        NEW.event_payload, NEW.cron_config, NEW.archived_at
    ) THEN
        RETURN NULL;
    END IF;

    IF TG_OP <> 'INSERT' THEN
        old_election := sequent_backend.voting_window_election_id(OLD);
    END IF;
    IF TG_OP <> 'DELETE' THEN
        new_election := sequent_backend.voting_window_election_id(NEW);
    END IF;

    FOR affected IN
        SELECT DISTINCT tenant_id, event_id, election_id
        FROM (VALUES
            (OLD.tenant_id, OLD.election_event_id, old_election),
            (NEW.tenant_id, NEW.election_event_id, new_election)
        ) AS scopes(tenant_id, event_id, election_id)
        WHERE election_id IS NOT NULL
        ORDER BY tenant_id, event_id, election_id
    LOOP
        PERFORM sequent_backend.refresh_election_voting_window(
            affected.tenant_id, affected.event_id, affected.election_id
        );
    END LOOP;
    RETURN NULL;
END;
$$;

CREATE OR REPLACE FUNCTION sequent_backend.refresh_election_voting_window(
    target_tenant uuid, target_event uuid, target_election uuid
) RETURNS void LANGUAGE plpgsql AS $$
DECLARE
    opening_count integer;
    closing_count integer;
    opening_date text;
    closing_date text;
    invalid_dates boolean;
    task_prefix text := format('tenant_%s_event_%s_election_%s_',
        target_tenant, target_event, target_election);
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended('voting-window-writer', 0));

    SELECT
        count(*) FILTER (WHERE task_id = task_prefix || 'START_VOTING_PERIOD'),
        count(*) FILTER (WHERE task_id = task_prefix || 'END_VOTING_PERIOD'),
        max(cron_config ->> 'scheduled_date') FILTER (WHERE task_id = task_prefix || 'START_VOTING_PERIOD'),
        max(cron_config ->> 'scheduled_date') FILTER (WHERE task_id = task_prefix || 'END_VOTING_PERIOD'),
        bool_or(cron_config IS NOT NULL AND (
            jsonb_typeof(cron_config) <> 'object'
            OR jsonb_typeof(cron_config -> 'cron') NOT IN ('string', 'null')
            OR jsonb_typeof(cron_config -> 'scheduled_date') NOT IN ('string', 'null')
        ))
    INTO opening_count, closing_count, opening_date, closing_date, invalid_dates
    FROM sequent_backend.scheduled_event schedule
    WHERE tenant_id = target_tenant
      AND election_event_id = target_event
      AND task_id IN (
          task_prefix || 'START_VOTING_PERIOD', task_prefix || 'END_VOTING_PERIOD'
      )
      AND sequent_backend.voting_window_election_id(schedule) = target_election
      AND archived_at IS NULL;

    IF opening_count > 1 OR closing_count > 1 OR invalid_dates THEN
        RAISE EXCEPTION 'ambiguous_or_invalid_voting_window for election %', target_election;
    END IF;

    PERFORM opening_date::timestamptz, closing_date::timestamptz;

    IF opening_count + closing_count = 0 THEN
        DELETE FROM sequent_backend.election_voting_window
        WHERE tenant_id = target_tenant AND election_event_id = target_event
          AND election_id = target_election;
    ELSE
        INSERT INTO sequent_backend.election_voting_window
            (tenant_id, election_event_id, election_id, start_date, end_date)
        VALUES (target_tenant, target_event, target_election, opening_date, closing_date)
        ON CONFLICT (tenant_id, election_event_id, election_id) DO UPDATE
            SET start_date = EXCLUDED.start_date, end_date = EXCLUDED.end_date;
    END IF;
END;
$$;

-- Rebuild every materialized window without the event-wide close.
DO $$
DECLARE
    scope record;
BEGIN
    FOR scope IN
        SELECT tenant_id, election_event_id, election_id
        FROM sequent_backend.election_voting_window
        ORDER BY tenant_id, election_event_id, election_id
    LOOP
        PERFORM sequent_backend.refresh_election_voting_window(
            scope.tenant_id, scope.election_event_id, scope.election_id
        );
    END LOOP;
END;
$$;

DROP FUNCTION sequent_backend.refresh_event_voting_windows(uuid, uuid);
DROP FUNCTION sequent_backend.event_voting_close_date(uuid, uuid);
DROP FUNCTION sequent_backend.voting_window_is_event_close(sequent_backend.scheduled_event);
