-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- VOTE-LIFECYCLE: one event-wide END_VOTING_PERIOD closes every election
-- (Post) without a close of its own, as generate_voting_period_dates reads
-- it. The projection keeps one row per election: its own opening, and its
-- own close or else the event-wide one.
LOCK TABLE sequent_backend.scheduled_event IN SHARE ROW EXCLUSIVE MODE;

-- The event-wide close task: the canonical name with no election, an object
-- payload without an election and channels that include ONLINE (missing,
-- null or empty mean ONLINE and KIOSK).
CREATE FUNCTION sequent_backend.voting_window_is_event_close(
    schedule sequent_backend.scheduled_event
) RETURNS boolean LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE
    channels jsonb;
BEGIN
    IF schedule.tenant_id IS NULL OR schedule.election_event_id IS NULL
       OR schedule.task_id IS DISTINCT FROM format('tenant_%s_event_%s_END_VOTING_PERIOD',
           schedule.tenant_id, schedule.election_event_id)
       OR jsonb_typeof(schedule.event_payload) IS DISTINCT FROM 'object'
       OR COALESCE(schedule.event_payload -> 'election_id', 'null'::jsonb) <> 'null'::jsonb
    THEN
        RETURN false;
    END IF;
    channels := COALESCE(schedule.event_payload -> 'voting_channels', 'null'::jsonb);
    RETURN channels IN ('null'::jsonb, '[]'::jsonb)
        OR (jsonb_typeof(channels) = 'array'
            AND channels ? 'ONLINE'
            AND channels <@ '["ONLINE", "KIOSK", "EARLY_VOTING", "TELEPHONE"]'::jsonb);
END;
$$;

-- The event-wide close date, or NULL. Only a date the scheduler runs (RFC
-- 3339 with an offset) closes anything. Two active closes are ambiguous.
-- Without an active one, the latest close that already ran still applies;
-- a past row with an invalid date is skipped with a warning, so it can't
-- block schedule writes.
CREATE FUNCTION sequent_backend.event_voting_close_date(
    target_tenant uuid, target_event uuid
) RETURNS text LANGUAGE plpgsql STABLE AS $$
DECLARE
    close_count integer;
    close_date text;
    candidate record;
BEGIN
    SELECT count(*), max(cron_config ->> 'scheduled_date')
    INTO close_count, close_date
    FROM sequent_backend.scheduled_event schedule
    WHERE tenant_id = target_tenant
      AND election_event_id = target_event
      AND task_id = format('tenant_%s_event_%s_END_VOTING_PERIOD', target_tenant, target_event)
      AND archived_at IS NULL
      AND stopped_at IS NULL
      AND sequent_backend.voting_window_is_event_close(schedule)
      AND jsonb_typeof(cron_config -> 'scheduled_date') = 'string'
      AND (cron_config ->> 'scheduled_date') ~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}[Tt ]([01][0-9]|2[0-3]):[0-5][0-9]:([0-5][0-9]|60)([.][0-9]+)?([Zz]|[+-][0-9]{2}:[0-9]{2})$';
    IF close_count > 1 THEN
        RAISE EXCEPTION 'ambiguous_event_voting_close for election event %', target_event;
    END IF;
    IF close_count = 1 THEN
        PERFORM close_date::timestamptz;
        RETURN close_date;
    END IF;

    FOR candidate IN
        SELECT schedule.id, cron_config ->> 'scheduled_date' AS scheduled_date
        FROM sequent_backend.scheduled_event schedule
        WHERE tenant_id = target_tenant
          AND election_event_id = target_event
          AND task_id = format('tenant_%s_event_%s_END_VOTING_PERIOD', target_tenant, target_event)
          AND archived_at IS NULL
          AND stopped_at IS NOT NULL
          AND sequent_backend.voting_window_is_event_close(schedule)
          AND jsonb_typeof(cron_config -> 'scheduled_date') = 'string'
        ORDER BY stopped_at DESC, id
    LOOP
        BEGIN
            IF candidate.scheduled_date ~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}[Tt ]([01][0-9]|2[0-3]):[0-5][0-9]:([0-5][0-9]|60)([.][0-9]+)?([Zz]|[+-][0-9]{2}:[0-9]{2})$' THEN
                PERFORM candidate.scheduled_date::timestamptz;
                RETURN candidate.scheduled_date;
            END IF;
            RAISE WARNING 'Skipping event-wide close % of election event %: its date has no offset',
                candidate.id, target_event;
        EXCEPTION WHEN data_exception THEN
            RAISE WARNING 'Skipping event-wide close % of election event %: invalid date',
                candidate.id, target_event;
        END;
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
    -- Direct refreshes use the same lock as source-table writers. A fresh
    -- query after waiting sees endpoints committed by the preceding writer.
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

    -- No close of its own (or one without a date): the event-wide close.
    closing_date := COALESCE(closing_date,
        sequent_backend.event_voting_close_date(target_tenant, target_event));

    IF opening_count + closing_count = 0 AND closing_date IS NULL THEN
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

