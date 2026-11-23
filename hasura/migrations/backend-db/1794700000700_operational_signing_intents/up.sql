-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Operational signer ledger. Not tracked in Hasura. Only the trusted backend
-- verifies IdP proofs, permissions and cryptographic material; SQL compares the
-- complete resulting immutable tuple. A device call is outside this transaction.
-- Provision dedicated trusted DB credentials/grants separately; SECURITY
-- INVOKER is not a boundary against a shared DB owner or privileged role.
-- Do not track these tables/actions in Hasura.
CREATE TYPE sequent_backend.operational_signing_binding AS (
    tenant_id uuid,
    event_id uuid,
    request_id uuid,
    actor text,
    action text,
    purpose text,
    key_id text,
    key_version integer,
    cert_fingerprint text,
    cert_spki text,
    payload_hash text,
    message_hash text,
    artifact_hash text,
    binding_sha256 text,
    pdf_revision integer,
    pdf_parent_hash text,
    pdf_digest text,
    issuer text,
    nonce_hash text,
    challenge_id uuid,
    challenge_issued_at timestamptz,
    request_expires_at timestamptz,
    intent_expires_at timestamptz
);

-- Unlinked deliberately: event/request deletion must not reopen an issuer nonce.
-- Retention/GC needs separate policy for all accepted IdP proof lifetimes.
CREATE TABLE sequent_backend.operational_signing_ceremony (
    issuer text NOT NULL,
    nonce_hash text NOT NULL CHECK (nonce_hash ~ '^[a-f0-9]{64}$'),
    PRIMARY KEY (issuer, nonce_hash)
);

CREATE TABLE sequent_backend.operational_signing_intent (
    tenant_id uuid NOT NULL,
    event_id uuid NOT NULL,
    request_id uuid NOT NULL,
    actor text NOT NULL,
    purpose text NOT NULL,
    binding sequent_backend.operational_signing_binding NOT NULL,
    issuer text GENERATED ALWAYS AS ((binding).issuer) STORED NOT NULL,
    nonce_hash text GENERATED ALWAYS AS ((binding).nonce_hash) STORED NOT NULL,
    challenge_id uuid GENERATED ALWAYS AS ((binding).challenge_id) STORED NOT NULL UNIQUE,
    phase text NOT NULL DEFAULT 'prepared',
    reservation_id uuid UNIQUE,
    reserved_at timestamptz,
    invoked_at timestamptz,
    reservation_deadline timestamptz,
    settled_at timestamptz,
    applied_at timestamptz,
    receipt bytea,
    -- Independent evidence of output attached by the verified approval
    -- transaction, not merely the HSM signature for a PDF's CMS attributes.
    applied_signature_sha256 text,
    applied_pdf_revision_id uuid,
    applied_pdf_document_id uuid,
    applied_pdf_sha256 text,
    PRIMARY KEY (tenant_id, event_id, request_id, actor, purpose),
    FOREIGN KEY (tenant_id, event_id, request_id)
        REFERENCES sequent_backend.signing_request (tenant_id, election_event_id, id)
        ON DELETE CASCADE,
    FOREIGN KEY (issuer, nonce_hash)
        REFERENCES sequent_backend.operational_signing_ceremony (issuer, nonce_hash),
    CHECK (phase IN ('prepared', 'reserved', 'invoked', 'completed', 'applied', 'indeterminate')),
    CHECK ((phase IN ('prepared', 'reserved', 'invoked')) = (settled_at IS NULL)),
    CHECK ((phase = 'prepared') = (reservation_id IS NULL)),
    CHECK ((reservation_id IS NULL) = (reserved_at IS NULL)),
    CHECK ((reservation_id IS NULL) = (reservation_deadline IS NULL)),
    CHECK (reserved_at < reservation_deadline),
    CHECK (invoked_at IS NULL OR (invoked_at >= reserved_at AND invoked_at < reservation_deadline)),
    CHECK (phase NOT IN ('invoked', 'completed', 'applied') OR invoked_at IS NOT NULL),
    CHECK (phase NOT IN ('prepared', 'reserved') OR invoked_at IS NULL),
    CHECK ((binding).tenant_id = tenant_id AND (binding).event_id = event_id
        AND (binding).request_id = request_id AND (binding).actor = actor
        AND (binding).purpose = purpose),
    CHECK ((phase IN ('completed', 'applied')) = (receipt IS NOT NULL)),
    CHECK ((phase = 'applied') = (applied_at IS NOT NULL)),
    CHECK ((phase = 'applied') = (applied_signature_sha256 IS NOT NULL)),
    CHECK (applied_signature_sha256 ~ '^[a-f0-9]{64}$'),
    CHECK ((phase = 'applied' AND purpose = 'pdf-cms') = (applied_pdf_revision_id IS NOT NULL)),
    CHECK ((applied_pdf_revision_id IS NULL) = (applied_pdf_document_id IS NULL)),
    CHECK ((applied_pdf_revision_id IS NULL) = (applied_pdf_sha256 IS NULL)),
    CHECK (applied_pdf_sha256 ~ '^[a-f0-9]{64}$'),
    CHECK (applied_at >= settled_at),
    CHECK (octet_length(receipt) BETWEEN 1 AND 16384)
);

