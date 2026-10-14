-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Restore the narrower voting/initialization/boundary guard from 1200.
CREATE OR REPLACE FUNCTION sequent_backend.guard_election_lifecycle_identity()
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
