-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DELETE FROM sequent_backend.signing_log_outbox WHERE statement_kind = 'LockdownChanged';
DO $$
DECLARE
    definition text;
BEGIN
    SELECT pg_get_constraintdef(oid) INTO definition
    FROM pg_constraint
    WHERE conname = 'signing_log_outbox_statement_kind_known'
      AND conrelid = 'sequent_backend.signing_log_outbox'::regclass;
    IF position('''LockdownChanged''' IN definition) > 0 THEN
        ALTER TABLE sequent_backend.signing_log_outbox
            DROP CONSTRAINT signing_log_outbox_statement_kind_known;
        EXECUTE 'ALTER TABLE sequent_backend.signing_log_outbox '
            || 'ADD CONSTRAINT signing_log_outbox_statement_kind_known '
            || replace(definition, ', ''LockdownChanged''::text', '');
    END IF;
END $$;
