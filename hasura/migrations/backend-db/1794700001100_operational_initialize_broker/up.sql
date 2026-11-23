-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Initialize voting ceremony store. Not tracked in Hasura. The private
-- orchestrator owns challenge creation, callback redemption and reservation;
-- the approval caller only reads the state of its own actor, cancels an
-- unredeemed attempt and keeps the browser return page. No caller has table
-- privileges: every step goes through the functions below, which reuse the
-- ledger transitions of 1794700000700 unchanged.
GRANT CREATE ON SCHEMA sequent_backend
    TO sequent_os_ledger_data_owner, sequent_os_ledger_api_owner;

-- Typed continuation carried between the steps of one attempt. A callback is
-- redeemed once, reserved once, claimed by the device once and settled once.
CREATE TYPE sequent_backend.operational_initialize_step AS ENUM (
    'issued', 'redeemed', 'reserved', 'device-claimed', 'settled', 'discarded'
);
CREATE TYPE sequent_backend.operational_signing_flow AS ENUM ('initialize', 'generic');
CREATE TYPE sequent_backend.operational_initialize_return_path AS ENUM (
    'event-base', 'event-show', 'event-edit', 'election-base', 'election-show', 'election-edit'
);
CREATE TYPE sequent_backend.operational_initialize_return_panel AS ENUM (
    'publish', 'signatures-requests'
);

