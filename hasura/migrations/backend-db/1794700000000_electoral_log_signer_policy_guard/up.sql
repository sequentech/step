-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only

-- An election event's electoral_log_signer_policy can't change once any of
-- its boards has a key (software or external): the key was made for the
-- setting, and every later signature and verification relies on it. The
-- server also refuses to use a board whose setting disagrees with its key;
-- this refuses the edit itself, so an administrator can't stop an event's
-- logging by mistake.

-- The setting in a stored presentation, read as the server reads it:
-- missing or null is 'software'; a presentation the server can't read (a
-- string that is not JSON, or anything but an object) is 'unreadable'.
CREATE OR REPLACE FUNCTION sequent_backend.electoral_log_signer_policy_of(presentation jsonb)
RETURNS text
LANGUAGE plpgsql
IMMUTABLE
AS $$
DECLARE
    inner_value jsonb;
BEGIN
    CASE COALESCE(jsonb_typeof(presentation), 'null')
        WHEN 'null' THEN
            RETURN 'software';
        WHEN 'object' THEN
            RETURN COALESCE(presentation ->> 'electoral_log_signer_policy', 'software');
        WHEN 'string' THEN
            BEGIN
                inner_value := (presentation #>> '{}')::jsonb;
            EXCEPTION WHEN others THEN
                RETURN 'unreadable';
            END;
            RETURN sequent_backend.electoral_log_signer_policy_of(inner_value);
        ELSE
            RETURN 'unreadable';
    END CASE;
END
$$;

CREATE OR REPLACE FUNCTION sequent_backend.guard_electoral_log_signer_policy()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    has_keys boolean;
    made_with text;
    old_policy text;
    new_policy text;
BEGIN
    SELECT EXISTS (
               SELECT 1 FROM sequent_backend.secret s
               WHERE s.tenant_id = NEW.tenant_id
                 AND s.election_event_id = NEW.id
                 AND s.key LIKE 'boards/%'
           ),
           CASE WHEN EXISTS (
               SELECT 1 FROM sequent_backend.secret s
               WHERE s.tenant_id = NEW.tenant_id
                 AND s.election_event_id = NEW.id
                 AND s.key LIKE 'boards/%/external-signer'
           ) THEN 'external' ELSE 'software' END
      INTO has_keys, made_with;
    IF NOT has_keys THEN
        RETURN NEW;
    END IF;
    new_policy := sequent_backend.electoral_log_signer_policy_of(NEW.presentation);
    -- The setting agrees with the keys (also: a malformed row repaired).
    IF new_policy = made_with THEN
        RETURN NEW;
    END IF;
    -- A row already unreadable stays editable for other changes; the server
    -- refuses to sign for it until it is repaired.
    old_policy := sequent_backend.electoral_log_signer_policy_of(OLD.presentation);
    IF new_policy = 'unreadable' AND old_policy = 'unreadable' THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'electoral_log_signer_policy can''t change once the event''s boards have keys (they were made with %)', made_with;
END
$$;

DROP TRIGGER IF EXISTS electoral_log_signer_policy_guard ON sequent_backend.election_event;
CREATE TRIGGER electoral_log_signer_policy_guard
    BEFORE UPDATE OF presentation ON sequent_backend.election_event
    FOR EACH ROW
    WHEN (OLD.presentation IS DISTINCT FROM NEW.presentation)
    EXECUTE FUNCTION sequent_backend.guard_electoral_log_signer_policy();
