-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Neither column is exposed by any Hasura role. Public annotations and
-- scheduled-event payloads are writable input and cannot carry authority.
ALTER TABLE sequent_backend.tasks_execution ADD COLUMN participation_authorization jsonb;
ALTER TABLE sequent_backend.scheduled_event ADD COLUMN participation_authorization jsonb;

CREATE FUNCTION sequent_backend.protect_participation_command() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF OLD.participation_authorization IS NOT NULL AND (
        NEW.participation_authorization IS DISTINCT FROM OLD.participation_authorization
        OR NEW.id IS DISTINCT FROM OLD.id
        OR NEW.tenant_id IS DISTINCT FROM OLD.tenant_id
        OR NEW.election_event_id IS DISTINCT FROM OLD.election_event_id
    ) THEN
        RAISE EXCEPTION 'Participation authorization binding is immutable';
    END IF;
    IF TG_TABLE_NAME = 'scheduled_event' AND OLD.participation_authorization IS NOT NULL THEN
        IF NEW.event_payload IS DISTINCT FROM OLD.event_payload
            OR NEW.created_by IS DISTINCT FROM OLD.created_by
            OR NEW.event_processor IS DISTINCT FROM OLD.event_processor THEN
            RAISE EXCEPTION 'Authorized send content is immutable; create a new send';
        END IF;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER protect_participation_command
BEFORE UPDATE ON sequent_backend.tasks_execution FOR EACH ROW
EXECUTE FUNCTION sequent_backend.protect_participation_command();
CREATE TRIGGER protect_participation_command
BEFORE UPDATE ON sequent_backend.scheduled_event FOR EACH ROW
EXECUTE FUNCTION sequent_backend.protect_participation_command();

CREATE FUNCTION sequent_backend.protect_participation_document() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF (OLD.annotations ? 'participation_scope'
        OR OLD.annotations->>'purpose' = 'voter-export'
        OR (OLD.election_event_id IS NOT NULL AND (OLD.name LIKE 'users-export-%'
            OR OLD.name LIKE 'election-event-%-export-sha256-%')))
       AND (NEW.id IS DISTINCT FROM OLD.id
         OR NEW.tenant_id IS DISTINCT FROM OLD.tenant_id
         OR NEW.election_event_id IS DISTINCT FROM OLD.election_event_id
         OR NEW.name IS DISTINCT FROM OLD.name
         OR NEW.is_public IS DISTINCT FROM OLD.is_public
         OR NEW.annotations->>'purpose' IS DISTINCT FROM OLD.annotations->>'purpose'
         OR NEW.annotations->'participation_scope' IS DISTINCT FROM OLD.annotations->'participation_scope') THEN
        RAISE EXCEPTION 'Voter export participation scope is immutable';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER protect_participation_document
BEFORE UPDATE ON sequent_backend.document FOR EACH ROW
EXECUTE FUNCTION sequent_backend.protect_participation_document();
