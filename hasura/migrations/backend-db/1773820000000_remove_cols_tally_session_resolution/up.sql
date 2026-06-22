-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

ALTER TABLE sequent_backend.tally_session_resolution
    DROP CONSTRAINT fk_tally_session_resolution_results_contest;

DROP INDEX IF EXISTS idx_tally_session_resolution_results_contest;

ALTER TABLE sequent_backend.tally_session_resolution
    DROP COLUMN results_contest_id,
    DROP COLUMN results_event_id,
    DROP COLUMN resolution;
