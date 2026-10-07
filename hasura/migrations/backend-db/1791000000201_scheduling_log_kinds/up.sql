-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- VOTE-LIFECYCLE: scheduling entries (a lifecycle window opened or closed,
-- a tz database recompute applied) go through the signing log outbox.
-- A later migration extending this list keeps these two.
ALTER TABLE sequent_backend.signing_log_outbox
    DROP CONSTRAINT signing_log_outbox_statement_kind_known;

ALTER TABLE sequent_backend.signing_log_outbox
    ADD CONSTRAINT signing_log_outbox_statement_kind_known CHECK (statement_kind IN (
        'SigningRequestCreated', 'SigningCertificateOpenFailed', 'SigningRequestSigned',
        'SigningSignatureRefused', 'SigningCertificateRegistered', 'SigningHandover',
        'SigningRequestCancelled', 'SigningRequestExpired', 'SigningRequestCompleted',
        'SigningActionExecuted', 'SigningRuleChanged', 'SigningPermissionChanged',
        'SigningIssuerChanged', 'SigningChecksChanged', 'SigningCertificateRevoked',
        'SigningRequestsExported',
        'LifecycleWindowChanged', 'ScheduleRecomputeApplied'
    ));
