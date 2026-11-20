-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- An operator may cancel an outbox entry that can't be relayed (who, when, why
-- are kept). The announcement of the administrator's key, signed with the tenant
-- log's system key in the action's transaction, travels with the entry, so the
-- relay forwards it without reading any key.
ALTER TABLE sequent_backend.electoral_log_outbox
    ADD COLUMN cancelled_at timestamptz,
    ADD COLUMN cancelled_by text,
    ADD COLUMN cancel_reason text,
    ADD COLUMN announcement bytea;

-- A log's direct posts are refused while it has an entry not yet relayed: they
-- look it up by the log's scope.
CREATE INDEX electoral_log_outbox_scope_waiting
    ON sequent_backend.electoral_log_outbox (tenant_id, election_event_id)
    WHERE relayed_at IS NULL AND cancelled_at IS NULL;

DROP INDEX IF EXISTS sequent_backend.electoral_log_outbox_unrelayed;
CREATE INDEX electoral_log_outbox_unrelayed
    ON sequent_backend.electoral_log_outbox (tenant_id, election_event_id, seq)
    WHERE relayed_at IS NULL AND cancelled_at IS NULL;
