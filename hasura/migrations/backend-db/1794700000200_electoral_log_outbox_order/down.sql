-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP INDEX IF EXISTS sequent_backend.electoral_log_outbox_unrelayed;
DROP INDEX IF EXISTS sequent_backend.electoral_log_outbox_scope_waiting;
CREATE INDEX electoral_log_outbox_unrelayed
    ON sequent_backend.electoral_log_outbox (seq)
    WHERE relayed_at IS NULL;
ALTER TABLE sequent_backend.electoral_log_outbox
    DROP COLUMN announcement,
    DROP COLUMN cancel_reason,
    DROP COLUMN cancelled_by,
    DROP COLUMN cancelled_at;
