-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- When the outbox worker last failed to post the entry: it waits before
-- trying again, longer after each failure.
ALTER TABLE sequent_backend.signing_log_outbox ADD COLUMN last_attempt_at timestamptz;
