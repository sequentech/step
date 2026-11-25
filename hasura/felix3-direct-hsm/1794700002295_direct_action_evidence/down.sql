-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only
-- Maintenance only: drain every writer before running 2302 down and this
-- file in one transaction. The event and policy barriers are defense in depth;
-- ordinary stock DDL/writers do not all take the signing advisory locks.
DO $$
DECLARE anchor record;
BEGIN
 IF current_setting('transaction_isolation')<>'read committed' OR NOT
    pg_try_advisory_xact_lock(hashtextextended('operational-signing-policy:deployment',0))
 THEN RAISE EXCEPTION 'direct action rollback requires a drained read-committed deployment' USING ERRCODE='55P03'; END IF;
 IF to_regclass('felix3_initialize_direct.action_request_scope') IS NOT NULL
    OR to_regclass('felix3_initialize_direct.action_request_birth') IS NOT NULL
    OR to_regclass('felix3_initialize_direct.action_generation') IS NOT NULL
 THEN RAISE EXCEPTION 'remove the inactive multi-event migration first'; END IF;

 SELECT i.tenant,i.event,i.post,i.publication,i.installation,a.enabled_at INTO anchor
 FROM felix3_initialize_direct.action_installation a
 JOIN felix3_initialize_direct.installation i USING(installation);
 IF FOUND AND NOT
    pg_try_advisory_xact_lock(hashtextextended('signing-event:'||anchor.tenant::text||':'||anchor.event::text,0))
 THEN RAISE EXCEPTION 'direct action rollback requires the drained anchor event' USING ERRCODE='55P03'; END IF;

 -- Refuse instead of waiting on a concurrent stock writer. Hold these to the
 -- end of the transaction so even a writer outside the signing lock cannot
 -- race the checked history. The proof column is dropped later by this DDL.
 LOCK TABLE sequent_backend.signing_request,sequent_backend.ballot_publication,
            sequent_backend.ballot_style IN SHARE MODE NOWAIT;
 LOCK TABLE sequent_backend.signing_approval IN ACCESS EXCLUSIVE MODE NOWAIT;

 IF EXISTS(SELECT FROM sequent_backend.signing_approval WHERE action_direct_proof IS NOT NULL)
    OR EXISTS(SELECT FROM felix3_initialize_direct.action_reservation)
 THEN RAISE EXCEPTION 'direct action reservations or evidence remain; do not downgrade'; END IF;

 IF anchor.installation IS NOT NULL AND (
    NOT felix3_initialize_direct.scope_enabled(
       anchor.installation,anchor.tenant,anchor.event,anchor.post,anchor.publication)
    OR EXISTS (SELECT FROM sequent_backend.signing_request r
        WHERE (r.tenant_id,r.election_event_id)=(anchor.tenant,anchor.event)
          -- Stock requests store a whole-second creation time. Ambiguity at
          -- the boundary is activity, not permission to remove the marker.
          AND (NOT isfinite(r.created_at) OR r.created_at>=date_trunc('second',anchor.enabled_at)))
    OR EXISTS (SELECT FROM sequent_backend.ballot_publication p
        WHERE (p.tenant_id,p.election_event_id)=(anchor.tenant,anchor.event)
          AND (NOT isfinite(p.created_at) OR p.created_at>=anchor.enabled_at
               OR (p.published_at IS NOT NULL AND (NOT isfinite(p.published_at) OR p.published_at>=anchor.enabled_at))
               OR (p.deleted_at IS NOT NULL AND (NOT isfinite(p.deleted_at) OR p.deleted_at>=anchor.enabled_at))
               -- Generation has no separate timestamp. An earlier draft may
               -- have been generated since activation; do not assume it old.
               OR (p.id<>anchor.publication AND p.published_at IS NULL)))
    OR EXISTS (SELECT FROM sequent_backend.ballot_style b
        WHERE (b.tenant_id,b.election_event_id)=(anchor.tenant,anchor.event)
          AND (b.created_at IS NULL OR NOT isfinite(b.created_at) OR b.created_at>=anchor.enabled_at
               OR b.last_updated_at IS NULL OR NOT isfinite(b.last_updated_at) OR b.last_updated_at>=anchor.enabled_at
               OR (b.deleted_at IS NOT NULL AND (NOT isfinite(b.deleted_at) OR b.deleted_at>=anchor.enabled_at)))))
 THEN RAISE EXCEPTION 'the anchor event may have used direct actions; preserve the marker'; END IF;
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
