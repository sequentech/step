-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Upgrade readers before enabling these signed audit statement kinds.
SET LOCAL lock_timeout = '10s';

ALTER TABLE sequent_backend.audit_log_outbox
    DROP CONSTRAINT audit_log_outbox_statement_kind_known,
    ADD CONSTRAINT audit_log_outbox_statement_kind_known CHECK (statement_kind IN (
        'AdminChange', 'KeycloakAdminEvent', 'LogsExported', 'LogWriterStarted',
        'LogWriterStopped', 'TransmissionFileDownloaded', 'VotersImported', 'ReadinessVerified'
    ));
