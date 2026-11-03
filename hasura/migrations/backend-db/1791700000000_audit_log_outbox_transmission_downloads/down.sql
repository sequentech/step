-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DELETE FROM sequent_backend.audit_log_outbox
    WHERE statement_kind = 'TransmissionFileDownloaded';

ALTER TABLE sequent_backend.audit_log_outbox
    DROP CONSTRAINT audit_log_outbox_statement_kind_known;

ALTER TABLE sequent_backend.audit_log_outbox
    ADD CONSTRAINT audit_log_outbox_statement_kind_known CHECK (statement_kind IN (
        'AdminChange', 'KeycloakAdminEvent', 'LogsExported', 'LogWriterStarted',
        'LogWriterStopped'
    ));
