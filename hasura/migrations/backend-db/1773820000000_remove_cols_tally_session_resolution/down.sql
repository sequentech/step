-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

ALTER TABLE sequent_backend.tally_session_resolution
    ADD COLUMN results_contest_id UUID,
    ADD COLUMN results_event_id UUID,
    ADD COLUMN resolution JSONB;

CREATE INDEX IF NOT EXISTS idx_tally_session_resolution_results_contest
    ON sequent_backend.tally_session_resolution(tenant_id, election_event_id, tally_session_id, results_contest_id);

ALTER TABLE sequent_backend.tally_session_resolution
    ADD CONSTRAINT fk_tally_session_resolution_results_contest
        FOREIGN KEY (tenant_id, results_contest_id, election_event_id, results_event_id)
        REFERENCES sequent_backend.results_contest(tenant_id, id, election_event_id, results_event_id)
        ON DELETE CASCADE;
