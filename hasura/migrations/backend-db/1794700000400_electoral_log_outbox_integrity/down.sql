-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

ALTER TABLE sequent_backend.electoral_log_outbox
    DROP COLUMN dead_letter_reason,
    DROP COLUMN dead_lettered_at;
DROP INDEX IF EXISTS sequent_backend.electoral_log_outbox_admin_signature;
DROP TRIGGER IF EXISTS electoral_log_outbox_database_time ON sequent_backend.electoral_log_outbox;
DROP FUNCTION IF EXISTS sequent_backend.electoral_log_outbox_database_time();
