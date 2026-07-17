CREATE OR REPLACE FUNCTION "sequent_backend"."guard_results_website_policy_update"()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    hasura_session text;
    hasura_role text;
BEGIN
    IF (OLD.presentation::jsonb -> 'results_website')
        IS NOT DISTINCT FROM
       (NEW.presentation::jsonb -> 'results_website') THEN
        RETURN NEW;
    END IF;

    hasura_session := current_setting('hasura.user', true);
    IF hasura_session IS NULL OR hasura_session = '' THEN
        RETURN NEW;
    END IF;

    hasura_role := hasura_session::jsonb ->> 'x-hasura-role';
    IF hasura_role NOT IN ('publish-results-write', 'service-account') THEN
        RAISE EXCEPTION 'Changing results_website requires publish-results-write'
            USING ERRCODE = '42501';
    END IF;

    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS "guard_results_website_policy_update"
ON "sequent_backend"."election_event";

CREATE TRIGGER "guard_results_website_policy_update"
BEFORE UPDATE OF presentation
ON "sequent_backend"."election_event"
FOR EACH ROW
EXECUTE FUNCTION "sequent_backend"."guard_results_website_policy_update"();
