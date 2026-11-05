-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only

-- Back to the lists before VOTE-LIFECYCLE's scheduled outcomes. Rows that
-- use the new values must be gone first.

ALTER TABLE sequent_backend.signing_request
    DROP CONSTRAINT signing_request_cancel_reason_known;
ALTER TABLE sequent_backend.signing_request
    ADD CONSTRAINT signing_request_cancel_reason_known CHECK (cancel_reason IN (
        'by-requester', 'by-operator', 'rule-changed', 'payload-changed', 'superseded',
        'certificate-revoked'
    ));

ALTER TABLE sequent_backend.signing_log_outbox
    DROP CONSTRAINT signing_log_outbox_statement_kind_known;
ALTER TABLE sequent_backend.signing_log_outbox
    ADD CONSTRAINT signing_log_outbox_statement_kind_known CHECK (statement_kind IN (
        'SigningRequestCreated', 'SigningCertificateOpenFailed', 'SigningRequestSigned',
        'SigningSignatureRefused', 'SigningCertificateRegistered', 'SigningHandover',
        'SigningRequestCancelled', 'SigningRequestExpired', 'SigningRequestCompleted',
        'SigningActionExecuted', 'SigningRuleChanged', 'SigningPermissionChanged',
        'SigningIssuerChanged', 'SigningChecksChanged', 'SigningCertificateRevoked',
        'SigningRequestsExported', 'LifecycleWindowChanged', 'ScheduleRecomputeApplied'
    ));