-- One row per challenge, kept after a discard so that an attempt identifier
-- is never accepted twice. Unlinked from the intent on purpose: discarding a
-- never-reserved intent deletes it, the attempt history stays.
CREATE TABLE sequent_backend.operational_initialize_attempt (
    challenge_id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL,
    event_id uuid NOT NULL,
    request_id uuid NOT NULL,
    actor text NOT NULL CHECK (actor ~ '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'),
    payload_sha256 text NOT NULL CHECK (payload_sha256 ~ '^[0-9a-f]{64}$'),
    binding_sha256 text NOT NULL CHECK (binding_sha256 ~ '^[0-9a-f]{64}$'),
    device_profile text NOT NULL CHECK (device_profile IN ('yubihsm', 'softhsm-test')),
    step sequent_backend.operational_initialize_step NOT NULL DEFAULT 'issued',
    issued_at timestamptz NOT NULL,
    expires_at timestamptz NOT NULL,
    redeemed_at timestamptz,
    reserved_at timestamptz,
    reservation_id uuid UNIQUE,
    device_claimed_at timestamptz,
    settled_at timestamptz,
    discarded_at timestamptz,
    return_path sequent_backend.operational_initialize_return_path,
    return_panel sequent_backend.operational_initialize_return_panel,
    return_election_id uuid,
    return_recorded_at timestamptz,
    FOREIGN KEY (tenant_id, event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE,
    CHECK (issued_at < expires_at AND expires_at <= issued_at + interval '5 minutes'),
    CHECK ((step IN ('issued', 'discarded')) OR redeemed_at IS NOT NULL),
    CHECK ((step IN ('reserved', 'device-claimed', 'settled')) = (reservation_id IS NOT NULL)),
    CHECK ((reservation_id IS NULL) = (reserved_at IS NULL)),
    CHECK ((step IN ('device-claimed', 'settled')) = (device_claimed_at IS NOT NULL)),
    CHECK ((step = 'settled') = (settled_at IS NOT NULL)),
    CHECK ((step = 'discarded') = (discarded_at IS NOT NULL)),
    CHECK ((return_path IS NULL) = (return_panel IS NULL)),
    CHECK ((return_path IS NULL) = (return_recorded_at IS NULL)),
    CHECK (return_election_id IS NULL OR return_path IS NOT NULL)
);
CREATE INDEX operational_initialize_attempt_by_request
    ON sequent_backend.operational_initialize_attempt (tenant_id, event_id, request_id, actor);

-- What the reservation proved, for the device side. Written only together
-- with the ledger reservation; the device profile recorded here is the
-- provenance later required of an applied receipt.
CREATE TABLE sequent_backend.operational_signing_device_grant (
    reservation_id uuid PRIMARY KEY,
    challenge_id uuid NOT NULL UNIQUE,
    tenant_id uuid NOT NULL,
    event_id uuid NOT NULL,
    request_id uuid NOT NULL,
    actor text NOT NULL,
    flow sequent_backend.operational_signing_flow NOT NULL,
    device_profile text NOT NULL CHECK (device_profile IN ('yubihsm', 'softhsm-test')),
    verified_issuer text NOT NULL CHECK (btrim(verified_issuer) <> '' AND octet_length(verified_issuer) <= 1024),
    verified_auth_time bigint NOT NULL CHECK (verified_auth_time > 0),
    proof_expires_at timestamptz NOT NULL,
    granted_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    claimed_at timestamptz,
    ca_version text CHECK (ca_version ~ '^[0-9a-f]{64}$'),
    settled_at timestamptz,
    FOREIGN KEY (tenant_id, event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE,
    CHECK (settled_at IS NULL OR claimed_at IS NOT NULL),
    CHECK (ca_version IS NULL OR claimed_at IS NOT NULL),
    CHECK (flow <> 'initialize' OR (claimed_at IS NULL) = (ca_version IS NULL))
);

CREATE FUNCTION sequent_backend.operational_initialize_attempt_guard()
RETURNS trigger LANGUAGE plpgsql SECURITY INVOKER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
BEGIN
    IF ROW(NEW.challenge_id, NEW.tenant_id, NEW.event_id, NEW.request_id, NEW.actor,
           NEW.payload_sha256, NEW.binding_sha256, NEW.device_profile, NEW.issued_at, NEW.expires_at)
       IS DISTINCT FROM
       ROW(OLD.challenge_id, OLD.tenant_id, OLD.event_id, OLD.request_id, OLD.actor,
           OLD.payload_sha256, OLD.binding_sha256, OLD.device_profile, OLD.issued_at, OLD.expires_at)
       OR NOT (
           NEW.step = OLD.step
           OR (OLD.step = 'issued' AND NEW.step IN ('redeemed', 'discarded'))
           OR (OLD.step = 'redeemed' AND NEW.step IN ('reserved', 'discarded'))
           OR (OLD.step = 'reserved' AND NEW.step = 'device-claimed')
           OR (OLD.step = 'device-claimed' AND NEW.step = 'settled')
       )
       OR (OLD.redeemed_at IS NOT NULL AND NEW.redeemed_at IS DISTINCT FROM OLD.redeemed_at)
       OR (OLD.reservation_id IS NOT NULL
           AND ROW(NEW.reservation_id, NEW.reserved_at) IS DISTINCT FROM ROW(OLD.reservation_id, OLD.reserved_at))
       OR (OLD.device_claimed_at IS NOT NULL AND NEW.device_claimed_at IS DISTINCT FROM OLD.device_claimed_at)
       -- The return page is written once, before the callback can be redeemed.
       OR (ROW(NEW.return_path, NEW.return_panel, NEW.return_election_id, NEW.return_recorded_at)
               IS DISTINCT FROM
               ROW(OLD.return_path, OLD.return_panel, OLD.return_election_id, OLD.return_recorded_at)
           AND (OLD.return_path IS NOT NULL OR OLD.step <> 'issued' OR NEW.step <> 'issued')) THEN
        RAISE EXCEPTION 'invalid operational Initialize attempt update'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER operational_initialize_attempt_guard BEFORE UPDATE
ON sequent_backend.operational_initialize_attempt
FOR EACH ROW EXECUTE FUNCTION sequent_backend.operational_initialize_attempt_guard();

CREATE FUNCTION sequent_backend.operational_signing_device_grant_guard()
RETURNS trigger LANGUAGE plpgsql SECURITY INVOKER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
BEGIN
    IF ROW(NEW.reservation_id, NEW.challenge_id, NEW.tenant_id, NEW.event_id, NEW.request_id,
           NEW.actor, NEW.flow, NEW.device_profile, NEW.verified_issuer, NEW.verified_auth_time,
           NEW.proof_expires_at, NEW.granted_at)
       IS DISTINCT FROM
       ROW(OLD.reservation_id, OLD.challenge_id, OLD.tenant_id, OLD.event_id, OLD.request_id,
           OLD.actor, OLD.flow, OLD.device_profile, OLD.verified_issuer, OLD.verified_auth_time,
           OLD.proof_expires_at, OLD.granted_at)
       OR (OLD.claimed_at IS NOT NULL
           AND ROW(NEW.claimed_at, NEW.ca_version) IS DISTINCT FROM ROW(OLD.claimed_at, OLD.ca_version))
       OR (OLD.settled_at IS NOT NULL AND NEW.settled_at IS DISTINCT FROM OLD.settled_at) THEN
        RAISE EXCEPTION 'invalid operational device grant update'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER operational_signing_device_grant_guard BEFORE UPDATE
ON sequent_backend.operational_signing_device_grant
FOR EACH ROW EXECUTE FUNCTION sequent_backend.operational_signing_device_grant_guard();

-- Internal. The waiting Initialize request together with the official's one
-- active hardware enrollment and its current attached event certificate, for
-- the exact device profile of the installation. Empty when anything about
-- the tenant mode, request, key, certificate or revocation is not current.
CREATE FUNCTION sequent_backend.operational_initialize_current(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_payload_sha256 text,
    p_profile text
) RETURNS TABLE (
    canonical_payload text, key_id text, key_version integer, cert_fingerprint text,
    cert_spki text, request_expiry timestamptz, enrollment_id uuid, certificate_id uuid
) LANGUAGE sql STABLE SECURITY INVOKER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT r.canonical_payload, e.key_id, e.key_version, a.certificate_fingerprint, e.spki,
           r.expires_at, e.id, a.certificate_id
    FROM sequent_backend.signing_request AS r
    JOIN sequent_backend.operational_signing_official_enrollment AS e
      ON e.tenant_id = r.tenant_id AND e.actor = p_actor
    JOIN sequent_backend.operational_signing_official_certificate AS a
      ON (a.tenant_id, a.actor, a.enrollment_id) = (e.tenant_id, e.actor, e.id)
     AND a.event_id = r.election_event_id
    JOIN sequent_backend.staff_certificate AS c
      ON (c.tenant_id, c.election_event_id, c.id) = (a.tenant_id, a.event_id, a.certificate_id)
    WHERE (r.tenant_id, r.election_event_id, r.id) = (p_tenant, p_event, p_request)
      AND sequent_backend.operational_signing_hsm_required(p_tenant)
      AND p_actor ~ '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
      AND p_payload_sha256 ~ '^[0-9a-f]{64}$'
      AND r.status = 'waiting' AND r.action = 'initialize-voting'
      AND r.payload_sha256 = p_payload_sha256
      AND r.document_id IS NULL AND r.document_sha256 IS NULL
      AND octet_length(r.canonical_payload) BETWEEN 1 AND 524288
      AND encode(sha256(convert_to(r.canonical_payload, 'UTF8')), 'hex') = r.payload_sha256
      AND r.expires_at IS NOT NULL AND isfinite(r.expires_at) AND r.expires_at > clock_timestamp()
      AND e.revoked_at IS NULL AND e.nonexportable_confirmed AND e.device_profile = p_profile
      AND e.key_id = 'official/' || p_tenant::text || '/' || p_actor
      AND a.retired_at IS NULL
      AND c.user_id = p_actor AND c.status = 'active'
      AND c.fingerprint_sha256 = a.certificate_fingerprint AND c.spki_sha256 = e.spki
      AND c.pem = a.chain_pem[1]
      AND c.not_before <= clock_timestamp() AND c.not_after > clock_timestamp()
      AND (c.election_id IS NULL OR c.election_id = r.election_id)
      AND NOT EXISTS (
          SELECT FROM sequent_backend.staff_certificate AS x
          WHERE x.tenant_id = p_tenant AND x.status = 'revoked'
            AND (x.fingerprint_sha256 = a.certificate_fingerprint OR x.spki_sha256 = e.spki)
      );
$$;

-- Orchestrator: what a challenge for this exact scope would be bound to,
-- stamped with this transaction's time. Allocates nothing.
CREATE FUNCTION sequent_backend.operational_initialize_context(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_payload_sha256 text,
    p_profile text
) RETURNS TABLE (
    canonical_payload text, payload_sha256 text, key_id text, key_version integer,
    cert_fingerprint text, cert_spki text, issued_at timestamptz, request_expiry timestamptz,
    intent_expiry timestamptz
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT c.canonical_payload, p_payload_sha256, c.key_id, c.key_version, c.cert_fingerprint,
           c.cert_spki, transaction_timestamp(), c.request_expiry,
           LEAST(transaction_timestamp() + interval '4 minutes', c.request_expiry)
    FROM sequent_backend.operational_initialize_current(
        p_tenant, p_event, p_request, p_actor, p_payload_sha256, p_profile) AS c
    WHERE c.request_expiry > transaction_timestamp();
$$;

-- Orchestrator: create the challenge in the transaction that read the
-- context. An attempt identifier and an issuer nonce are each usable once.
-- An expired challenge that never reserved a device is superseded here.
CREATE FUNCTION sequent_backend.operational_initialize_prepare(
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

-- Orchestrator: the exact stored challenge of this actor and attempt, with
-- the request's present immutable payload.
CREATE FUNCTION sequent_backend.operational_initialize_load(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_payload_sha256 text,
    p_attempt uuid
) RETURNS TABLE (binding jsonb, phase text, canonical_payload text)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT to_jsonb(i.binding), i.phase, r.canonical_payload
    FROM sequent_backend.operational_initialize_attempt AS t
    JOIN sequent_backend.operational_signing_intent AS i ON i.challenge_id = t.challenge_id
    JOIN sequent_backend.signing_request AS r
      ON (r.tenant_id, r.election_event_id, r.id) = (i.tenant_id, i.event_id, i.request_id)
    WHERE t.challenge_id = p_attempt AND t.step <> 'discarded'
      AND (t.tenant_id, t.event_id, t.request_id, t.actor, t.payload_sha256)
        = (p_tenant, p_event, p_request, p_actor, p_payload_sha256)
      AND (i.tenant_id, i.event_id, i.request_id, i.actor, i.purpose)
        = (p_tenant, p_event, p_request, p_actor, 'approval-payload')
      AND (i.binding).action = 'initialize-voting'
      AND (i.binding).payload_hash = p_payload_sha256
      AND (i.binding).binding_sha256 = t.binding_sha256
      AND r.action = 'initialize-voting' AND r.payload_sha256 = p_payload_sha256
      AND octet_length(r.canonical_payload) BETWEEN 1 AND 524288
      AND encode(sha256(convert_to(r.canonical_payload, 'UTF8')), 'hex') = p_payload_sha256
      AND sequent_backend.operational_signing_hsm_required(p_tenant);
$$;

-- Orchestrator: burn the callback before any token exchange. One winner.
CREATE FUNCTION sequent_backend.operational_initialize_take_callback(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_attempt uuid,
    p_binding_sha256 text
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
BEGIN
    IF p_tenant IS NULL OR p_event IS NULL
       OR NOT sequent_backend.operational_signing_hsm_required(p_tenant) THEN
        RETURN false;
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'signing-event:' || p_tenant::text || ':' || p_event::text, 0));
    UPDATE sequent_backend.operational_initialize_attempt AS t
    SET step = 'redeemed', redeemed_at = clock_timestamp()
    WHERE t.challenge_id = p_attempt
      AND (t.tenant_id, t.event_id, t.request_id, t.actor, t.binding_sha256)
        = (p_tenant, p_event, p_request, p_actor, p_binding_sha256)
      AND t.step = 'issued' AND t.expires_at > clock_timestamp()
      AND EXISTS (
          SELECT FROM sequent_backend.operational_signing_intent AS i
          WHERE i.challenge_id = t.challenge_id AND i.phase = 'prepared'
            AND (i.tenant_id, i.event_id, i.request_id, i.actor, i.purpose)
              = (p_tenant, p_event, p_request, p_actor, 'approval-payload')
            AND (i.binding).binding_sha256 = p_binding_sha256
      );
    RETURN FOUND;
END;
$$;

-- Orchestrator: reserve the device use for its own redeemed callback and a
-- verifier-produced proof, against the enrollment that is current now.
-- A refusal leaves the callback burned; it is never offered again.
CREATE FUNCTION sequent_backend.operational_initialize_reserve_own_callback(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_attempt uuid,
    p_binding_sha256 text, p_nonce_sha256 text, p_issuer text, p_verified_auth_time bigint,
    p_proof_expires_at timestamptz, p_profile text
) RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
DECLARE
    b sequent_backend.operational_signing_binding;
    c record;
    v_id uuid;
BEGIN
    IF p_tenant IS NULL OR p_event IS NULL OR p_verified_auth_time IS NULL
       OR p_verified_auth_time <= 0 OR p_proof_expires_at IS NULL
       OR NOT isfinite(p_proof_expires_at)
       OR NOT sequent_backend.operational_signing_hsm_required(p_tenant) THEN
        RETURN NULL;
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'signing-event:' || p_tenant::text || ':' || p_event::text, 0));
    PERFORM FROM sequent_backend.operational_initialize_attempt AS t
    WHERE t.challenge_id = p_attempt
      AND (t.tenant_id, t.event_id, t.request_id, t.actor, t.binding_sha256, t.device_profile)
        = (p_tenant, p_event, p_request, p_actor, p_binding_sha256, p_profile)
      AND t.step = 'redeemed'
    FOR UPDATE;
    IF NOT FOUND THEN RETURN NULL; END IF;
    SELECT (i.binding).* INTO b
    FROM sequent_backend.operational_signing_intent AS i
    WHERE i.challenge_id = p_attempt AND i.phase = 'prepared'
      AND (i.tenant_id, i.event_id, i.request_id, i.actor, i.purpose)
        = (p_tenant, p_event, p_request, p_actor, 'approval-payload');
    IF NOT FOUND OR b.action <> 'initialize-voting'
       OR b.binding_sha256 IS DISTINCT FROM p_binding_sha256
       OR b.nonce_hash IS DISTINCT FROM p_nonce_sha256
       OR b.issuer IS DISTINCT FROM p_issuer THEN
        RETURN NULL;
    END IF;
    SELECT * INTO c FROM sequent_backend.operational_initialize_current(
        p_tenant, p_event, p_request, p_actor, b.payload_hash, p_profile);
    IF NOT FOUND
       OR ROW(c.key_id, c.key_version, c.cert_fingerprint, c.cert_spki)
          IS DISTINCT FROM ROW(b.key_id, b.key_version, b.cert_fingerprint, b.cert_spki) THEN
        RETURN NULL;
    END IF;
    v_id := sequent_backend.operational_signing_reserve(b, p_verified_auth_time, p_proof_expires_at);
    IF v_id IS NULL THEN RETURN NULL; END IF;
    INSERT INTO sequent_backend.operational_signing_device_grant
        (reservation_id, challenge_id, tenant_id, event_id, request_id, actor, flow,
         device_profile, verified_issuer, verified_auth_time, proof_expires_at)
    VALUES (v_id, p_attempt, p_tenant, p_event, p_request, p_actor, 'initialize',
            p_profile, p_issuer, p_verified_auth_time, p_proof_expires_at);
    UPDATE sequent_backend.operational_initialize_attempt AS t
    SET step = 'reserved', reservation_id = v_id, reserved_at = clock_timestamp()
    WHERE t.challenge_id = p_attempt AND t.step = 'redeemed';
    IF NOT FOUND THEN
        RAISE EXCEPTION 'operational Initialize attempt changed while reserving';
    END IF;
    RETURN v_id;
END;
$$;

-- Approval caller: the live attempt of this actor for this request. An
-- expired challenge that never reserved a device is not a live attempt.
CREATE FUNCTION sequent_backend.operational_initialize_current_state(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_payload_sha256 text
) RETURNS TABLE (phase text, challenge uuid, callback_claimed boolean)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT i.phase, i.challenge_id, t.step <> 'issued'
    FROM sequent_backend.operational_signing_intent AS i
    JOIN sequent_backend.operational_initialize_attempt AS t ON t.challenge_id = i.challenge_id
    WHERE (i.tenant_id, i.event_id, i.request_id, i.actor, i.purpose)
        = (p_tenant, p_event, p_request, p_actor, 'approval-payload')
      AND (t.tenant_id, t.event_id, t.request_id, t.actor, t.payload_sha256)
        = (p_tenant, p_event, p_request, p_actor, p_payload_sha256)
      AND t.step <> 'discarded'
      AND (i.binding).action = 'initialize-voting'
      AND (i.binding).payload_hash = p_payload_sha256
      AND NOT (i.phase = 'prepared' AND (i.binding).intent_expires_at <= clock_timestamp())
      AND sequent_backend.operational_signing_hsm_required(p_tenant);
$$;

-- Approval caller: cancel the actor's own attempt while it provably cannot
-- reach the device: the callback is unredeemed, or the challenge expired
-- before any reservation. The issuer nonce stays consumed.
CREATE FUNCTION sequent_backend.operational_initialize_discard_unredeemed(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_payload_sha256 text,
    p_attempt uuid
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
DECLARE
    b sequent_backend.operational_signing_binding;
BEGIN
    IF p_tenant IS NULL OR p_event IS NULL
       OR NOT sequent_backend.operational_signing_hsm_required(p_tenant) THEN
        RETURN false;
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'signing-event:' || p_tenant::text || ':' || p_event::text, 0));
    UPDATE sequent_backend.operational_initialize_attempt AS t
    SET step = 'discarded', discarded_at = clock_timestamp()
    WHERE t.challenge_id = p_attempt
      AND (t.tenant_id, t.event_id, t.request_id, t.actor, t.payload_sha256)
        = (p_tenant, p_event, p_request, p_actor, p_payload_sha256)
      AND (t.step = 'issued' OR (t.step = 'redeemed' AND t.expires_at <= clock_timestamp()));
    IF NOT FOUND THEN RETURN false; END IF;
    SELECT (i.binding).* INTO b
    FROM sequent_backend.operational_signing_intent AS i
    WHERE i.challenge_id = p_attempt AND i.phase = 'prepared'
      AND (i.tenant_id, i.event_id, i.request_id, i.actor, i.purpose)
        = (p_tenant, p_event, p_request, p_actor, 'approval-payload');
    IF NOT FOUND OR NOT sequent_backend.operational_signing_discard_prepared(b) THEN
        RAISE EXCEPTION 'operational Initialize attempt does not match its ledger intent';
    END IF;
    RETURN true;