-- Hold the PDF lease across device work and receipt storage. Only verified
-- application to the persisted expected signed revision releases it.
CREATE UNIQUE INDEX operational_signing_pdf_lease
    ON sequent_backend.operational_signing_intent (tenant_id, event_id, request_id)
    WHERE purpose = 'pdf-cms' AND phase IN ('reserved', 'invoked', 'completed', 'indeterminate');

CREATE FUNCTION sequent_backend.operational_signing_immutable()
RETURNS trigger LANGUAGE plpgsql SECURITY INVOKER
SET search_path = pg_catalog
AS $$
BEGIN
    IF ROW(NEW.tenant_id, NEW.event_id, NEW.request_id, NEW.actor, NEW.purpose,
           NEW.binding)
       IS DISTINCT FROM
       ROW(OLD.tenant_id, OLD.event_id, OLD.request_id, OLD.actor, OLD.purpose,
           OLD.binding)
       OR NOT (
           (OLD.phase = 'prepared' AND NEW.phase = 'reserved')
           OR (
               ROW(NEW.reservation_id, NEW.reserved_at, NEW.reservation_deadline)
                    IS NOT DISTINCT FROM ROW(OLD.reservation_id, OLD.reserved_at, OLD.reservation_deadline)
               AND (
                   (OLD.phase = 'reserved' AND NEW.phase = 'invoked')
                   OR (
                       NEW.invoked_at IS NOT DISTINCT FROM OLD.invoked_at
                       AND (
                           (OLD.phase = 'invoked' AND NEW.phase IN ('completed', 'indeterminate'))
                           OR (OLD.phase = 'reserved' AND NEW.phase = 'indeterminate')
                           OR (OLD.phase = 'completed' AND NEW.phase = 'applied'
                               AND NEW.receipt IS NOT DISTINCT FROM OLD.receipt
                               AND NEW.settled_at IS NOT DISTINCT FROM OLD.settled_at)
                       )
                   )
               )
           )
       ) THEN
        RAISE EXCEPTION 'invalid operational-signing ledger update'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER operational_signing_immutable BEFORE UPDATE
ON sequent_backend.operational_signing_intent
FOR EACH ROW EXECUTE FUNCTION sequent_backend.operational_signing_immutable();

CREATE FUNCTION sequent_backend.operational_signing_validate(
    p sequent_backend.operational_signing_binding
) RETURNS boolean LANGUAGE plpgsql SECURITY INVOKER
SET search_path = pg_catalog
AS $$
DECLARE
    r sequent_backend.signing_request%ROWTYPE;
    at_time timestamptz;
    v text;
