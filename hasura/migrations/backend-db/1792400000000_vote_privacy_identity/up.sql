-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Historical configurations and signed ballots retain their identity semantics.
UPDATE sequent_backend.election_event
SET presentation = jsonb_set(COALESCE(presentation, '{}'::jsonb), '{ballot_identity}', '"voter-id"'::jsonb)
WHERE presentation->>'ballot_identity' IS NULL;

CREATE FUNCTION sequent_backend.check_ballot_privacy_identity() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE mode text := COALESCE(NEW.presentation->>'ballot_identity', 'voter-id');
BEGIN
    IF mode NOT IN ('voter-id', 'pseudonym') THEN
        RAISE EXCEPTION 'Invalid ballot identity mode';
    END IF;
    IF TG_OP = 'UPDATE' AND mode <> COALESCE(OLD.presentation->>'ballot_identity', 'voter-id') THEN
        RAISE EXCEPTION 'Ballot identity mode is immutable';
    END IF;
    IF COALESCE(NEW.presentation->>'network_data_policy', 'do-not-store') NOT IN ('do-not-store', 'forensic-store') THEN
        RAISE EXCEPTION 'Invalid network data policy';
    END IF;
    IF mode = 'voter-id' AND NEW.presentation->>'network_data_policy' = 'forensic-store' THEN
        RAISE EXCEPTION 'Forensic network attribution requires a pseudonymous event';
    END IF;
    IF mode = 'pseudonym' THEN
        IF NEW.annotations ? 'datafix:id' THEN
            RAISE EXCEPTION 'External voter reconciliation requires voter-id mode';
        END IF;
        IF COALESCE(NEW.presentation->>'voter_signing_policy', '') <> 'with-signature'
           OR COALESCE(NEW.presentation#>>'{receipts,policy}', '') <> 'signed-by-ballot-box' THEN
            RAISE EXCEPTION 'Pseudonyms require signed receipts and voter signatures';
        END IF;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER check_ballot_privacy_identity BEFORE INSERT OR UPDATE ON sequent_backend.election_event
FOR EACH ROW EXECUTE FUNCTION sequent_backend.check_ballot_privacy_identity();
