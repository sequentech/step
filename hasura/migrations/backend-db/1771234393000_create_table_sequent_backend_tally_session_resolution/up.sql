-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Create tally_session_resolution table for generic resolution handling
CREATE TABLE IF NOT EXISTS sequent_backend.tally_session_resolution (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    election_event_id UUID NOT NULL,
    tally_session_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolution_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    resolution_data JSONB NOT NULL DEFAULT '{}',
    resolution JSONB,
    resolved_by_user UUID,
    resolved_at TIMESTAMPTZ,
    labels JSONB,
    annotations JSONB,

    -- Foreign key constraints
    FOREIGN KEY (tenant_id, election_event_id, tally_session_id)
        REFERENCES sequent_backend.tally_session(tenant_id, election_event_id, id)
        ON DELETE CASCADE,

    -- Check constraints
    CONSTRAINT valid_status CHECK (status IN ('pending', 'resolved', 'cancelled')),
    CONSTRAINT valid_resolution_type CHECK (resolution_type IN ('irv_tie_break', 'manual_recount', 'external_validation'))
);

-- Create indexes for common queries
CREATE INDEX IF NOT EXISTS idx_tally_session_resolution_tally_session
    ON sequent_backend.tally_session_resolution(tenant_id, election_event_id, tally_session_id);

CREATE INDEX IF NOT EXISTS idx_tally_session_resolution_status
    ON sequent_backend.tally_session_resolution(status);

CREATE INDEX IF NOT EXISTS idx_tally_session_resolution_type
    ON sequent_backend.tally_session_resolution(resolution_type);
