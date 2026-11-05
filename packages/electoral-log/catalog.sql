-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Catalog of the election events' electoral-log databases, in the base database
-- (ELECTORAL_LOG_PG_DATABASE). Each event's board, the logs it continues and its
-- ballot box are in the database named here.
BEGIN;
-- Serializes concurrent initializations of the catalog.
SELECT pg_advisory_xact_lock(7307648119525449474);
CREATE TABLE IF NOT EXISTS electoral_log_events (
    election_event_id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    database_name TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Set before votes are accepted, and cleared once the event has no ballot waiting
    -- for the sequencer or for review: the events background tasks visit.
    ballots_accepted_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS electoral_log_events_ballot_activity
    ON electoral_log_events (ballots_accepted_at) WHERE ballots_accepted_at IS NOT NULL;
-- The tenants' log databases: what happens outside election events.
CREATE TABLE IF NOT EXISTS electoral_log_tenants (
    tenant_id UUID PRIMARY KEY,
    database_name TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- Databases of deleted election events, renamed and closed to connections. They
-- are kept until they are erased under the retention policy.
CREATE TABLE IF NOT EXISTS electoral_log_retired_events (
    database_name TEXT PRIMARY KEY,
    election_event_id UUID NOT NULL,
    tenant_id UUID NOT NULL,
    retired_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS electoral_log_retired_events_tenant
    ON electoral_log_retired_events (tenant_id, retired_at);
COMMIT;
