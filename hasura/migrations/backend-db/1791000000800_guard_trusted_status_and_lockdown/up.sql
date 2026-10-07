-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Hasura lets admin roles write the whole `status` and `presentation` of
-- elections and election events, so an admin acting alone could open or
-- close voting, or lift the lockdown, without the server's checks, its
-- signatures or its electoral log entries. These triggers refuse an UPDATE
-- that changes those values, an INSERT that doesn't start from their
-- defaults, and a change of an election's grace period once its voting
-- started (the grace period lets votes in after a close), unless the server
-- marked its transaction
-- (`SET LOCAL sequent.trusted_write = 'on'`, windmill
-- `postgres::trusted_write`). Updates that keep them, such as an admin form
-- saving the whole record as loaded, pass, and every other key stays freely
-- writable.

-- A JSON timestamp as its instant, so that "…T00:00:00.000Z" and
-- "…T04:00:00+04:00" compare equal; anything else stays as it is.
CREATE OR REPLACE FUNCTION "sequent_backend"."trusted_write_instant"(value jsonb)
RETURNS jsonb
LANGUAGE plpgsql
STABLE
AS $$
BEGIN
    IF jsonb_typeof(value) IS DISTINCT FROM 'string' THEN
        RETURN value;
    END IF;
    RETURN to_jsonb(extract(epoch FROM (value #>> '{}')::timestamptz));
EXCEPTION
    WHEN others THEN
        RETURN value;
END;
$$;

-- The values of a `status` that let votes in: the voting status of each
-- channel (missing = NOT_STARTED, as ElectionStatus deserializes it) and its
-- period dates (the last close bounds grace-period votes). Missing and null
-- keys count as absent.
CREATE OR REPLACE FUNCTION "sequent_backend"."trusted_voting_state"(status jsonb)
RETURNS jsonb
LANGUAGE plpgsql
STABLE
AS $$
DECLARE
    state jsonb := '{}'::jsonb;
    channel text;
    dates_key text;
    dates jsonb;
    instants jsonb;
    entry record;
BEGIN
    IF jsonb_typeof(status) IS DISTINCT FROM 'object' THEN
        status := '{}'::jsonb;
    END IF;
    FOREACH channel IN ARRAY ARRAY[
        'voting_status', 'kiosk_voting_status', 'early_voting_status', 'telephone_voting_status'
    ] LOOP
        state := state || jsonb_build_object(
            channel,
            COALESCE(NULLIF(status -> channel, 'null'::jsonb), '"NOT_STARTED"'::jsonb)
        );
        dates_key := replace(channel, '_status', '_period_dates');
        dates := status -> dates_key;
        instants := '{}'::jsonb;
        IF jsonb_typeof(dates) = 'object' THEN
            FOR entry IN SELECT key, value FROM jsonb_each(dates) LOOP
                CONTINUE WHEN entry.value = 'null'::jsonb;
                instants := instants || jsonb_build_object(
                    entry.key, "sequent_backend"."trusted_write_instant"(entry.value)
                );
            END LOOP;
        ELSIF dates IS NOT NULL AND dates <> 'null'::jsonb THEN
            instants := dates;
        END IF;
        state := state || jsonb_build_object(dates_key, instants);
    END LOOP;
    RETURN state;
END;
$$;

CREATE OR REPLACE FUNCTION "sequent_backend"."trusted_write_allowed"()
RETURNS boolean
LANGUAGE sql
STABLE
AS $$
    SELECT COALESCE(current_setting('sequent.trusted_write', true), '') = 'on';
$$;

-- Whether voting started on any channel of a `status`.
CREATE OR REPLACE FUNCTION "sequent_backend"."trusted_voting_started"(status jsonb)
RETURNS boolean
LANGUAGE sql
STABLE
AS $$
    SELECT EXISTS (
        SELECT 1
        FROM jsonb_each("sequent_backend"."trusted_voting_state"(status)) AS entry
        WHERE (entry.key LIKE '%voting_status'
               AND entry.value IS DISTINCT FROM '"NOT_STARTED"'::jsonb)
           OR (entry.key LIKE '%voting_period_dates' AND entry.value <> '{}'::jsonb)
    );
$$;

-- An election's grace period settings, as ElectionPresentation reads them:
-- no policy = no grace period, no seconds = 0.
CREATE OR REPLACE FUNCTION "sequent_backend"."trusted_grace_period"(presentation jsonb)
RETURNS jsonb
LANGUAGE sql
STABLE
AS $$
    SELECT jsonb_build_object(
        'grace_period_policy',
        COALESCE(NULLIF(presentation -> 'grace_period_policy', 'null'::jsonb), '"no-grace-period"'::jsonb),
        'grace_period_secs',
        COALESCE(NULLIF(presentation -> 'grace_period_secs', 'null'::jsonb), '0'::jsonb)
    );
$$;

-- Updates must keep the voting state; inserts must start from the default
-- one (voting not started on any channel, no period dates).
CREATE OR REPLACE FUNCTION "sequent_backend"."guard_trusted_voting_status_update"()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    old_status jsonb := NULL;
BEGIN
    IF TG_OP = 'UPDATE' THEN
        old_status := OLD.status::jsonb;
    END IF;
    IF "sequent_backend"."trusted_voting_state"(old_status)
        IS NOT DISTINCT FROM
       "sequent_backend"."trusted_voting_state"(NEW.status::jsonb)
       OR "sequent_backend"."trusted_write_allowed"() THEN
        RETURN NEW;
    END IF;

    IF TG_OP = 'INSERT' THEN
        RAISE EXCEPTION 'A new % can only start with voting not started: its voting status can only change through the voting status actions',
            TG_TABLE_NAME
            USING ERRCODE = '42501',
                  HINT = 'Create it without a voting status, then open voting from the Publish tab or a scheduled event.';
    END IF;
    RAISE EXCEPTION 'The voting status of % % can only change through the voting status actions',
        TG_TABLE_NAME, OLD.id
        USING ERRCODE = '42501',
              HINT = 'Open, pause or close voting from the Publish tab or a scheduled event.';
END;
$$;

CREATE OR REPLACE FUNCTION "sequent_backend"."guard_trusted_lockdown_update"()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    old_presentation jsonb := NULL;
BEGIN
    IF TG_OP = 'UPDATE' THEN
        old_presentation := OLD.presentation::jsonb;
    END IF;
    -- A missing key is not locked down, as ElectionEventPresentation reads it.
    IF COALESCE(NULLIF(old_presentation -> 'locked_down', 'null'::jsonb), '"not-locked-down"'::jsonb)
        IS NOT DISTINCT FROM
       COALESCE(NULLIF(NEW.presentation::jsonb -> 'locked_down', 'null'::jsonb), '"not-locked-down"'::jsonb)
       OR "sequent_backend"."trusted_write_allowed"() THEN
        RETURN NEW;
    END IF;

    IF TG_OP = 'INSERT' THEN
        RAISE EXCEPTION 'A new election event can only start not locked down: its lockdown can only change through a scheduled lockdown event'
            USING ERRCODE = '42501',
                  HINT = 'Schedule a start of the lockdown period.';
    END IF;
    RAISE EXCEPTION 'The lockdown of election event % can only change through a scheduled lockdown event',
        OLD.id
        USING ERRCODE = '42501',
              HINT = 'Schedule a start or end of the lockdown period.';
END;
$$;

-- The grace period lets votes in after a close, so once voting started on a
-- channel of the election it can't change outside the server.
CREATE OR REPLACE FUNCTION "sequent_backend"."guard_trusted_grace_period_update"()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF "sequent_backend"."trusted_grace_period"(OLD.presentation::jsonb)
        IS NOT DISTINCT FROM
       "sequent_backend"."trusted_grace_period"(NEW.presentation::jsonb)
       OR NOT (
           "sequent_backend"."trusted_voting_started"(OLD.status::jsonb)
           OR "sequent_backend"."trusted_voting_started"(NEW.status::jsonb)
       )
       OR "sequent_backend"."trusted_write_allowed"() THEN
        RETURN NEW;
    END IF;

    RAISE EXCEPTION 'The grace period of election % can only change before voting starts',
        OLD.id
        USING ERRCODE = '42501',
              HINT = 'Set the grace period before opening voting.';
END;
$$;

DROP TRIGGER IF EXISTS "guard_trusted_voting_status_update"
ON "sequent_backend"."election";

CREATE TRIGGER "guard_trusted_voting_status_update"
BEFORE UPDATE OF status
ON "sequent_backend"."election"
FOR EACH ROW
EXECUTE FUNCTION "sequent_backend"."guard_trusted_voting_status_update"();

DROP TRIGGER IF EXISTS "guard_trusted_voting_status_insert"
ON "sequent_backend"."election";

CREATE TRIGGER "guard_trusted_voting_status_insert"
BEFORE INSERT
ON "sequent_backend"."election"
FOR EACH ROW
EXECUTE FUNCTION "sequent_backend"."guard_trusted_voting_status_update"();

DROP TRIGGER IF EXISTS "guard_trusted_grace_period_update"
ON "sequent_backend"."election";

CREATE TRIGGER "guard_trusted_grace_period_update"
BEFORE UPDATE OF presentation
ON "sequent_backend"."election"
FOR EACH ROW
EXECUTE FUNCTION "sequent_backend"."guard_trusted_grace_period_update"();

DROP TRIGGER IF EXISTS "guard_trusted_voting_status_update"
ON "sequent_backend"."election_event";

CREATE TRIGGER "guard_trusted_voting_status_update"
BEFORE UPDATE OF status
ON "sequent_backend"."election_event"
FOR EACH ROW
EXECUTE FUNCTION "sequent_backend"."guard_trusted_voting_status_update"();

DROP TRIGGER IF EXISTS "guard_trusted_voting_status_insert"
ON "sequent_backend"."election_event";

CREATE TRIGGER "guard_trusted_voting_status_insert"
BEFORE INSERT
ON "sequent_backend"."election_event"
FOR EACH ROW
EXECUTE FUNCTION "sequent_backend"."guard_trusted_voting_status_update"();

DROP TRIGGER IF EXISTS "guard_trusted_lockdown_update"
ON "sequent_backend"."election_event";

CREATE TRIGGER "guard_trusted_lockdown_update"
BEFORE UPDATE OF presentation
ON "sequent_backend"."election_event"
FOR EACH ROW
EXECUTE FUNCTION "sequent_backend"."guard_trusted_lockdown_update"();

DROP TRIGGER IF EXISTS "guard_trusted_lockdown_insert"
ON "sequent_backend"."election_event";

CREATE TRIGGER "guard_trusted_lockdown_insert"
BEFORE INSERT
ON "sequent_backend"."election_event"
FOR EACH ROW
EXECUTE FUNCTION "sequent_backend"."guard_trusted_lockdown_update"();
