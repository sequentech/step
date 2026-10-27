-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Written in the same transaction as a security-state change. This outbox is
-- deliberately not tracked by GraphQL; human roles cannot suppress a notice.
CREATE TABLE sequent_backend.security_notification (
    id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    kind text NOT NULL,
    details text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    state text NOT NULL DEFAULT 'pending' CHECK (state IN ('pending','sending','sent','failed','uncertain')),
    attempted_at timestamptz,
    attempts integer NOT NULL DEFAULT 0 CHECK (attempts>=0),
    next_attempt_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    delivered_at timestamptz,
    failure text
);
CREATE INDEX security_notification_pending ON sequent_backend.security_notification(created_at) WHERE state='pending';

-- A short-lived mirror lets Hasura enforce the same condition without a
-- cross-database credential or a client-supplied health flag.
CREATE TABLE sequent_backend.audit_storage_status (
    singleton boolean PRIMARY KEY CHECK(singleton),
    state text NOT NULL CHECK(state IN ('available','threshold-exceeded','stale','unavailable','disabled')),
    generation bigint NOT NULL DEFAULT 1 CHECK(generation>0),
    queue_generation bigint,
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp()
);

CREATE OR REPLACE FUNCTION sequent_backend.audit_storage_allows_voters()
RETURNS boolean LANGUAGE sql STABLE AS $$
    SELECT EXISTS(SELECT 1 FROM sequent_backend.audit_storage_status
        WHERE singleton AND state IN ('available','disabled')
          AND updated_at<=statement_timestamp()
          AND updated_at>statement_timestamp()-interval '45 seconds');
$$;

-- An idempotent link between each storage-state transition and each event's
-- audit/email outboxes. The queue database is observed, never treated as a log.
CREATE TABLE sequent_backend.audit_storage_notice (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    generation bigint NOT NULL,
    PRIMARY KEY(tenant_id,election_event_id,generation)
);
