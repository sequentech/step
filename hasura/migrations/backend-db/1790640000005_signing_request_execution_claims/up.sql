-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- A completed request whose action a task runs: when its task last claimed
-- it (the claim is a lease; a stale one may be claimed again) and how many
-- times a task claimed it.
ALTER TABLE sequent_backend.signing_request
    ADD COLUMN execution_started_at timestamptz,
    ADD COLUMN execution_attempts integer NOT NULL DEFAULT 0,
    ADD CONSTRAINT signing_request_execution_attempts_count CHECK (execution_attempts >= 0);

-- What the sweeper looks at.
CREATE INDEX signing_request_unexecuted
    ON sequent_backend.signing_request (completed_at)
    WHERE status = 'completed' AND executed_at IS NULL;
