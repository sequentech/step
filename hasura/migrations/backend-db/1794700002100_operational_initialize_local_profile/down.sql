-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Run as one transaction. Serialize with prepare and operator provisioning.
-- The operator explicitly retires a reconciled installation before rollback.
SELECT pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended('operational-signing-policy:deployment', 0));
-- A reader may already have called the lookup before asking for the shared
-- policy lock in prepare. Retry rollback after that transaction finishes.
LOCK TABLE sequent_backend.operational_initialize_local_profile IN ACCESS EXCLUSIVE MODE NOWAIT;
DO $$
BEGIN
    IF current_setting('transaction_isolation') <> 'read committed' THEN
        RAISE EXCEPTION 'roll back the local Initialize profile at read committed';
    END IF;
    IF EXISTS (SELECT FROM sequent_backend.operational_initialize_local_profile) THEN
        RAISE EXCEPTION 'retire the installed local Initialize profile before rollback';
    END IF;
    IF EXISTS (
        SELECT FROM sequent_backend.operational_signing_intent AS i
        WHERE i.issuer COLLATE "C" = 'http://localhost:18090/realms/tenant-' || i.tenant_id::text
          AND (i.binding).action = 'initialize-voting' AND i.purpose = 'approval-payload'
          AND i.phase <> 'applied'
    ) THEN
        RAISE EXCEPTION 'reconcile pending local Initialize evidence before rollback';
    END IF;
END;
$$;

-- Original 1794700001100 definition. Replacing in place retains its ACL.
CREATE OR REPLACE FUNCTION sequent_backend.operational_initialize_prepare(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_payload_sha256 text,
    p_profile text, p_attempt uuid, p_issuer text, p_nonce_sha256 text, p_binding_sha256 text
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
DECLARE
    c record;
    b sequent_backend.operational_signing_binding;
    stale sequent_backend.operational_signing_binding;
BEGIN
    IF p_tenant IS NULL OR p_event IS NULL OR p_request IS NULL OR p_actor IS NULL
       OR p_attempt IS NULL OR p_attempt = '00000000-0000-0000-0000-000000000000'
       OR p_issuer IS NULL OR p_issuer !~ '^https://[^[:space:][:cntrl:]]+$'
       OR octet_length(p_issuer) > 1024
       OR p_nonce_sha256 IS NULL OR p_nonce_sha256 !~ '^[0-9a-f]{64}$'
       OR p_binding_sha256 IS NULL OR p_binding_sha256 !~ '^[0-9a-f]{64}$' THEN
        RETURN false;
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'signing-event:' || p_tenant::text || ':' || p_event::text, 0));
    SELECT * INTO c FROM sequent_backend.operational_initialize_context(
        p_tenant, p_event, p_request, p_actor, p_payload_sha256, p_profile);
    IF NOT FOUND
       OR EXISTS (SELECT FROM sequent_backend.operational_initialize_attempt AS t
                  WHERE t.challenge_id = p_attempt) THEN
        RETURN false;
    END IF;

    SELECT (i.binding).* INTO stale
    FROM sequent_backend.operational_signing_intent AS i
    JOIN sequent_backend.operational_initialize_attempt AS t ON t.challenge_id = i.challenge_id
    WHERE (i.tenant_id, i.event_id, i.request_id, i.actor, i.purpose)
        = (p_tenant, p_event, p_request, p_actor, 'approval-payload')
      AND i.phase = 'prepared' AND t.step IN ('issued', 'redeemed')
      AND (i.binding).intent_expires_at <= clock_timestamp();
    IF FOUND THEN
        UPDATE sequent_backend.operational_initialize_attempt AS t
        SET step = 'discarded', discarded_at = clock_timestamp()
        WHERE t.challenge_id = stale.challenge_id AND t.step IN ('issued', 'redeemed');
        IF NOT FOUND OR NOT sequent_backend.operational_signing_discard_prepared(stale) THEN
            RAISE EXCEPTION 'cannot supersede the expired Initialize challenge';
        END IF;
    END IF;

    b.tenant_id := p_tenant;
    b.event_id := p_event;
    b.request_id := p_request;
    b.actor := p_actor;
    b.action := 'initialize-voting';
    b.purpose := 'approval-payload';
    b.key_id := c.key_id;
    b.key_version := c.key_version;
    b.cert_fingerprint := c.cert_fingerprint;
    b.cert_spki := c.cert_spki;
    b.payload_hash := p_payload_sha256;
    b.message_hash := p_payload_sha256;
    b.artifact_hash := p_payload_sha256;
    b.binding_sha256 := p_binding_sha256;
    b.issuer := p_issuer;
    b.nonce_hash := p_nonce_sha256;
    b.challenge_id := p_attempt;
    b.challenge_issued_at := c.issued_at;
    b.request_expires_at := c.request_expiry;
    b.intent_expires_at := c.intent_expiry;
    IF NOT sequent_backend.operational_signing_prepare(b) THEN
        RETURN false;
    END IF;
    INSERT INTO sequent_backend.operational_initialize_attempt
        (challenge_id, tenant_id, event_id, request_id, actor, payload_sha256, binding_sha256,
         device_profile, issued_at, expires_at)
    VALUES (p_attempt, p_tenant, p_event, p_request, p_actor, p_payload_sha256, p_binding_sha256,
            p_profile, c.issued_at, c.intent_expiry);
    RETURN true;
END;
$$;

DROP TRIGGER local_initialize_profile_check ON sequent_backend.operational_initialize_local_profile;
DROP TRIGGER local_initialize_profile_immutable ON sequent_backend.operational_initialize_local_profile;
DROP FUNCTION sequent_backend.operational_initialize_local_profile_guard();
DROP FUNCTION sequent_backend.operational_signing_local_initialize_installation_id(uuid,uuid,uuid,uuid,text);
DROP VIEW sequent_backend.operational_initialize_local_authority;
DROP TABLE sequent_backend.operational_initialize_local_profile;
-- The marker is a cluster role, like the groups created by 1000. It may
-- serve other databases. Its only grant in this migration was on the helper.
