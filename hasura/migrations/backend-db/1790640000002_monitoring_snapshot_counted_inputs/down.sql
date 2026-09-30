-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

ALTER TABLE sequent_backend.monitoring_snapshot_run
    DROP CONSTRAINT IF EXISTS monitoring_snapshot_run_inputs_when_complete;
ALTER TABLE sequent_backend.monitoring_snapshot_run
    DROP COLUMN IF EXISTS counted_inputs;