-- Every election of an event: the elections that exist, the windows already
-- materialized and the elections its schedules name (imports may write
-- schedules before elections).
CREATE FUNCTION sequent_backend.refresh_event_voting_windows(
    target_tenant uuid, target_event uuid
) RETURNS void LANGUAGE plpgsql AS $$
DECLARE
    scope record;
BEGIN
    FOR scope IN
        SELECT id AS election_id
        FROM sequent_backend.election
        WHERE tenant_id = target_tenant AND election_event_id = target_event
        UNION
        SELECT election_id
        FROM sequent_backend.election_voting_window
        WHERE tenant_id = target_tenant AND election_event_id = target_event
        UNION
        SELECT sequent_backend.voting_window_election_id(schedule)
        FROM sequent_backend.scheduled_event schedule
        WHERE tenant_id = target_tenant AND election_event_id = target_event
          AND sequent_backend.voting_window_election_id(schedule) IS NOT NULL
        ORDER BY election_id
    LOOP
        PERFORM sequent_backend.refresh_election_voting_window(
            target_tenant, target_event, scope.election_id
        );
    END LOOP;
END;
$$;

CREATE OR REPLACE FUNCTION sequent_backend.update_election_voting_window()
RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    old_election uuid;
    new_election uuid;
    affected record;
BEGIN
    -- Execution bookkeeping and labels do not change the voting window.
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

    -- A moved/renamed task invalidates its old scope as well as its new scope.
    -- Stable ordering avoids opposing lock orders for a single scope move.
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

    -- The event-wide close task (by name, so a payload that stops matching
    -- also counts) changes the window of every election of its event.
    FOR affected IN
        SELECT DISTINCT tenant_id, event_id
        FROM (VALUES
            (OLD.tenant_id, OLD.election_event_id, OLD.task_id),
            (NEW.tenant_id, NEW.election_event_id, NEW.task_id)
        ) AS scopes(tenant_id, event_id, task_id)
        WHERE tenant_id IS NOT NULL AND event_id IS NOT NULL
          AND task_id = format('tenant_%s_event_%s_END_VOTING_PERIOD', tenant_id, event_id)
        ORDER BY tenant_id, event_id
    LOOP
        PERFORM sequent_backend.refresh_event_voting_windows(
            affected.tenant_id, affected.event_id
        );
    END LOOP;
    RETURN NULL;
END;
$$;

-- A new election under an event-wide close gets that close.
CREATE FUNCTION sequent_backend.add_election_voting_window()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.tenant_id IS NOT NULL AND NEW.election_event_id IS NOT NULL THEN
        PERFORM sequent_backend.refresh_election_voting_window(
            NEW.tenant_id, NEW.election_event_id, NEW.id
        );
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER add_election_voting_window
AFTER INSERT ON sequent_backend.election
FOR EACH ROW EXECUTE FUNCTION sequent_backend.add_election_voting_window();

-- Report every ambiguous or invalid active event-wide close before
-- rebuilding. Keep the source intact: operators archive or correct them
-- before retrying. Closes that already ran never block (see above).
DO $$
DECLARE
    scope record;
    failures jsonb := '[]'::jsonb;
BEGIN
    FOR scope IN
        SELECT DISTINCT tenant_id, election_event_id
        FROM sequent_backend.scheduled_event schedule
        WHERE archived_at IS NULL
          AND stopped_at IS NULL
          AND sequent_backend.voting_window_is_event_close(schedule)
        ORDER BY tenant_id, election_event_id
    LOOP
        BEGIN
            PERFORM sequent_backend.event_voting_close_date(
                scope.tenant_id, scope.election_event_id
            );
        EXCEPTION WHEN raise_exception OR data_exception THEN
            failures := failures || jsonb_build_array(jsonb_build_object(
                'tenant_id', scope.tenant_id,
                'election_event_id', scope.election_event_id,
                'problem', SQLERRM
            ));
        END;
    END LOOP;
    IF jsonb_array_length(failures) > 0 THEN
        RAISE EXCEPTION 'invalid_event_voting_close_backfill'
            USING DETAIL = failures::text,
                  HINT = 'Archive duplicate or correct invalid event-wide END_VOTING_PERIOD schedules before retrying the migration.';
    END IF;
END;
$$;

-- Rebuild every event with an event-wide close, under the source-table lock.
DO $$
DECLARE
    scope record;
BEGIN
    FOR scope IN
        SELECT DISTINCT tenant_id, election_event_id
        FROM sequent_backend.scheduled_event schedule
        WHERE archived_at IS NULL
          AND sequent_backend.voting_window_is_event_close(schedule)
        ORDER BY tenant_id, election_event_id
    LOOP
        PERFORM sequent_backend.refresh_event_voting_windows(
            scope.tenant_id, scope.election_event_id
        );
    END LOOP;
END;
$$;
