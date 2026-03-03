-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Drop indexes that reference contest_id before renaming the column
DROP INDEX IF EXISTS sequent_backend.idx_tally_session_resolution_contest;
DROP INDEX IF EXISTS sequent_backend.idx_tally_session_resolution_latest;

-- Drop FK constraint before renaming the column
ALTER TABLE sequent_backend.tally_session_resolution
DROP CONSTRAINT IF EXISTS fk_tally_session_resolution_contest;

-- Rename contest_id (FK to results_contest.id) to results_contest_id
ALTER TABLE sequent_backend.tally_session_resolution
RENAME COLUMN contest_id TO results_contest_id;

-- Recreate FK constraint with new column name
ALTER TABLE sequent_backend.tally_session_resolution
ADD CONSTRAINT fk_tally_session_resolution_results_contest
    FOREIGN KEY (tenant_id, results_contest_id, election_event_id, results_event_id)
    REFERENCES sequent_backend.results_contest(tenant_id, id, election_event_id, results_event_id)
    ON DELETE CASCADE;

-- Recreate index on results_contest_id
CREATE INDEX IF NOT EXISTS idx_tally_session_resolution_results_contest
    ON sequent_backend.tally_session_resolution(tenant_id, election_event_id, tally_session_id, results_contest_id);

-- Add new contest_id column referencing sequent_backend.contest directly
ALTER TABLE sequent_backend.tally_session_resolution
ADD COLUMN contest_id UUID;

-- Backfill contest_id from resolution_data for existing rows
UPDATE sequent_backend.tally_session_resolution
SET contest_id = (resolution_data->>'contest_id')::uuid
WHERE contest_id IS NULL
  AND resolution_data->>'contest_id' IS NOT NULL;

-- Add FK constraint for the stable contest reference
ALTER TABLE sequent_backend.tally_session_resolution
ADD CONSTRAINT fk_tally_session_resolution_contest
    FOREIGN KEY (contest_id, tenant_id, election_event_id)
    REFERENCES sequent_backend.contest(id, tenant_id, election_event_id);

-- Recreate indexes for efficient filtering by contest
CREATE INDEX IF NOT EXISTS idx_tally_session_resolution_contest
    ON sequent_backend.tally_session_resolution(tenant_id, election_event_id, tally_session_id, contest_id);

CREATE INDEX IF NOT EXISTS idx_tally_session_resolution_latest
    ON sequent_backend.tally_session_resolution(
        tenant_id,
        election_event_id,
        tally_session_id,
        contest_id,
        resolution_type,
        created_at DESC
    );
