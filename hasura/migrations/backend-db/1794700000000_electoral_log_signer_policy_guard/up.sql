-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only

-- An election event's electoral_log_signer_policy can't change once any of
-- its boards has a key (software or external): the key was made for the
-- setting, and every later signature and verification relies on it. The
-- server also refuses to use a board whose setting disagrees with its key;
-- this refuses the edit itself, so an administrator can't stop an event's
-- logging by mistake.
CREATE OR REPLACE FUNCTION sequent_backend.electoral_log_signer_policy_of(presentation jsonb)
RETURNS text
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT COALESCE(
        CASE jsonb_typeof(presentation)
            WHEN 'object' THEN presentation ->> 'electoral_log_signer_policy'
            WHEN 'string' THEN (presentation #>> '{}')::jsonb ->> 'electoral_log_signer_policy'
        END,
        'software'
    )
$$;

CREATE OR REPLACE FUNCTION sequent_backend.guard_electoral_log_signer_policy()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF sequent_backend.electoral_log_signer_policy_of(OLD.presentation)
           IS DISTINCT FROM sequent_backend.electoral_log_signer_policy_of(NEW.presentation)
       AND EXISTS (
           SELECT 1
           FROM sequent_backend.secret s
           WHERE s.tenant_id = NEW.tenant_id
             AND s.election_event_id = NEW.id
             AND s.key LIKE 'boards/%'
       )
    THEN
        RAISE EXCEPTION 'electoral_log_signer_policy can''t change once the event''s boards have keys';
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS electoral_log_signer_policy_guard ON sequent_backend.election_event;
CREATE TRIGGER electoral_log_signer_policy_guard
    BEFORE UPDATE OF presentation ON sequent_backend.election_event
    FOR EACH ROW
    EXECUTE FUNCTION sequent_backend.guard_electoral_log_signer_policy();
