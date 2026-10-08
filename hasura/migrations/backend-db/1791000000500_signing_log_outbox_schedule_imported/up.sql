-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- VOTE-LIFECYCLE: a schedule CSV import stages its electoral log entry in the
-- signing outbox, in the import's transaction. The list keeps every kind the
-- earlier VOTE-LIFECYCLE migrations added (it runs after them).
ALTER TABLE sequent_backend.signing_log_outbox
    DROP CONSTRAINT signing_log_outbox_statement_kind_known;
ALTER TABLE sequent_backend.signing_log_outbox
    ADD CONSTRAINT signing_log_outbox_statement_kind_known CHECK (statement_kind IN (
        'SigningRequestCreated', 'SigningCertificateOpenFailed', 'SigningRequestSigned',
        'SigningSignatureRefused', 'SigningCertificateRegistered', 'SigningHandover',
        'SigningRequestCancelled', 'SigningRequestExpired', 'SigningRequestCompleted',
        'SigningActionExecuted', 'SigningRuleChanged', 'SigningPermissionChanged',
        'SigningIssuerChanged', 'SigningChecksChanged', 'SigningCertificateRevoked',
        'SigningRequestsExported', 'LifecycleWindowChanged', 'ScheduleRecomputeApplied',
        'ScheduledOutcomeChanged', 'ScheduleImported'
    ));
