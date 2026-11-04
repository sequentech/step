-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
CREATE TABLE sequent_backend.tsf_integrity_check (
    id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    requested_by text NOT NULL,
    started_at timestamptz NOT NULL DEFAULT now(),
    finished_at timestamptz,
    outcome text CHECK (outcome IN ('verified','different','unverifiable')),
    result jsonb NOT NULL DEFAULT '{}'::jsonb,
    -- The evidence commits first; a busy safety writer leaves the maintenance entry to the sweep.
    safety_pending boolean NOT NULL DEFAULT false,
    FOREIGN KEY (tenant_id,election_event_id) REFERENCES sequent_backend.election_lifecycle(tenant_id,election_event_id) ON DELETE CASCADE,
    CHECK ((finished_at IS NULL)=(outcome IS NULL))
);
CREATE UNIQUE INDEX tsf_integrity_one_pending ON sequent_backend.tsf_integrity_check(tenant_id,election_event_id) WHERE finished_at IS NULL;
CREATE INDEX tsf_integrity_event_history ON sequent_backend.tsf_integrity_check(tenant_id,election_event_id,started_at DESC);
-- Deliberately untracked by Hasura: only the authenticated TSF actions expose these records.