BEGIN
    IF p.tenant_id IS NULL OR p.event_id IS NULL OR p.request_id IS NULL
       OR p.key_version IS NULL OR p.key_version <= 0
       OR p.challenge_id IS NULL OR p.challenge_issued_at IS NULL
       OR NOT isfinite(p.challenge_issued_at) OR p.challenge_issued_at <= 'epoch'::timestamptz
       OR p.intent_expires_at IS NULL OR p.intent_expires_at <= p.challenge_issued_at
       OR p.intent_expires_at > p.challenge_issued_at + interval '5 minutes'
       OR NOT isfinite(p.intent_expires_at)
       OR (p.request_expires_at IS NOT NULL AND NOT isfinite(p.request_expires_at))
       OR p.purpose IS NULL OR p.purpose NOT IN ('approval-payload', 'pdf-cms', 'eml')
       OR p.action IS NULL THEN
        RETURN false;
    END IF;
    FOREACH v IN ARRAY ARRAY[p.actor, p.key_id, p.issuer] LOOP
        IF v IS NULL OR btrim(v) = '' OR octet_length(v) > 1024 OR v ~ '[[:cntrl:]]' THEN
            RETURN false;
        END IF;
    END LOOP;
    FOREACH v IN ARRAY ARRAY[p.cert_fingerprint, p.cert_spki, p.payload_hash,
                              p.message_hash, p.artifact_hash, p.binding_sha256, p.nonce_hash] LOOP
        IF v IS NULL OR v !~ '^[a-f0-9]{64}$' THEN
            RETURN false;
        END IF;
    END LOOP;
    IF p.purpose <> 'pdf-cms' AND
       (p.pdf_revision IS NOT NULL OR p.pdf_parent_hash IS NOT NULL OR p.pdf_digest IS NOT NULL) THEN
        RETURN false;
    END IF;

    -- Same lock and lock ordering as windmill::postgres::signing::lock_signing_event.
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'signing-event:' || p.tenant_id::text || ':' || p.event_id::text, 0));
    SELECT * INTO r FROM sequent_backend.signing_request AS q
    WHERE (q.tenant_id, q.election_event_id, q.id) = (p.tenant_id, p.event_id, p.request_id)
    FOR UPDATE;
    at_time := clock_timestamp();
    IF NOT FOUND OR r.status <> 'waiting' OR r.action <> p.action
       OR r.payload_sha256 <> p.payload_hash
       OR p.challenge_issued_at > at_time OR p.intent_expires_at <= at_time OR r.expires_at IS DISTINCT FROM p.request_expires_at
       OR (r.expires_at IS NOT NULL AND (r.expires_at <= at_time OR p.intent_expires_at > r.expires_at)) THEN
        RETURN false;
    END IF;

    -- Existing authorizer additionally checks role, requester/quorum, Post,
    -- certificate checks and trustee ownership. SQL adds defensive actor/scope checks.
    IF NOT EXISTS (
        SELECT FROM sequent_backend.staff_certificate AS c
        WHERE (c.tenant_id, c.election_event_id, c.user_id) = (p.tenant_id, p.event_id, p.actor)
          AND c.fingerprint_sha256 = p.cert_fingerprint AND c.spki_sha256 = p.cert_spki
          AND c.status = 'active' AND c.not_before <= at_time AND c.not_after > at_time
          AND (c.election_id IS NULL OR c.election_id = r.election_id)
    ) THEN
        RETURN false;
    END IF;

    CASE p.purpose
        WHEN 'approval-payload' THEN
            IF p.message_hash <> p.payload_hash THEN RETURN false; END IF;
        WHEN 'eml' THEN
            IF p.action <> 'transmit-results'
               OR r.document_sha256 IS DISTINCT FROM p.message_hash THEN RETURN false; END IF;
        WHEN 'pdf-cms' THEN
            IF p.action NOT IN ('generate-election-returns', 'generate-reports')
               OR p.pdf_revision IS NULL OR p.pdf_revision <= 0
               OR p.pdf_parent_hash IS NULL OR p.pdf_parent_hash !~ '^[a-f0-9]{64}$'
               OR p.pdf_digest IS NULL OR p.pdf_digest !~ '^[a-f0-9]{64}$'
               OR NOT EXISTS (
                   SELECT FROM sequent_backend.signing_document_revision AS d
                   WHERE (d.tenant_id, d.election_event_id, d.request_id) =
                         (p.tenant_id, p.event_id, p.request_id)
                     AND d.state = 'prepared' AND d.prepared_for_user = p.actor
                     AND d.revision = p.pdf_revision AND d.parent_sha256 = p.pdf_parent_hash
                     AND d.digest_sha256 = p.pdf_digest
               )
               OR p.pdf_parent_hash IS DISTINCT FROM (
                   SELECT d.sha256 FROM sequent_backend.signing_document_revision AS d
                   WHERE (d.tenant_id, d.election_event_id, d.request_id) =
                         (p.tenant_id, p.event_id, p.request_id)
                     AND d.state IN ('base', 'signed')
                   ORDER BY d.revision DESC LIMIT 1
               ) THEN RETURN false; END IF;
    END CASE;

    RETURN true;
