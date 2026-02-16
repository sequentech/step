-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Add contest_id column to tally_session_resolution table
ALTER TABLE sequent_backend.tally_session_resolution
ADD COLUMN contest_id UUID;

-- Add results_event_id column to tally_session_resolution table
ALTER TABLE sequent_backend.tally_session_resolution
ADD COLUMN results_event_id UUID;

-- Create index for queries by contest
CREATE INDEX IF NOT EXISTS idx_tally_session_resolution_contest
    ON sequent_backend.tally_session_resolution(tenant_id, election_event_id, tally_session_id, contest_id);

-- Add foreign key constraint to results_contest
ALTER TABLE sequent_backend.tally_session_resolution
ADD CONSTRAINT fk_tally_session_resolution_contest
    FOREIGN KEY (tenant_id, contest_id, election_event_id, results_event_id)
    REFERENCES sequent_backend.results_contest(tenant_id, id, election_event_id, results_event_id)
    ON DELETE CASCADE;
