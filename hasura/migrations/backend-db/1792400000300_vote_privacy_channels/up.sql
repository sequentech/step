-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Telephone voting cannot produce the signatures pseudonymous ballots need.
-- Enforce the invariant for both lifecycle settings and direct child edits.
CREATE FUNCTION sequent_backend.check_event_privacy_channels() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.presentation->>'ballot_identity' = 'pseudonym' AND (
        NEW.voting_channels->>'telephone' = 'true' OR EXISTS (
            SELECT 1 FROM sequent_backend.election child
            WHERE child.election_event_id = NEW.id AND child.tenant_id = NEW.tenant_id
              AND child.voting_channels->>'telephone' = 'true'
        )
    ) THEN
        RAISE EXCEPTION 'Pseudonymous events cannot enable telephone voting';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER check_event_privacy_channels
BEFORE INSERT OR UPDATE ON sequent_backend.election_event
FOR EACH ROW EXECUTE FUNCTION sequent_backend.check_event_privacy_channels();

CREATE FUNCTION sequent_backend.check_election_privacy_channels() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.voting_channels->>'telephone' = 'true' AND EXISTS (
        SELECT 1 FROM sequent_backend.election_event event
        WHERE event.id = NEW.election_event_id AND event.tenant_id = NEW.tenant_id
          AND event.presentation->>'ballot_identity' = 'pseudonym'
    ) THEN
        RAISE EXCEPTION 'Pseudonymous elections cannot enable telephone voting';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER check_election_privacy_channels
BEFORE INSERT OR UPDATE ON sequent_backend.election
FOR EACH ROW EXECUTE FUNCTION sequent_backend.check_election_privacy_channels();