END;
$$;

-- Called before redirecting to the IdP, storing only the expected nonce
-- digest, never the code, raw nonce, verifier, or any bearer credential.
CREATE FUNCTION sequent_backend.operational_signing_prepare(
    p sequent_backend.operational_signing_binding
) RETURNS boolean LANGUAGE plpgsql SECURITY INVOKER
SET search_path = pg_catalog
AS $$
BEGIN
    -- Obtain the timestamp and derive the binding in this same transaction.
    -- The input must equal the exact DB-authored timestamp, not browser time.
    IF p.challenge_issued_at IS DISTINCT FROM transaction_timestamp()
       OR NOT sequent_backend.operational_signing_validate(p) THEN RETURN false; END IF;
    -- Unique actor/request/purpose blocks retries with a different binding or nonce.
    -- A repeated (issuer, nonce) across any request cannot reserve a device use.
    IF EXISTS (
        SELECT FROM sequent_backend.operational_signing_intent AS i
        WHERE (i.tenant_id, i.event_id, i.request_id, i.actor, i.purpose)
            = (p.tenant_id, p.event_id, p.request_id, p.actor, p.purpose)
    ) THEN
        RETURN false;
    END IF;
    INSERT INTO sequent_backend.operational_signing_ceremony (issuer, nonce_hash)
    VALUES (p.issuer, p.nonce_hash) ON CONFLICT DO NOTHING;
    IF NOT FOUND THEN RETURN false; END IF;
    INSERT INTO sequent_backend.operational_signing_intent
        (tenant_id, event_id, request_id, actor, purpose, binding)
    VALUES (p.tenant_id, p.event_id, p.request_id, p.actor, p.purpose, p)
    ON CONFLICT DO NOTHING;
    RETURN FOUND;
END;
$$;

-- A verified caller may explicitly discard an exact never-reserved
-- challenge (for cancellation or known-no-effect supersession). The old
-- ceremony is still consumed. Reserve and discard take the same event lock.
CREATE FUNCTION sequent_backend.operational_signing_discard_prepared(
    p sequent_backend.operational_signing_binding
) RETURNS boolean LANGUAGE plpgsql SECURITY INVOKER
SET search_path = pg_catalog
AS $$
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'signing-event:' || p.tenant_id::text || ':' || p.event_id::text, 0));
    DELETE FROM sequent_backend.operational_signing_intent AS i
    WHERE (i.tenant_id, i.event_id, i.request_id, i.actor, i.purpose)
        = (p.tenant_id, p.event_id, p.request_id, p.actor, p.purpose)
      AND i.binding IS NOT DISTINCT FROM p AND i.phase = 'prepared';
    RETURN FOUND;
END;
$$;

-- Only a trusted cryptographic verifier/authz caller may pass the already
-- checked proof expiry. Revalidate request, certificate, and exact pre-auth
-- binding under the election lock immediately before reserving device work.
CREATE FUNCTION sequent_backend.operational_signing_reserve(
    p sequent_backend.operational_signing_binding, p_verified_auth_time bigint, p_proof_expires_at timestamptz
) RETURNS uuid LANGUAGE plpgsql SECURITY INVOKER
SET search_path = pg_catalog
AS $$
DECLARE
    v_id uuid;
    v_now timestamptz;
    v_deadline timestamptz;
