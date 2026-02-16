-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Add compound index for efficient queries by latest resolution
CREATE INDEX IF NOT EXISTS idx_tally_session_resolution_latest
    ON sequent_backend.tally_session_resolution(
        tenant_id,
        election_event_id,
        tally_session_id,
        contest_id,
        resolution_type,
        created_at DESC
    );
