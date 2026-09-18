-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- On populated deployments, drop concurrently with the standalone script first.
DROP INDEX IF EXISTS sequent_backend.ballot_style_voter_reference_idx;