BEGIN
    IF NOT sequent_backend.operational_signing_validate(p) THEN RETURN NULL; END IF;
    v_now := clock_timestamp();
    IF p_proof_expires_at IS NULL OR NOT isfinite(p_proof_expires_at)
       OR p_proof_expires_at <= v_now OR p.intent_expires_at <= v_now
       OR p_verified_auth_time IS NULL
       OR p_verified_auth_time <= floor(extract(epoch FROM p.challenge_issued_at))
       OR p_verified_auth_time > floor(extract(epoch FROM v_now))
       OR p_verified_auth_time > floor(extract(epoch FROM p_proof_expires_at))
       OR p_verified_auth_time < ceil(extract(epoch FROM v_now - interval '2 minutes')) THEN
        RETURN NULL;
    END IF;
    IF p.purpose = 'pdf-cms' AND EXISTS (
        SELECT FROM sequent_backend.operational_signing_intent AS i
        WHERE (i.tenant_id, i.event_id, i.request_id) = (p.tenant_id, p.event_id, p.request_id)
          AND i.purpose = 'pdf-cms'
          AND i.phase IN ('reserved', 'invoked', 'completed', 'indeterminate')
    ) THEN
        RETURN NULL;
    END IF;
    v_deadline := LEAST(p_proof_expires_at, p.intent_expires_at, v_now + interval '30 seconds');
    UPDATE sequent_backend.operational_signing_intent AS i
    SET phase = 'reserved', reservation_id = gen_random_uuid(), reserved_at = v_now,
        reservation_deadline = v_deadline
    WHERE (i.tenant_id, i.event_id, i.request_id, i.actor, i.purpose)
        = (p.tenant_id, p.event_id, p.request_id, p.actor, p.purpose)
      AND i.binding IS NOT DISTINCT FROM p AND i.phase = 'prepared'
    RETURNING i.reservation_id INTO v_id;
    RETURN v_id;
END;
$$;

-- Only one owner may claim the scheduled device invocation. Commit this
-- transition before calling the HSM, never starting after the DB deadline.
-- A crash after commit remains ambiguous, not automatically retryable.
CREATE FUNCTION sequent_backend.operational_signing_claim_device(
    p sequent_backend.operational_signing_binding, p_reservation_id uuid
) RETURNS boolean LANGUAGE plpgsql SECURITY INVOKER
SET search_path = pg_catalog
AS $$
DECLARE
    v_now timestamptz;
BEGIN
    IF NOT sequent_backend.operational_signing_validate(p) THEN RETURN false; END IF;
    v_now := clock_timestamp();
    UPDATE sequent_backend.operational_signing_intent AS i
    SET phase = 'invoked', invoked_at = v_now
    WHERE (i.tenant_id, i.event_id, i.request_id, i.actor, i.purpose)
        = (p.tenant_id, p.event_id, p.request_id, p.actor, p.purpose)
      AND i.binding IS NOT DISTINCT FROM p AND i.phase = 'reserved'
      AND i.reservation_id = p_reservation_id AND v_now < i.reservation_deadline;
    RETURN FOUND;
END;
$$;

-- The caller must commit the device claim before touching the device. Settling stores
-- evidence, not approval; the request may have closed during the external call.
-- The trusted backend must validate receipt contents/signature before writing.
-- Accept late evidence if the reserved fence has not been quarantined; never
-- start a device call after the recorded deadline. Neither read nor settle
-- authorizes approval or dispatch.
CREATE FUNCTION sequent_backend.operational_signing_settle(
    p sequent_backend.operational_signing_binding, p_reservation_id uuid, p_receipt bytea
) RETURNS boolean LANGUAGE plpgsql SECURITY INVOKER
SET search_path = pg_catalog
AS $$
BEGIN
    IF p_receipt IS NULL OR octet_length(p_receipt) NOT BETWEEN 1 AND 16384 THEN
        RETURN false;
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'signing-event:' || p.tenant_id::text || ':' || p.event_id::text, 0));
    UPDATE sequent_backend.operational_signing_intent AS i
    SET phase = 'completed', settled_at = clock_timestamp(), receipt = p_receipt
    WHERE (i.tenant_id, i.event_id, i.request_id, i.actor, i.purpose)
        = (p.tenant_id, p.event_id, p.request_id, p.actor, p.purpose)
      AND i.binding IS NOT DISTINCT FROM p AND i.phase = 'invoked'
      AND i.reservation_id = p_reservation_id;
    RETURN FOUND;
