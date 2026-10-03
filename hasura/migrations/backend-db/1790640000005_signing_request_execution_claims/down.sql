-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP INDEX IF EXISTS sequent_backend.signing_request_unexecuted;
ALTER TABLE sequent_backend.signing_request
    DROP CONSTRAINT IF EXISTS signing_request_execution_attempts_count,
    DROP COLUMN IF EXISTS execution_attempts,
    DROP COLUMN IF EXISTS execution_started_at;