END;
$$;

-- Approval caller: keep the fixed return page of an attempt, once, while its
-- callback is still unredeemed. Never a URL: two closed enumerations and the
-- request's own Post.
CREATE FUNCTION sequent_backend.operational_initialize_record_browser_return(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_attempt uuid,
    p_path text, p_panel text, p_election uuid
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
BEGIN
    IF p_path IS NULL OR p_panel IS NULL
       OR p_path NOT IN ('event-base', 'event-show', 'event-edit',
                         'election-base', 'election-show', 'election-edit')
       OR p_panel NOT IN ('publish', 'signatures-requests')
       OR NOT sequent_backend.operational_signing_hsm_required(p_tenant) THEN
        RETURN false;
    END IF;
    UPDATE sequent_backend.operational_initialize_attempt AS t
    SET return_path = p_path::sequent_backend.operational_initialize_return_path,
        return_panel = p_panel::sequent_backend.operational_initialize_return_panel,
        return_election_id = p_election,
        return_recorded_at = clock_timestamp()
    WHERE t.challenge_id = p_attempt
      AND (t.tenant_id, t.event_id, t.request_id, t.actor)
        = (p_tenant, p_event, p_request, p_actor)
      AND t.step = 'issued' AND t.expires_at > clock_timestamp() AND t.return_path IS NULL
      AND EXISTS (
          SELECT FROM sequent_backend.signing_request AS r
          WHERE (r.tenant_id, r.election_event_id, r.id) = (p_tenant, p_event, p_request)
            AND r.status = 'waiting' AND r.action = 'initialize-voting'
            AND CASE WHEN p_path IN ('event-base', 'event-show', 'event-edit')
                     THEN p_election IS NULL
                     ELSE p_panel = 'publish' AND p_election IS NOT NULL
                          AND p_election = r.election_id
                END
      );
    RETURN FOUND;
END;
$$;

CREATE FUNCTION sequent_backend.operational_initialize_get_browser_return(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_attempt uuid
) RETURNS TABLE (path_kind text, panel text, election_id uuid)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT t.return_path::text, t.return_panel::text, t.return_election_id
    FROM sequent_backend.operational_initialize_attempt AS t
    WHERE t.challenge_id = p_attempt
      AND (t.tenant_id, t.event_id, t.request_id, t.actor)
        = (p_tenant, p_event, p_request, p_actor)
      AND t.step <> 'discarded' AND t.return_path IS NOT NULL
      AND sequent_backend.operational_signing_hsm_required(p_tenant);
$$;

REVOKE ALL ON TABLE sequent_backend.operational_initialize_attempt,
    sequent_backend.operational_signing_device_grant FROM PUBLIC;
REVOKE ALL ON TYPE sequent_backend.operational_initialize_step,
    sequent_backend.operational_signing_flow,
    sequent_backend.operational_initialize_return_path,
    sequent_backend.operational_initialize_return_panel FROM PUBLIC;
GRANT USAGE ON TYPE sequent_backend.operational_initialize_step,
    sequent_backend.operational_signing_flow,
    sequent_backend.operational_initialize_return_path,
    sequent_backend.operational_initialize_return_panel
    TO sequent_os_ledger_data_owner, sequent_os_ledger_api_owner;
REVOKE ALL ON FUNCTION
    sequent_backend.operational_initialize_attempt_guard(),
    sequent_backend.operational_signing_device_grant_guard(),
    sequent_backend.operational_initialize_current(uuid, uuid, uuid, text, text, text),
    sequent_backend.operational_initialize_context(uuid, uuid, uuid, text, text, text),
    sequent_backend.operational_initialize_prepare(uuid, uuid, uuid, text, text, text, uuid, text, text, text),
    sequent_backend.operational_initialize_load(uuid, uuid, uuid, text, text, uuid),
    sequent_backend.operational_initialize_take_callback(uuid, uuid, uuid, text, uuid, text),
    sequent_backend.operational_initialize_reserve_own_callback(uuid, uuid, uuid, text, uuid, text, text, text, bigint, timestamptz, text),
    sequent_backend.operational_initialize_current_state(uuid, uuid, uuid, text, text),
    sequent_backend.operational_initialize_discard_unredeemed(uuid, uuid, uuid, text, text, uuid),
    sequent_backend.operational_initialize_record_browser_return(uuid, uuid, uuid, text, uuid, text, text, uuid),
    sequent_backend.operational_initialize_get_browser_return(uuid, uuid, uuid, text, uuid)
    FROM PUBLIC;

ALTER TABLE sequent_backend.operational_initialize_attempt
    OWNER TO sequent_os_ledger_data_owner;
ALTER TABLE sequent_backend.operational_signing_device_grant
    OWNER TO sequent_os_ledger_data_owner;
GRANT SELECT, INSERT, UPDATE ON TABLE sequent_backend.operational_initialize_attempt,
    sequent_backend.operational_signing_device_grant TO sequent_os_ledger_api_owner;

ALTER FUNCTION sequent_backend.operational_initialize_attempt_guard()
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_signing_device_grant_guard()
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_initialize_current(uuid, uuid, uuid, text, text, text)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_initialize_context(uuid, uuid, uuid, text, text, text)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_initialize_prepare(uuid, uuid, uuid, text, text, text, uuid, text, text, text)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_initialize_load(uuid, uuid, uuid, text, text, uuid)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_initialize_take_callback(uuid, uuid, uuid, text, uuid, text)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_initialize_reserve_own_callback(uuid, uuid, uuid, text, uuid, text, text, text, bigint, timestamptz, text)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_initialize_current_state(uuid, uuid, uuid, text, text)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_initialize_discard_unredeemed(uuid, uuid, uuid, text, text, uuid)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_initialize_record_browser_return(uuid, uuid, uuid, text, uuid, text, text, uuid)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_initialize_get_browser_return(uuid, uuid, uuid, text, uuid)
    OWNER TO sequent_os_ledger_api_owner;

GRANT EXECUTE ON FUNCTION
    sequent_backend.operational_initialize_context(uuid, uuid, uuid, text, text, text),
    sequent_backend.operational_initialize_prepare(uuid, uuid, uuid, text, text, text, uuid, text, text, text),
    sequent_backend.operational_initialize_load(uuid, uuid, uuid, text, text, uuid),
    sequent_backend.operational_initialize_take_callback(uuid, uuid, uuid, text, uuid, text),
    sequent_backend.operational_initialize_reserve_own_callback(uuid, uuid, uuid, text, uuid, text, text, text, bigint, timestamptz, text)
    TO sequent_os_initialize_orchestrator;
GRANT EXECUTE ON FUNCTION
    sequent_backend.operational_initialize_current_state(uuid, uuid, uuid, text, text),
    sequent_backend.operational_initialize_discard_unredeemed(uuid, uuid, uuid, text, text, uuid),
    sequent_backend.operational_initialize_record_browser_return(uuid, uuid, uuid, text, uuid, text, text, uuid),
    sequent_backend.operational_initialize_get_browser_return(uuid, uuid, uuid, text, uuid)
    TO sequent_os_approve;

REVOKE CREATE ON SCHEMA sequent_backend
    FROM sequent_os_ledger_data_owner, sequent_os_ledger_api_owner;