END;
$$;

-- Explicit quarantine invoked only after trusted recovery determines the
-- outcome is unknown. No transition back to reserved, even with a new nonce.
CREATE FUNCTION sequent_backend.operational_signing_quarantine(
    p sequent_backend.operational_signing_binding, p_reservation_id uuid
) RETURNS boolean LANGUAGE plpgsql SECURITY INVOKER
SET search_path = pg_catalog
AS $$
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'signing-event:' || p.tenant_id::text || ':' || p.event_id::text, 0));
    UPDATE sequent_backend.operational_signing_intent AS i
    SET phase = 'indeterminate', settled_at = clock_timestamp()
    WHERE (i.tenant_id, i.event_id, i.request_id, i.actor, i.purpose)
        = (p.tenant_id, p.event_id, p.request_id, p.actor, p.purpose)
      AND i.binding IS NOT DISTINCT FROM p AND i.phase IN ('reserved', 'invoked')
      AND i.reservation_id = p_reservation_id AND i.reservation_deadline <= clock_timestamp();
    RETURN FOUND;
END;
$$;

-- Exact read-only evidence, admin ingress must still authorize actor/tenant
-- and action permission. A grant expiry does not destroy an existing receipt.
CREATE FUNCTION sequent_backend.operational_signing_read(
    p sequent_backend.operational_signing_binding
) RETURNS TABLE(phase text, reservation_id uuid, reservation_deadline timestamptz, receipt bytea) LANGUAGE sql STABLE SECURITY INVOKER
SET search_path = pg_catalog
AS $$
    SELECT i.phase, i.reservation_id, i.reservation_deadline, i.receipt FROM sequent_backend.operational_signing_intent AS i
    WHERE (i.tenant_id, i.event_id, i.request_id, i.actor, i.purpose)
        = (p.tenant_id, p.event_id, p.request_id, p.actor, p.purpose)
      AND i.binding IS NOT DISTINCT FROM p;
$$;

-- Commit callers must verify current identity/approval authorization first.
-- A lookup only supplies the exact completed grant to that trusted caller.
CREATE FUNCTION sequent_backend.operational_signing_get_completed(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_challenge uuid
) RETURNS TABLE (
    binding jsonb, phase text, reservation_id uuid, receipt bytea, certificate_pem text
) LANGUAGE plpgsql SECURITY INVOKER SET search_path = pg_catalog
AS $$
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'signing-event:' || p_tenant::text || ':' || p_event::text, 0));
    PERFORM FROM sequent_backend.signing_request AS q
    WHERE (q.tenant_id, q.election_event_id, q.id) = (p_tenant, p_event, p_request)
    FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;

    RETURN QUERY
    SELECT to_jsonb(i.binding), i.phase, i.reservation_id, i.receipt, cert.pem
    FROM sequent_backend.operational_signing_intent AS i
    LEFT JOIN sequent_backend.staff_certificate AS cert
      ON (cert.tenant_id, cert.election_event_id, cert.user_id) = (i.tenant_id, i.event_id, i.actor)
      AND cert.fingerprint_sha256 = (i.binding).cert_fingerprint
      AND cert.spki_sha256 = (i.binding).cert_spki
      AND cert.status = 'active'
      AND cert.not_before <= statement_timestamp() AND cert.not_after > statement_timestamp()
    WHERE (i.tenant_id, i.event_id, i.request_id, i.actor, i.challenge_id)
        = (p_tenant, p_event, p_request, p_actor, p_challenge)
      AND (i.phase = 'applied' OR (i.phase = 'completed' AND cert.id IS NOT NULL))
    FOR UPDATE OF i;
