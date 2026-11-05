-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- The figures still showing, which every snapshot pass reads and closes.
CREATE INDEX IF NOT EXISTS monitoring_snapshot_figure_showing
    ON sequent_backend.monitoring_snapshot_figure
        (tenant_id, election_event_id, source, election_set_key)
    WHERE to_revision IS NULL;

-- The closed figures, which pruning checks against the runs it keeps.
CREATE INDEX IF NOT EXISTS monitoring_snapshot_figure_closed
    ON sequent_backend.monitoring_snapshot_figure
        (tenant_id, election_event_id, to_revision)
    WHERE to_revision IS NOT NULL;
