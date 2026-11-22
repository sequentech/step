-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- tally_session.last_updated_at was only set when the row was created, so it
-- did not show when a tally session last changed. The autoscaler queries for
-- the tally workers and trustees read it to tell which sessions are active.
-- Set it on every update, as keys_ceremony already does.

DROP TRIGGER IF EXISTS "set_sequent_backend_tally_session_last_updated_at"
ON "sequent_backend"."tally_session";

CREATE TRIGGER "set_sequent_backend_tally_session_last_updated_at"
BEFORE UPDATE ON "sequent_backend"."tally_session"
FOR EACH ROW
EXECUTE PROCEDURE "sequent_backend"."set_current_timestamp_last_updated_at"();

COMMENT ON TRIGGER "set_sequent_backend_tally_session_last_updated_at" ON "sequent_backend"."tally_session"
IS 'trigger to set value of column "last_updated_at" to current timestamp on row update';
