-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Hasura admin roles can change an election's identity and parent event.
-- Keeping its OPEN status while doing so must not detach voting from the
-- event that authorized it, its initialization or its signed deadline.
CREATE FUNCTION sequent_backend.guard_election_lifecycle_identity()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF ROW(OLD.id, OLD.tenant_id, OLD.election_event_id)
        IS NOT DISTINCT FROM ROW(NEW.id, NEW.tenant_id, NEW.election_event_id)
       OR sequent_backend.trusted_write_allowed() THEN
        RETURN NEW;
    END IF;
    IF sequent_backend.trusted_voting_started(OLD.status::jsonb)
       OR sequent_backend.trusted_voting_started(NEW.status::jsonb)
       OR COALESCE(OLD.initialization_report_generated, false)
       OR COALESCE(NEW.initialization_report_generated, false)
       OR EXISTS (
           SELECT 1 FROM sequent_backend.election_initialization
           WHERE tenant_id = OLD.tenant_id AND election_event_id = OLD.election_event_id
             AND election_id = OLD.id
       )
       OR EXISTS (
           SELECT 1 FROM sequent_backend.signed_voting_boundary
           WHERE tenant_id = OLD.tenant_id AND election_event_id = OLD.election_event_id
             AND election_id = OLD.id
       ) THEN
        RAISE EXCEPTION 'A Post with voting or initialization evidence can only change identity or event through a trusted server transaction'
            USING ERRCODE = '42501',
                  HINT = 'Keep the original Post and event. Create a new Post for a different event.';
    END IF;
    RETURN NEW;
END;
$$;

-- BEFORE precedes projection refresh and FK checks. It takes no parent or
-- advisory lock: the child row is already locked by UPDATE. Trusted server
-- paths retain their existing event-first lock order, and projection refresh
-- still refuses advisory-lock contention instead of waiting in reverse order.
CREATE TRIGGER guard_election_lifecycle_identity
BEFORE UPDATE OF id, tenant_id, election_event_id ON sequent_backend.election
FOR EACH ROW EXECUTE FUNCTION sequent_backend.guard_election_lifecycle_identity();

-- A legitimate trusted identity change must refresh the projection too;
-- otherwise the old id keeps the signed bound and the new id has none.
DROP TRIGGER update_signed_voting_election ON sequent_backend.election;
CREATE TRIGGER update_signed_voting_election
AFTER INSERT OR UPDATE OF id, tenant_id, election_event_id OR DELETE
ON sequent_backend.election
FOR EACH ROW EXECUTE FUNCTION sequent_backend.update_signed_voting_election();
