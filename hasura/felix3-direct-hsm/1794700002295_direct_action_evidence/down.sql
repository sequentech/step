-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only
SELECT pg_advisory_xact_lock(hashtextextended('operational-signing-policy:deployment',0));
DO $$
BEGIN
 IF EXISTS(SELECT FROM sequent_backend.signing_approval WHERE action_direct_proof IS NOT NULL)
    OR EXISTS(SELECT FROM felix3_initialize_direct.action_installation)
    OR EXISTS(SELECT FROM felix3_initialize_direct.action_reservation)
 THEN RAISE EXCEPTION 'direct action installation or evidence remains; preserve it and do not downgrade'; END IF;
END;
$$;
DROP TRIGGER direct_action_approval_evidence ON sequent_backend.signing_approval;
DROP FUNCTION felix3_initialize_direct.action_evidence_guard();
DROP INDEX sequent_backend.signing_approval_action_direct_nonce;
ALTER TABLE sequent_backend.signing_approval
 DROP CONSTRAINT signing_approval_proof_format,
 DROP COLUMN action_direct_proof,
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
DROP FUNCTION felix3_initialize_direct.consume_action(uuid,uuid,uuid,uuid,uuid,uuid,uuid,uuid,text,bytea);
DROP FUNCTION felix3_initialize_direct.proof_action_key(uuid,uuid,uuid,uuid,uuid,uuid,uuid,uuid,text,bytea);
DROP FUNCTION felix3_initialize_direct.record_action_key(uuid,bytea);
DROP FUNCTION felix3_initialize_direct.reserve_action(uuid,uuid,uuid,uuid,uuid,uuid,uuid,text,bytea);
DROP FUNCTION felix3_initialize_direct.waiting_action(uuid,uuid,uuid,uuid,uuid,uuid,text,bytea);
DROP TRIGGER direct_action_install ON felix3_initialize_direct.action_installation;
DROP TRIGGER direct_action_immutable ON felix3_initialize_direct.action_installation;
DROP FUNCTION felix3_initialize_direct.guard_action_installation();
DROP FUNCTION felix3_initialize_direct.action_scope_enabled(uuid,uuid,uuid,uuid,uuid);
DROP FUNCTION felix3_initialize_direct.action_scope(uuid,uuid);
DROP TABLE felix3_initialize_direct.action_reservation;
DROP TABLE felix3_initialize_direct.action_installation;
DROP FUNCTION felix3_initialize_direct.canonical_action_ecdsa(bytea);
DROP FUNCTION felix3_initialize_direct.known_action(text);
