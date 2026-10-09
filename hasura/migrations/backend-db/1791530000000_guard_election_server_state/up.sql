-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- An election's voting status, its key ceremony and, once a channel has
-- started voting, that channel's switch and the revote limit are written by
-- the server over its own database connections. Hasura requests from roles
-- other than service-account (or the admin secret) can still change
-- allow_tally and save these values unchanged.

CREATE OR REPLACE FUNCTION "sequent_backend"."election_server_status"(status jsonb)
RETURNS jsonb
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT CASE jsonb_typeof(COALESCE(status, 'null'::jsonb))
        WHEN 'null' THEN '{}'::jsonb
        WHEN 'object' THEN status - 'allow_tally'
        ELSE status
    END;
$$;

CREATE OR REPLACE FUNCTION "sequent_backend"."guard_election_server_state"()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    hasura_session text;
    old_status jsonb;
    old_voting_channels jsonb;
    old_num_allowed_revotes integer;
    old_keys_ceremony_id uuid;
    channel record;
    voting_started boolean := false;
BEGIN
    hasura_session := current_setting('hasura.user', true);
    IF hasura_session IS NULL OR hasura_session = ''
       OR hasura_session::jsonb ->> 'x-hasura-role' IN ('admin', 'service-account') THEN
        RETURN NEW;
    END IF;

    IF TG_OP = 'UPDATE' THEN
        old_status := OLD.status;
        old_voting_channels := OLD.voting_channels;
        old_num_allowed_revotes := OLD.num_allowed_revotes;
        old_keys_ceremony_id := OLD.keys_ceremony_id;
    END IF;

    IF "sequent_backend"."election_server_status"(old_status)
        IS DISTINCT FROM
       "sequent_backend"."election_server_status"(NEW.status) THEN
        RAISE EXCEPTION 'The status of election % can only change through the voting status actions',
            NEW.id
            USING ERRCODE = '42501';
    END IF;

    IF old_keys_ceremony_id IS DISTINCT FROM NEW.keys_ceremony_id THEN
        RAISE EXCEPTION 'The key ceremony of election % can only change through the keys ceremony',
            NEW.id
            USING ERRCODE = '42501';
    END IF;

    FOR channel IN
        SELECT status_key, channel_key
        FROM (VALUES
            ('voting_status', 'online'),
            ('kiosk_voting_status', 'kiosk'),
            ('early_voting_status', 'early_voting'),
            ('telephone_voting_status', 'telephone')
        ) AS channels(status_key, channel_key)
    LOOP
        CONTINUE WHEN jsonb_typeof(old_status) IS DISTINCT FROM 'object'
            OR COALESCE(old_status -> channel.status_key, 'null'::jsonb)
                IN ('null'::jsonb, '"NOT_STARTED"'::jsonb);
        voting_started := true;
        IF COALESCE((old_voting_channels -> channel.channel_key) = 'true'::jsonb, false)
            IS DISTINCT FROM
           COALESCE((NEW.voting_channels -> channel.channel_key) = 'true'::jsonb, false) THEN
            RAISE EXCEPTION 'The % channel of election % cannot be switched after its voting started',
                channel.channel_key, NEW.id
                USING ERRCODE = '42501';
        END IF;
    END LOOP;

    IF voting_started
       AND COALESCE(old_num_allowed_revotes, 1) IS DISTINCT FROM COALESCE(NEW.num_allowed_revotes, 1) THEN
        RAISE EXCEPTION 'The revote limit of election % cannot change after its voting started',
            NEW.id
            USING ERRCODE = '42501';
    END IF;

    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS "guard_election_server_state"
ON "sequent_backend"."election";

CREATE TRIGGER "guard_election_server_state"
BEFORE INSERT OR UPDATE OF status, keys_ceremony_id, voting_channels, num_allowed_revotes
ON "sequent_backend"."election"
FOR EACH ROW
EXECUTE FUNCTION "sequent_backend"."guard_election_server_state"();
