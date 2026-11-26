-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
--
-- Optional synthetic Initialize format. No staff-certificate row or CRL state
-- is invented. The dual-format reader and scoped, fail-closed worker must be
-- installed before activating any direct demo scope.
SELECT pg_advisory_xact_lock(hashtextextended('operational-signing-policy:deployment', 0));

ALTER TABLE sequent_backend.signing_approval
    ADD COLUMN initialize_direct_proof jsonb,
    ALTER COLUMN certificate_id DROP NOT NULL,
    ALTER COLUMN certificate_pem DROP NOT NULL,
    ALTER COLUMN chain_pem DROP NOT NULL,
    ALTER COLUMN fingerprint_sha256 DROP NOT NULL,
    ALTER COLUMN holder_sha256 DROP NOT NULL,
    ALTER COLUMN revocation_status DROP NOT NULL,
    ADD CONSTRAINT signing_approval_proof_format CHECK ((
        CASE WHEN initialize_direct_proof IS NULL THEN
            certificate_id IS NOT NULL AND certificate_pem IS NOT NULL AND chain_pem IS NOT NULL
            AND fingerprint_sha256 IS NOT NULL AND holder_sha256 IS NOT NULL AND revocation_status IS NOT NULL
        ELSE
            certificate_id IS NULL AND certificate_pem IS NULL AND chain_pem IS NULL
            AND fingerprint_sha256 IS NULL AND holder_sha256 IS NULL AND revocation_status IS NULL
            AND document_signature IS NULL AND pdf_cms IS NULL
            AND algorithm = 'ecdsa-p256-sha256' AND octet_length(payload_signature) BETWEEN 8 AND 72
            AND jsonb_typeof(initialize_direct_proof) = 'object'
            AND initialize_direct_proof - ARRAY['installation','post','publication','nonce','payload_sha256','profile'] = '{}'::jsonb
            AND initialize_direct_proof->>'profile' = 'softhsm-test'
            AND initialize_direct_proof->>'payload_sha256' ~ '^[0-9a-f]{64}$'
            AND initialize_direct_proof->>'installation' ~ '^[0-9a-f]{8}-([0-9a-f]{4}-){3}[0-9a-f]{12}$'
            AND initialize_direct_proof->>'post' ~ '^[0-9a-f]{8}-([0-9a-f]{4}-){3}[0-9a-f]{12}$'
            AND initialize_direct_proof->>'publication' ~ '^[0-9a-f]{8}-([0-9a-f]{4}-){3}[0-9a-f]{12}$'
            AND initialize_direct_proof->>'nonce' ~ '^[0-9a-f]{8}-([0-9a-f]{4}-){3}[0-9a-f]{12}$'
        END
    ) IS TRUE);

CREATE UNIQUE INDEX signing_approval_initialize_direct_nonce
    ON sequent_backend.signing_approval ((initialize_direct_proof->>'nonce'))
    WHERE initialize_direct_proof IS NOT NULL;
