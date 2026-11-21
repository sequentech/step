-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Electoral-log entries an action records in its own transaction, already signed
-- by the person who acted (a pending statement, `payload`), relayed to the
-- electoral log only after that transaction commits: an action that rolls back
-- leaves no entry, by construction. The relay (Windmill's
-- `services::electoral_log_outbox`) moves each row, in `seq` order per log, to the
-- task-queue database's waiting table, then marks it relayed. Rows are never
-- signed or stored from here: the electoral log's batch task does that.
CREATE TABLE sequent_backend.electoral_log_outbox (
    delivery_id text NOT NULL,
    seq bigserial NOT NULL,
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    payload jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    relayed_at timestamptz,
    attempts integer NOT NULL DEFAULT 0,
    last_error text,
    PRIMARY KEY (delivery_id),
    CONSTRAINT electoral_log_outbox_seq_key UNIQUE (seq),
    CONSTRAINT electoral_log_outbox_attempts_count CHECK (attempts >= 0)
);

-- What the relay still has to move.
CREATE INDEX electoral_log_outbox_unrelayed
    ON sequent_backend.electoral_log_outbox (seq)
    WHERE relayed_at IS NULL;
