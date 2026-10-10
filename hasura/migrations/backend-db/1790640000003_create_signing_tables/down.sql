-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP TABLE IF EXISTS sequent_backend.signing_log_outbox;
DROP TABLE IF EXISTS sequent_backend.staff_crl;
DROP TABLE IF EXISTS sequent_backend.signing_document_revision;
DROP TABLE IF EXISTS sequent_backend.signing_approval;
DROP TABLE IF EXISTS sequent_backend.staff_certificate;
DROP TABLE IF EXISTS sequent_backend.signing_request;
DROP TABLE IF EXISTS sequent_backend.signing_checks;
DROP TABLE IF EXISTS sequent_backend.signing_rule;
-- Staff issuers go: without their purpose they would sign voters in.
DELETE FROM sequent_backend.certificate_authority WHERE purpose = 'staff-signatures';
ALTER TABLE sequent_backend.certificate_authority
    DROP CONSTRAINT IF EXISTS certificate_authority_in_its_event,
    DROP CONSTRAINT IF EXISTS certificate_authority_one_per_purpose,
    ADD CONSTRAINT certificate_authority_tenant_id_election_event_id_fingerpri_key
        UNIQUE (tenant_id, election_event_id, fingerprint_sha256),
    DROP CONSTRAINT IF EXISTS certificate_authority_purpose_known,
    DROP COLUMN IF EXISTS purpose;
