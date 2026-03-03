-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Drop new indexes
DROP INDEX IF EXISTS sequent_backend.idx_tally_session_resolution_contest;
DROP INDEX IF EXISTS sequent_backend.idx_tally_session_resolution_latest;

-- Drop new FK constraint to contest
ALTER TABLE sequent_backend.tally_session_resolution
DROP CONSTRAINT IF EXISTS fk_tally_session_resolution_contest;

-- Drop new contest_id column
ALTER TABLE sequent_backend.tally_session_resolution
DROP COLUMN IF EXISTS contest_id;

-- Drop renamed FK constraint to results_contest
ALTER TABLE sequent_backend.tally_session_resolution
DROP CONSTRAINT IF EXISTS fk_tally_session_resolution_results_contest;

-- Drop index on results_contest_id
DROP INDEX IF EXISTS sequent_backend.idx_tally_session_resolution_results_contest;

-- Rename results_contest_id back to contest_id
ALTER TABLE sequent_backend.tally_session_resolution
RENAME COLUMN results_contest_id TO contest_id;

-- Restore original FK constraint
ALTER TABLE sequent_backend.tally_session_resolution
ADD CONSTRAINT fk_tally_session_resolution_contest
    FOREIGN KEY (tenant_id, contest_id, election_event_id, results_event_id)
    REFERENCES sequent_backend.results_contest(tenant_id, id, election_event_id, results_event_id)
    ON DELETE CASCADE;

-- Restore original index
CREATE INDEX IF NOT EXISTS idx_tally_session_resolution_contest
    ON sequent_backend.tally_session_resolution(tenant_id, election_event_id, tally_session_id, contest_id);

-- Restore original latest-resolution index
CREATE INDEX IF NOT EXISTS idx_tally_session_resolution_latest
    ON sequent_backend.tally_session_resolution(
        tenant_id,
        election_event_id,
        tally_session_id,
        contest_id,
        resolution_type,
        created_at DESC
    );