END;
$$;

-- Call inside the same approval savepoint directly after record_signature,
-- before dispatch. The caller must cryptographically verify p_receipt and
-- derive p_approval_bytes (CMS for PDF) from it; SQL compares/stores evidence
-- and does not verify a device signature or certificate chain itself.
CREATE FUNCTION sequent_backend.operational_signing_mark_applied(
    p sequent_backend.operational_signing_binding, p_reservation_id uuid,
    p_receipt bytea, p_approval_bytes bytea
) RETURNS boolean LANGUAGE plpgsql SECURITY INVOKER SET search_path = pg_catalog
AS $$
DECLARE
    ledger sequent_backend.operational_signing_intent%ROWTYPE;
    signed_revision_id uuid;
    signed_document_id uuid;
    signed_sha256 text;
BEGIN
    IF p_receipt IS NULL OR p_approval_bytes IS NULL OR octet_length(p_approval_bytes) = 0 THEN
        RETURN false;
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'signing-event:' || p.tenant_id::text || ':' || p.event_id::text, 0));
    PERFORM FROM sequent_backend.signing_request AS q
    WHERE (q.tenant_id, q.election_event_id, q.id) = (p.tenant_id, p.event_id, p.request_id)
      AND q.status = 'waiting' AND q.action = p.action AND q.payload_sha256 = p.payload_hash
    FOR UPDATE;
    IF NOT FOUND THEN RETURN false; END IF;

    SELECT * INTO ledger FROM sequent_backend.operational_signing_intent AS i
    WHERE (i.tenant_id, i.event_id, i.request_id, i.actor, i.purpose)
        = (p.tenant_id, p.event_id, p.request_id, p.actor, p.purpose)
      AND i.binding IS NOT DISTINCT FROM p
      AND i.phase = 'completed' AND i.reservation_id = p_reservation_id AND i.receipt = p_receipt
    FOR UPDATE;
    IF NOT FOUND THEN RETURN false; END IF;

    PERFORM FROM sequent_backend.signing_approval AS a
    JOIN sequent_backend.staff_certificate AS cert
      ON (cert.tenant_id, cert.election_event_id, cert.id) =
         (a.tenant_id, a.election_event_id, a.certificate_id)
    WHERE (a.tenant_id, a.election_event_id, a.request_id, a.user_id) =
          (p.tenant_id, p.event_id, p.request_id, p.actor)
      AND a.fingerprint_sha256 = p.cert_fingerprint AND a.spki_sha256 = p.cert_spki
      AND cert.user_id = p.actor AND cert.fingerprint_sha256 = p.cert_fingerprint
      AND cert.spki_sha256 = p.cert_spki AND cert.status = 'active'
      AND a.created_at >= GREATEST(transaction_timestamp(), ledger.settled_at)
      AND CASE p.purpose
        WHEN 'approval-payload' THEN a.payload_signature
        WHEN 'eml' THEN a.document_signature
        WHEN 'pdf-cms' THEN a.pdf_cms
      END = p_approval_bytes
    FOR SHARE OF a, cert;
    IF NOT FOUND THEN RETURN false; END IF;

    IF p.purpose = 'pdf-cms' THEN
        SELECT signed.id, signed.document_id, signed.sha256
        INTO signed_revision_id, signed_document_id, signed_sha256
        FROM sequent_backend.signing_document_revision AS prep
        JOIN sequent_backend.signing_document_revision AS signed
          ON (signed.tenant_id, signed.election_event_id, signed.request_id) =
             (prep.tenant_id, prep.election_event_id, prep.request_id)
        WHERE (prep.tenant_id, prep.election_event_id, prep.request_id) =
              (p.tenant_id, p.event_id, p.request_id)
          AND prep.state = 'prepared' AND prep.revision = p.pdf_revision
          AND prep.prepared_for_user = p.actor AND prep.field_index IS NOT NULL
          AND prep.parent_sha256 = p.pdf_parent_hash AND prep.digest_sha256 = p.pdf_digest
          AND prep.appearance->>'certificate_sha256' = p.cert_fingerprint
          AND signed.state = 'signed' AND signed.document_id IS NOT NULL
          AND signed.prepared_for_user = p.actor AND signed.field_index = prep.field_index
          AND signed.parent_sha256 = prep.parent_sha256 AND signed.digest_sha256 = prep.digest_sha256
          AND signed.byte_range = prep.byte_range AND signed.appearance = prep.appearance
          AND signed.created_at >= GREATEST(transaction_timestamp(), ledger.settled_at)
          AND signed.revision = (
              SELECT max(latest.revision) FROM sequent_backend.signing_document_revision AS latest
              WHERE (latest.tenant_id, latest.election_event_id, latest.request_id) =
                    (p.tenant_id, p.event_id, p.request_id)
                AND latest.state IN ('base', 'signed')
          )
        FOR SHARE OF prep, signed;
        IF NOT FOUND THEN RETURN false; END IF;
    END IF;

    UPDATE sequent_backend.operational_signing_intent AS i
    SET phase = 'applied', applied_at = clock_timestamp(),
        applied_signature_sha256 = encode(sha256(p_approval_bytes), 'hex'),
        applied_pdf_revision_id = signed_revision_id,
        applied_pdf_document_id = signed_document_id,
        applied_pdf_sha256 = signed_sha256
    WHERE i.reservation_id = ledger.reservation_id AND i.phase = 'completed';
    RETURN FOUND;
