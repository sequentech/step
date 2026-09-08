-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- A transactionally maintained projection, not a periodically refreshed view.
-- Keep it independent of election creation: imports may create schedules first.
-- It is internal state, not a Hasura-managed API table.
LOCK TABLE sequent_backend.scheduled_event IN SHARE ROW EXCLUSIVE MODE;

-- The leading keys also serve event-scoped schedule queries. Task IDs let
-- window maintenance fetch its two endpoints even in a very busy event.
-- Build under the existing configuration-write lock, before backfilling.
CREATE INDEX scheduled_event_active_scope_task_idx
    ON sequent_backend.scheduled_event (tenant_id, election_event_id, task_id)
    WHERE archived_at IS NULL;

CREATE TABLE sequent_backend.election_voting_window (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    election_id uuid NOT NULL,
    start_date text,
    end_date text,
    PRIMARY KEY (tenant_id, election_event_id, election_id)
);

-- Match the same task names and exact payload as generate_voting_period_dates.
-- Ignore unrelated tasks without attempting to cast their arbitrary payloads.
CREATE FUNCTION sequent_backend.voting_window_election_id(
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

-- Serialize schedule writers before they acquire row locks. Per-election
-- locks cannot prevent inverse ordering across several statements in one
-- transaction. Vote casting only reads the projection and never takes this lock.
CREATE FUNCTION sequent_backend.lock_voting_window_writer()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended('voting-window-writer', 0));
    RETURN NULL;
END;
$$;

CREATE TRIGGER lock_voting_window_writer
BEFORE INSERT OR UPDATE OR DELETE ON sequent_backend.scheduled_event
FOR EACH STATEMENT EXECUTE FUNCTION sequent_backend.lock_voting_window_writer();

CREATE FUNCTION sequent_backend.refresh_election_voting_window(
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
      -- Expose ordinary equality predicates to the index planner. The helper
      -- remains a residual check of the exact canonical name/payload contract.
      AND task_id IN (
          task_prefix || 'START_VOTING_PERIOD', task_prefix || 'END_VOTING_PERIOD'
      )
      AND sequent_backend.voting_window_election_id(schedule) = target_election
      AND archived_at IS NULL;

    -- Previously duplicate active tasks were selected in unspecified row order.
    -- Refuse an ambiguous policy instead of materializing an arbitrary deadline.
    IF opening_count > 1 OR closing_count > 1 OR invalid_dates THEN
        RAISE EXCEPTION 'ambiguous_or_invalid_voting_window for election %', target_election;
    END IF;

    -- Reject malformed dates while editing configuration, not during a vote.
    -- Keep the original text so Rust retains its existing boundary semantics.
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

CREATE FUNCTION sequent_backend.update_election_voting_window()
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
    RETURN NULL;
END;
$$;

CREATE TRIGGER update_election_voting_window
AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.scheduled_event
FOR EACH ROW EXECUTE FUNCTION sequent_backend.update_election_voting_window();

CREATE FUNCTION sequent_backend.clear_election_voting_windows()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    DELETE FROM sequent_backend.election_voting_window;
    RETURN NULL;
END;
$$;

CREATE TRIGGER clear_election_voting_windows
AFTER TRUNCATE ON sequent_backend.scheduled_event
FOR EACH STATEMENT EXECUTE FUNCTION sequent_backend.clear_election_voting_windows();

-- Backfill under the source-table lock: no schedule write can fall between
-- this snapshot and trigger installation. Any invalid policy aborts migration.
DO $$
DECLARE
    scope record;
BEGIN
    FOR scope IN
        SELECT DISTINCT tenant_id, election_event_id,
            sequent_backend.voting_window_election_id(schedule) AS election_id
        FROM sequent_backend.scheduled_event schedule
        WHERE sequent_backend.voting_window_election_id(schedule) IS NOT NULL
    LOOP
        PERFORM sequent_backend.refresh_election_voting_window(
            scope.tenant_id, scope.election_event_id, scope.election_id
        );
    END LOOP;
END;
$$;
