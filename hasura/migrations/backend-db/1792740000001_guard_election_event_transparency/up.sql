-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only

CREATE OR REPLACE FUNCTION "sequent_backend"."guard_election_event_transparency_update"()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    hasura_session text;
BEGIN
    IF (OLD.presentation::jsonb -> 'transparency')
        IS NOT DISTINCT FROM
       (NEW.presentation::jsonb -> 'transparency')
       AND (OLD.presentation::jsonb -> 'vitrine')
        IS NOT DISTINCT FROM
       (NEW.presentation::jsonb -> 'vitrine') THEN
        RETURN NEW;
    END IF;

    hasura_session := current_setting('hasura.user', true);
    IF hasura_session IS NULL OR hasura_session = '' THEN
        RETURN NEW;
    END IF;

    IF "sequent_backend"."vitrine_voting_opened"(OLD.tenant_id, OLD.id) THEN
        RAISE EXCEPTION 'The transparency mode and the Vitrine logs and policy of an election event can''t change once voting opened'
            USING ERRCODE = '42501';
    END IF;

    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS "guard_election_event_transparency_update"
ON "sequent_backend"."election_event";

CREATE TRIGGER "guard_election_event_transparency_update"
BEFORE UPDATE OF presentation
ON "sequent_backend"."election_event"
FOR EACH ROW
EXECUTE FUNCTION "sequent_backend"."guard_election_event_transparency_update"();
