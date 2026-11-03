-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- VOTE-TRANSMISSION: each download of an ER transmission package's files
-- goes to the election event's log through the audit log outbox.
-- An upgraded database can already contain readiness audit records from a
-- release that did not include this migration. Preserve those accepted kinds.
ALTER TABLE sequent_backend.audit_log_outbox
    DROP CONSTRAINT audit_log_outbox_statement_kind_known;

ALTER TABLE sequent_backend.audit_log_outbox
    ADD CONSTRAINT audit_log_outbox_statement_kind_known CHECK (statement_kind IN (
        'AdminChange', 'KeycloakAdminEvent', 'LogsExported', 'LogWriterStarted',
        'LogWriterStopped', 'TransmissionFileDownloaded', 'VotersImported', 'ReadinessVerified'
    ));
