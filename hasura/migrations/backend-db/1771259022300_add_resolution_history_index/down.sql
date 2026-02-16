-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Remove resolution history index
DROP INDEX IF EXISTS sequent_backend.idx_tally_session_resolution_latest;