END;
$$;

-- Scoped wire projection without requiring the caller to construct a
-- database composite. All receipt/binding evidence is checked by mark_applied.
CREATE FUNCTION sequent_backend.operational_signing_mark_applied_from_evidence(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_challenge uuid,
    p_reservation_id uuid, p_receipt bytea, p_approval_bytes bytea
) RETURNS boolean LANGUAGE plpgsql SECURITY INVOKER SET search_path = pg_catalog
AS $$
DECLARE
    p sequent_backend.operational_signing_binding;
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'signing-event:' || p_tenant::text || ':' || p_event::text, 0));
    SELECT (i.binding).* INTO p
    FROM sequent_backend.operational_signing_intent AS i
    WHERE (i.tenant_id, i.event_id, i.request_id, i.actor, i.challenge_id)
        = (p_tenant, p_event, p_request, p_actor, p_challenge)
      AND i.phase = 'completed' AND i.reservation_id = p_reservation_id;
    IF NOT FOUND THEN RETURN false; END IF;
    RETURN sequent_backend.operational_signing_mark_applied(
        p, p_reservation_id, p_receipt, p_approval_bytes);
END;
$$;

REVOKE ALL ON TABLE sequent_backend.operational_signing_intent,
    sequent_backend.operational_signing_ceremony FROM PUBLIC;
REVOKE ALL ON FUNCTION sequent_backend.operational_signing_immutable(),
    sequent_backend.operational_signing_validate(sequent_backend.operational_signing_binding),
    sequent_backend.operational_signing_prepare(sequent_backend.operational_signing_binding),
    sequent_backend.operational_signing_discard_prepared(sequent_backend.operational_signing_binding),
    sequent_backend.operational_signing_reserve(sequent_backend.operational_signing_binding, bigint, timestamptz),
    sequent_backend.operational_signing_claim_device(sequent_backend.operational_signing_binding, uuid),
    sequent_backend.operational_signing_settle(sequent_backend.operational_signing_binding, uuid, bytea),
    sequent_backend.operational_signing_quarantine(sequent_backend.operational_signing_binding, uuid),
    sequent_backend.operational_signing_read(sequent_backend.operational_signing_binding),
    sequent_backend.operational_signing_get_completed(uuid, uuid, uuid, text, uuid),
    sequent_backend.operational_signing_mark_applied(sequent_backend.operational_signing_binding, uuid, bytea, bytea),
    sequent_backend.operational_signing_mark_applied_from_evidence(uuid, uuid, uuid, text, uuid, uuid, bytea, bytea) FROM PUBLIC;
REVOKE ALL ON TYPE sequent_backend.operational_signing_binding FROM PUBLIC;
