-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Remove foreign key constraint
ALTER TABLE sequent_backend.tally_session_resolution
DROP CONSTRAINT IF EXISTS fk_tally_session_resolution_contest;

-- Remove index
DROP INDEX IF EXISTS sequent_backend.idx_tally_session_resolution_contest;

-- Remove results_event_id column
ALTER TABLE sequent_backend.tally_session_resolution
DROP COLUMN IF EXISTS results_event_id;

-- Remove contest_id column
ALTER TABLE sequent_backend.tally_session_resolution
DROP COLUMN IF EXISTS contest_id;
