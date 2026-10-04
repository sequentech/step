-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

CREATE EXTENSION IF NOT EXISTS pg_stat_statements;
CREATE SCHEMA sequent_backend;

-- Minimal database contracts exercised by the production helpers. This fixture
-- deliberately excludes external services and cryptography from SQL timings.
CREATE TABLE sequent_backend.election (
    id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    num_allowed_revotes integer DEFAULT 3,
    presentation jsonb,
    status jsonb,
    voting_channels jsonb,
    eml text
);
CREATE TABLE sequent_backend.scheduled_event (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id uuid,
    election_event_id uuid,
    created_at timestamptz DEFAULT now(),
    stopped_at timestamptz,
    archived_at timestamptz,
    labels jsonb,
    annotations jsonb,
    event_processor text,
    cron_config jsonb,
    event_payload jsonb,
    task_id text
);
CREATE TABLE sequent_backend.cast_vote (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    election_id uuid NOT NULL,
    voter_id_string text,
    area_id uuid,
    status text NOT NULL DEFAULT 'valid',
    content text,
    ballot_id text,
    cast_ballot_signature bytea,
    annotations jsonb,
    created_at timestamptz DEFAULT now(),
    last_updated_at timestamptz DEFAULT now()
);
CREATE INDEX cast_vote_participation_election_idx
    ON sequent_backend.cast_vote (tenant_id, election_event_id, election_id, voter_id_string);

CREATE TABLE sequent_backend.area (
    id uuid PRIMARY KEY, tenant_id uuid, election_event_id uuid, presentation jsonb
);
CREATE TABLE sequent_backend.election_event (
    id uuid PRIMARY KEY, tenant_id uuid, presentation jsonb, bulletin_board_reference text
);
CREATE TABLE sequent_backend.secret (
    tenant_id uuid, election_event_id uuid, key text, value text,
    PRIMARY KEY (tenant_id, election_event_id, key)
);

-- This narrow timing fixture has no configuration approvals. Keep the empty
-- private boundary projection required by the production point-read query;
-- full migration-backed signed-approval controls live in the backend tests.
CREATE TABLE sequent_backend.signed_voting_boundary (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    election_id uuid NOT NULL,
    approval_request_id uuid NOT NULL,
    scheduled_event_id text NOT NULL,
    event_processor text NOT NULL,
    channels jsonb NOT NULL,
    source_timezone text,
    scheduled_date text NOT NULL,
    scheduled_at timestamptz NOT NULL,
    fingerprint text,
    PRIMARY KEY (tenant_id, election_event_id, election_id, scheduled_event_id)
);
CREATE INDEX signed_voting_boundary_bound ON sequent_backend.signed_voting_boundary
    (tenant_id, election_event_id, election_id, event_processor, scheduled_at);

-- Match the execution proof consumed by the production point-read query.
-- The timing fixtures remain unsigned, so this private projection starts empty.
CREATE TABLE sequent_backend.lifecycle_fired (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    scheduled_event_id uuid NOT NULL,
    election_id uuid NOT NULL,
    fingerprint text NOT NULL CHECK (fingerprint ~ '^[0-9a-f]{64}$'),
    fired_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    executed_channels jsonb CHECK (executed_channels IS NULL OR (
        jsonb_typeof(executed_channels) = 'array'
        AND executed_channels <@ '["ONLINE", "KIOSK", "EARLY_VOTING", "TELEPHONE"]'::jsonb
    ))
);

-- These unsigned timing fixtures use the default optional closing rule.
-- Production authorization (including both-copy signature policies) is
-- exercised with the complete migrations in the backend regression suite.
CREATE FUNCTION sequent_backend.live_voting_close_allowed(uuid, uuid, uuid)
RETURNS boolean LANGUAGE sql STABLE AS $$ SELECT true $$;
