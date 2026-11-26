-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
SELECT pg_advisory_xact_lock(hashtextextended('operational-signing-policy:deployment', 0));

-- Never discard real approvals to make an unsafe downgrade appear clean.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM sequent_backend.signing_approval
               WHERE initialize_direct_proof IS NOT NULL) THEN
        RAISE EXCEPTION 'Direct Initialize evidence remains; preserve it and do not downgrade';
    END IF;
END $$;

DROP INDEX sequent_backend.signing_approval_initialize_direct_nonce;
ALTER TABLE sequent_backend.signing_approval
    DROP CONSTRAINT signing_approval_proof_format,
    ALTER COLUMN certificate_id SET NOT NULL,
    ALTER COLUMN certificate_pem SET NOT NULL,
    ALTER COLUMN chain_pem SET NOT NULL,
    ALTER COLUMN fingerprint_sha256 SET NOT NULL,
    ALTER COLUMN holder_sha256 SET NOT NULL,
    ALTER COLUMN revocation_status SET NOT NULL,
    DROP COLUMN initialize_direct_proof;
