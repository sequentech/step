-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- What the figures of a complete run were last found to be counted from:
-- digests of the projection, the elections, areas, tally sessions, sign-in
-- counters and sets of elections a pass reads, with the settings, the day
-- and when they were counted (windmill's snapshot::CountedInputs). A pass
-- that finds the same inputs as the shown run's does not count again; it
-- only marks the run checked. A pass that counts again and finds the same
-- figures replaces them, so it is not final as the rest of the run is.
-- NULL, as for runs completed before this column: unknown, so the next pass
-- counts.
ALTER TABLE sequent_backend.monitoring_snapshot_run
    ADD COLUMN IF NOT EXISTS counted_inputs jsonb;

ALTER TABLE sequent_backend.monitoring_snapshot_run
    ADD CONSTRAINT monitoring_snapshot_run_inputs_when_complete
        CHECK (
            counted_inputs IS NULL
            OR (status = 'COMPLETE' AND jsonb_typeof(counted_inputs) = 'object')
        );
