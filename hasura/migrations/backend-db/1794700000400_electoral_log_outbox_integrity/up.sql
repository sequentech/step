-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- When a row was written is the database's, never its writer's: the batch task
-- checks a statement's time against it (freshness), so a row inserted later
-- with an old statement is refused. A writer's created_at is replaced, and an
-- update can't change it.
CREATE OR REPLACE FUNCTION sequent_backend.electoral_log_outbox_database_time()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        NEW.created_at := clock_timestamp();
    ELSE
        NEW.created_at := OLD.created_at;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER electoral_log_outbox_database_time
    BEFORE INSERT OR UPDATE OF created_at ON sequent_backend.electoral_log_outbox
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.electoral_log_outbox_database_time();

-- An administrator's signature is attested once: a second row carrying the
-- same signed statement (a copy under another delivery ID) is refused, even
-- after the first was relayed or cancelled.
CREATE UNIQUE INDEX electoral_log_outbox_admin_signature
    ON sequent_backend.electoral_log_outbox ((payload -> 'statement' ->> 'admin_signature'))
    WHERE payload -> 'statement' ->> 'kind' = 'admin-certificate-event';

-- A row the relay can never move (unreadable, a MAC or announcement that
-- doesn't verify) is dead-lettered: kept, alerted on, no longer heading its
-- log; an operator replays it (once fixed) or cancels it.
ALTER TABLE sequent_backend.electoral_log_outbox
    ADD COLUMN dead_lettered_at timestamptz,
    ADD COLUMN dead_letter_reason text;
