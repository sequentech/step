-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- The event policy is authoritative even if a client bypasses the portal's
-- Automatic Ceremony toggle. Missing event policy means manual ceremonies.
CREATE FUNCTION sequent_backend.guard_ceremony_event_policy()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, sequent_backend AS $$
DECLARE event_policy text; ceremony_policy text;
BEGIN
    SELECT COALESCE(presentation->>'ceremonies_policy', 'manual-ceremonies')
      INTO event_policy FROM sequent_backend.election_event
      WHERE tenant_id = NEW.tenant_id AND id = NEW.election_event_id FOR SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Ceremony event does not belong to this tenant'
          USING ERRCODE = '23503';
    END IF;
    IF TG_TABLE_NAME = 'keys_ceremony' THEN
        ceremony_policy := COALESCE(NEW.settings->>'policy', 'manual-ceremonies');
    ELSE
        SELECT COALESCE(settings->>'policy', 'manual-ceremonies')
          INTO ceremony_policy FROM sequent_backend.keys_ceremony
          WHERE tenant_id = NEW.tenant_id AND election_event_id = NEW.election_event_id
            AND id = NEW.keys_ceremony_id FOR SHARE;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'Tally keys ceremony does not belong to this tenant and event'
              USING ERRCODE = '23503';
        END IF;
    END IF;
    IF ceremony_policy NOT IN ('manual-ceremonies', 'automated-ceremonies')
       OR (ceremony_policy = 'automated-ceremonies' AND event_policy <> 'automated-ceremonies') THEN
        RAISE EXCEPTION 'Automatic ceremony is not permitted by the election event policy'
          USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END $$;

CREATE TRIGGER enforce_keys_event_ceremony_policy
BEFORE INSERT OR UPDATE OF tenant_id, election_event_id, settings, execution_status
ON sequent_backend.keys_ceremony FOR EACH ROW
EXECUTE FUNCTION sequent_backend.guard_ceremony_event_policy();

-- Also refuse a new tally using a historical automatic ceremony whose event
-- requires manual custody; never rewrite existing ceremonies or accepted data.
CREATE TRIGGER enforce_tally_event_ceremony_policy
BEFORE INSERT OR UPDATE OF tenant_id, election_event_id, keys_ceremony_id, execution_status
ON sequent_backend.tally_session FOR EACH ROW
EXECUTE FUNCTION sequent_backend.guard_ceremony_event_policy();

CREATE FUNCTION sequent_backend.guard_event_ceremony_policy_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, sequent_backend AS $$
BEGIN
    IF COALESCE(NEW.presentation->>'ceremonies_policy', 'manual-ceremonies')
         IS DISTINCT FROM COALESCE(OLD.presentation->>'ceremonies_policy', 'manual-ceremonies')
       AND COALESCE(NEW.presentation->>'ceremonies_policy', 'manual-ceremonies') <> 'automated-ceremonies'
       AND EXISTS(SELECT FROM sequent_backend.keys_ceremony
          WHERE tenant_id = NEW.tenant_id AND election_event_id = NEW.id
            AND settings->>'policy' = 'automated-ceremonies') THEN
        RAISE EXCEPTION 'Manual event policy conflicts with an existing automatic ceremony'
          USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END $$;

CREATE TRIGGER enforce_event_ceremony_policy_change
BEFORE UPDATE OF presentation ON sequent_backend.election_event FOR EACH ROW
EXECUTE FUNCTION sequent_backend.guard_event_ceremony_policy_change();
