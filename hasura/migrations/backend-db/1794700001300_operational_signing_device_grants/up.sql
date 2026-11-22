-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Device side of a reserved operational signature. Not tracked in Hasura.
-- A device principal reads only the grant of an exact reference, claims it
-- once before touching the device and settles its receipt afterwards. The
-- Initialize device and the generic device are different principals and
-- cannot use each other's grants; the CA assessment service only reads.
-- The functions reuse the ledger transitions of 1794700000700 unchanged.
GRANT CREATE ON SCHEMA sequent_backend TO sequent_os_ledger_api_owner;

-- Internal. Callers verify current_user = session_user at start-up, so the
-- session identity is the installed device principal for the grant's flow.
CREATE FUNCTION sequent_backend.operational_signing_device_caller(
    p_flow sequent_backend.operational_signing_flow
) RETURNS boolean LANGUAGE sql STABLE SECURITY INVOKER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT CASE p_flow
        WHEN 'initialize' THEN pg_has_role(session_user, 'sequent_os_initialize_device', 'USAGE')
        WHEN 'generic' THEN pg_has_role(session_user, 'sequent_os_device', 'USAGE')
        ELSE false
    END;
$$;

-- CA assessment and device claim: one digest over everything the staff CA
-- assessment of this certificate depends on in the database. Any change of
-- an issuer, a stored revocation list, the certificate's registrations or
-- the official's enrollment gives another version.
CREATE FUNCTION sequent_backend.operational_initialize_ca_version(
    p_tenant uuid, p_event uuid, p_cert_fingerprint text, p_cert_spki text
) RETURNS text LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT encode(sha256(convert_to(jsonb_build_object(
        'version', 1,
        'tenant', p_tenant,
        'event', p_event,
        'fingerprint', p_cert_fingerprint,
        'spki', p_cert_spki,
        'mode', sequent_backend.operational_signing_hsm_required(p_tenant),
        'issuers', (
            SELECT coalesce(jsonb_agg(v.digest ORDER BY v.digest), '[]'::jsonb)
            FROM (SELECT encode(sha256(convert_to(a.pem, 'UTF8')), 'hex') AS digest
                  FROM sequent_backend.certificate_authority AS a
                  WHERE (a.tenant_id, a.election_event_id, a.purpose)
                      = (p_tenant, p_event, 'staff-signatures')) AS v),
        'revocation_lists', (
            SELECT coalesce(jsonb_agg(jsonb_build_array(
                       l.url, encode(sha256(l.der), 'hex'),
                       (extract(epoch FROM l.last_ok_at) * 1000000)::bigint) ORDER BY l.url),
                   '[]'::jsonb)
            FROM sequent_backend.staff_crl AS l
            WHERE (l.tenant_id, l.election_event_id) = (p_tenant, p_event)
              AND l.der IS NOT NULL AND l.last_ok_at IS NOT NULL),
        'registrations', (
            SELECT coalesce(jsonb_agg(jsonb_build_array(c.id, c.election_event_id, c.status)
                                      ORDER BY c.id), '[]'::jsonb)
            FROM sequent_backend.staff_certificate AS c
            WHERE c.tenant_id = p_tenant
              AND (c.fingerprint_sha256 = p_cert_fingerprint OR c.spki_sha256 = p_cert_spki)),
        'enrollments', (
            SELECT coalesce(jsonb_agg(jsonb_build_array(
                       e.id, e.key_id, e.key_version, e.device_profile, e.revoked_at IS NULL,
                       a.certificate_id, a.retired_at IS NULL) ORDER BY e.id, a.certificate_id),
                   '[]'::jsonb)
            FROM sequent_backend.operational_signing_official_enrollment AS e
            LEFT JOIN sequent_backend.operational_signing_official_certificate AS a
              ON (a.tenant_id, a.actor, a.enrollment_id) = (e.tenant_id, e.actor, e.id)
             AND a.event_id = p_event AND a.certificate_fingerprint = p_cert_fingerprint
            WHERE e.tenant_id = p_tenant AND e.spki = p_cert_spki)
    )::text, 'UTF8')), 'hex')
    WHERE p_tenant IS NOT NULL AND p_event IS NOT NULL
      AND p_cert_fingerprint ~ '^[0-9a-f]{64}$' AND p_cert_spki ~ '^[0-9a-f]{64}$';
$$;

-- CA assessment service: the server-owned chain, issuers and stored
-- revocation lists for one live Initialize reservation of the installed
-- device profile. Read-only; nothing here is taken from the device.
CREATE FUNCTION sequent_backend.operational_initialize_get_ca_enrollment(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_challenge uuid,
    p_fence uuid, p_profile text
) RETURNS TABLE (
    binding jsonb, certificate_id uuid, chain_pem text[], grant_until timestamptz,
    ca_version text, anchors_pem text[], revocation_lists jsonb, locally_revoked boolean
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT to_jsonb(i.binding), a.certificate_id, a.chain_pem, i.reservation_deadline,
           sequent_backend.operational_initialize_ca_version(
               p_tenant, p_event, (i.binding).cert_fingerprint, (i.binding).cert_spki),
           (SELECT coalesce(array_agg(ca.pem ORDER BY ca.created_at, ca.id), '{}'::text[])
            FROM sequent_backend.certificate_authority AS ca
            WHERE (ca.tenant_id, ca.election_event_id, ca.purpose)
                = (p_tenant, p_event, 'staff-signatures')),
           (SELECT coalesce(jsonb_agg(jsonb_build_object(
                       'url', l.url, 'der_hex', encode(l.der, 'hex'), 'fetched_at', l.last_ok_at)
                       ORDER BY l.url), '[]'::jsonb)
            FROM sequent_backend.staff_crl AS l
            WHERE (l.tenant_id, l.election_event_id) = (p_tenant, p_event)
              AND l.der IS NOT NULL AND l.last_ok_at IS NOT NULL),
           (EXISTS (SELECT FROM sequent_backend.staff_certificate AS x
                    WHERE x.tenant_id = p_tenant AND x.status = 'revoked'
                      AND (x.fingerprint_sha256 = (i.binding).cert_fingerprint
                           OR x.spki_sha256 = (i.binding).cert_spki))
            OR NOT EXISTS (SELECT FROM sequent_backend.staff_certificate AS c
                           WHERE (c.tenant_id, c.election_event_id, c.id)
                               = (p_tenant, p_event, a.certificate_id)
                             AND c.user_id = p_actor AND c.status = 'active'
                             AND c.fingerprint_sha256 = (i.binding).cert_fingerprint
                             AND c.spki_sha256 = (i.binding).cert_spki
                             AND c.pem = a.chain_pem[1]))
    FROM sequent_backend.operational_signing_device_grant AS g
    JOIN sequent_backend.operational_signing_intent AS i ON i.reservation_id = g.reservation_id
    JOIN sequent_backend.operational_signing_official_enrollment AS e
      ON (e.tenant_id, e.actor, e.key_id, e.key_version, e.spki)
       = (i.tenant_id, i.actor, (i.binding).key_id, (i.binding).key_version, (i.binding).cert_spki)
    JOIN sequent_backend.operational_signing_official_certificate AS a
      ON (a.tenant_id, a.actor, a.enrollment_id) = (e.tenant_id, e.actor, e.id)
     AND a.event_id = i.event_id AND a.certificate_fingerprint = (i.binding).cert_fingerprint
    WHERE g.reservation_id = p_fence AND g.challenge_id = p_challenge
      AND (g.tenant_id, g.event_id, g.request_id, g.actor) = (p_tenant, p_event, p_request, p_actor)
      AND g.flow = 'initialize' AND g.device_profile = p_profile AND g.claimed_at IS NULL
      AND (i.tenant_id, i.event_id, i.request_id, i.actor, i.purpose, i.challenge_id)
        = (p_tenant, p_event, p_request, p_actor, 'approval-payload', p_challenge)
      AND i.phase = 'reserved' AND i.reservation_deadline > clock_timestamp()
      AND (i.binding).action = 'initialize-voting'
      AND e.revoked_at IS NULL AND e.nonexportable_confirmed AND e.device_profile = p_profile
      AND a.retired_at IS NULL
      AND sequent_backend.operational_signing_hsm_required(p_tenant);
$$;

-- Device principals: the exact reserved grant of a reference, while it can
-- still be claimed. Never another flow's grant.
CREATE FUNCTION sequent_backend.operational_signing_get_device_grant(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_challenge uuid, p_fence uuid
) RETURNS TABLE (
    binding jsonb, reservation_deadline timestamptz, verified_auth_time bigint,
    verified_issuer text
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT to_jsonb(i.binding), i.reservation_deadline, g.verified_auth_time, g.verified_issuer
    FROM sequent_backend.operational_signing_device_grant AS g
    JOIN sequent_backend.operational_signing_intent AS i ON i.reservation_id = g.reservation_id
    WHERE g.reservation_id = p_fence AND g.challenge_id = p_challenge
      AND (g.tenant_id, g.event_id, g.request_id, g.actor) = (p_tenant, p_event, p_request, p_actor)
      AND g.claimed_at IS NULL AND g.verified_issuer = (i.binding).issuer
      AND (i.tenant_id, i.event_id, i.request_id, i.actor, i.challenge_id)
        = (p_tenant, p_event, p_request, p_actor, p_challenge)
      AND i.phase = 'reserved' AND i.reservation_deadline > clock_timestamp()
      AND sequent_backend.operational_signing_device_caller(g.flow)
      AND sequent_backend.operational_signing_hsm_required(p_tenant);
$$;

-- Initialize device: the one durable claim before the hardware call. The
-- installed device profile, the official's current enrollment and the CA
-- assessment version the device just obtained must all still be current.
CREATE FUNCTION sequent_backend.operational_initialize_claim_current_device(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_challenge uuid,
    p_fence uuid, p_binding_sha256 text, p_profile text, p_ca_version text
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
DECLARE
    b sequent_backend.operational_signing_binding;
    c record;
BEGIN
    IF p_tenant IS NULL OR p_event IS NULL OR p_fence IS NULL
       OR p_binding_sha256 IS NULL OR p_binding_sha256 !~ '^[0-9a-f]{64}$'
       OR p_ca_version IS NULL OR p_ca_version !~ '^[0-9a-f]{64}$'
       OR NOT sequent_backend.operational_signing_device_caller('initialize')
       OR NOT sequent_backend.operational_signing_hsm_required(p_tenant) THEN
        RETURN false;
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'signing-event:' || p_tenant::text || ':' || p_event::text, 0));
    PERFORM FROM sequent_backend.operational_signing_device_grant AS g
    JOIN sequent_backend.operational_initialize_attempt AS t ON t.challenge_id = g.challenge_id
    WHERE g.reservation_id = p_fence AND g.challenge_id = p_challenge
      AND (g.tenant_id, g.event_id, g.request_id, g.actor) = (p_tenant, p_event, p_request, p_actor)
      AND g.flow = 'initialize' AND g.device_profile = p_profile AND g.claimed_at IS NULL
      AND t.step = 'reserved' AND t.reservation_id = p_fence
      AND t.device_profile = p_profile AND t.binding_sha256 = p_binding_sha256
    FOR UPDATE OF g, t;
    IF NOT FOUND THEN RETURN false; END IF;
    SELECT (i.binding).* INTO b
    FROM sequent_backend.operational_signing_intent AS i
    WHERE i.reservation_id = p_fence AND i.challenge_id = p_challenge AND i.phase = 'reserved'
      AND (i.tenant_id, i.event_id, i.request_id, i.actor, i.purpose)
        = (p_tenant, p_event, p_request, p_actor, 'approval-payload');
    IF NOT FOUND OR b.action <> 'initialize-voting'
       OR b.binding_sha256 IS DISTINCT FROM p_binding_sha256 THEN
        RETURN false;
    END IF;
    SELECT * INTO c FROM sequent_backend.operational_initialize_current(
        p_tenant, p_event, p_request, p_actor, b.payload_hash, p_profile);
    IF NOT FOUND
       OR ROW(c.key_id, c.key_version, c.cert_fingerprint, c.cert_spki)
          IS DISTINCT FROM ROW(b.key_id, b.key_version, b.cert_fingerprint, b.cert_spki)
       OR sequent_backend.operational_initialize_ca_version(
              p_tenant, p_event, b.cert_fingerprint, b.cert_spki) IS DISTINCT FROM p_ca_version
       OR NOT sequent_backend.operational_signing_claim_device(b, p_fence) THEN
        RETURN false;
    END IF;
    UPDATE sequent_backend.operational_signing_device_grant AS g
    SET claimed_at = clock_timestamp(), ca_version = p_ca_version
    WHERE g.reservation_id = p_fence AND g.claimed_at IS NULL;
    IF NOT FOUND THEN RAISE EXCEPTION 'operational device grant changed while claiming'; END IF;
    UPDATE sequent_backend.operational_initialize_attempt AS t
    SET step = 'device-claimed', device_claimed_at = clock_timestamp()
    WHERE t.challenge_id = p_challenge AND t.step = 'reserved';
    IF NOT FOUND THEN RAISE EXCEPTION 'operational Initialize attempt changed while claiming'; END IF;
    RETURN true;
END;
$$;

-- Generic device: the one durable claim of a generic grant, against the
-- official's current enrollment and attached certificate. Initialize grants
-- are refused here; they need the profile and CA version checks above.
CREATE FUNCTION sequent_backend.operational_signing_claim_device_from_reference(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_challenge uuid,
    p_fence uuid, p_binding_sha256 text
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
DECLARE
    b sequent_backend.operational_signing_binding;
    v_profile text;
BEGIN
    IF p_tenant IS NULL OR p_event IS NULL OR p_fence IS NULL
       OR p_binding_sha256 IS NULL OR p_binding_sha256 !~ '^[0-9a-f]{64}$'
       OR NOT sequent_backend.operational_signing_device_caller('generic')
       OR NOT sequent_backend.operational_signing_hsm_required(p_tenant) THEN
        RETURN false;
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'signing-event:' || p_tenant::text || ':' || p_event::text, 0));
    SELECT g.device_profile INTO v_profile
    FROM sequent_backend.operational_signing_device_grant AS g
    WHERE g.reservation_id = p_fence AND g.challenge_id = p_challenge
      AND (g.tenant_id, g.event_id, g.request_id, g.actor) = (p_tenant, p_event, p_request, p_actor)
      AND g.flow = 'generic' AND g.claimed_at IS NULL
    FOR UPDATE;
    IF NOT FOUND THEN RETURN false; END IF;
    SELECT (i.binding).* INTO b
    FROM sequent_backend.operational_signing_intent AS i
    WHERE i.reservation_id = p_fence AND i.challenge_id = p_challenge AND i.phase = 'reserved'
      AND (i.tenant_id, i.event_id, i.request_id, i.actor) = (p_tenant, p_event, p_request, p_actor);
    IF NOT FOUND OR b.binding_sha256 IS DISTINCT FROM p_binding_sha256
       OR NOT EXISTS (
           SELECT FROM sequent_backend.operational_signing_official_enrollment AS e
           JOIN sequent_backend.operational_signing_official_certificate AS a
             ON (a.tenant_id, a.actor, a.enrollment_id) = (e.tenant_id, e.actor, e.id)
           JOIN sequent_backend.staff_certificate AS s
             ON (s.tenant_id, s.election_event_id, s.id) = (a.tenant_id, a.event_id, a.certificate_id)
           WHERE (e.tenant_id, e.actor, e.key_id, e.key_version, e.spki)
               = (p_tenant, p_actor, b.key_id, b.key_version, b.cert_spki)
             AND e.revoked_at IS NULL AND e.nonexportable_confirmed
             AND e.device_profile = v_profile
             AND a.event_id = p_event AND a.retired_at IS NULL
             AND a.certificate_fingerprint = b.cert_fingerprint
             AND s.user_id = p_actor AND s.status = 'active'
             AND s.fingerprint_sha256 = b.cert_fingerprint AND s.spki_sha256 = b.cert_spki
             AND s.pem = a.chain_pem[1]
             AND s.not_before <= clock_timestamp() AND s.not_after > clock_timestamp()
       )
       OR EXISTS (SELECT FROM sequent_backend.staff_certificate AS x
                  WHERE x.tenant_id = p_tenant AND x.status = 'revoked'
                    AND (x.fingerprint_sha256 = b.cert_fingerprint OR x.spki_sha256 = b.cert_spki))
       OR NOT sequent_backend.operational_signing_claim_device(b, p_fence) THEN
        RETURN false;
    END IF;
    UPDATE sequent_backend.operational_signing_device_grant AS g
    SET claimed_at = clock_timestamp()
    WHERE g.reservation_id = p_fence AND g.claimed_at IS NULL;
    IF NOT FOUND THEN RAISE EXCEPTION 'operational device grant changed while claiming'; END IF;
    RETURN true;
END;
$$;

-- Device principals: store the receipt of their own claimed reference, once.
-- The device verified the signature; SQL checks only that it is a DER ECDSA
-- P-256 signature shape. Late evidence of a claimed fence is still stored.
CREATE FUNCTION sequent_backend.operational_signing_settle_device_from_reference(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_challenge uuid,
    p_fence uuid, p_binding_sha256 text, p_signature_der bytea
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
DECLARE
    b sequent_backend.operational_signing_binding;
    v_flow sequent_backend.operational_signing_flow;
BEGIN
    IF p_tenant IS NULL OR p_event IS NULL OR p_fence IS NULL
       OR p_binding_sha256 IS NULL OR p_binding_sha256 !~ '^[0-9a-f]{64}$'
       OR p_signature_der IS NULL OR octet_length(p_signature_der) NOT BETWEEN 8 AND 72
       OR get_byte(p_signature_der, 0) <> 48
       OR get_byte(p_signature_der, 1) <> octet_length(p_signature_der) - 2 THEN
        RETURN false;
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'signing-event:' || p_tenant::text || ':' || p_event::text, 0));
    SELECT g.flow INTO v_flow
    FROM sequent_backend.operational_signing_device_grant AS g
    WHERE g.reservation_id = p_fence AND g.challenge_id = p_challenge
      AND (g.tenant_id, g.event_id, g.request_id, g.actor) = (p_tenant, p_event, p_request, p_actor)
      AND g.claimed_at IS NOT NULL AND g.settled_at IS NULL
    FOR UPDATE;
    IF NOT FOUND OR NOT sequent_backend.operational_signing_device_caller(v_flow) THEN
        RETURN false;
    END IF;
    SELECT (i.binding).* INTO b
    FROM sequent_backend.operational_signing_intent AS i
    WHERE i.reservation_id = p_fence AND i.challenge_id = p_challenge AND i.phase = 'invoked'
      AND (i.tenant_id, i.event_id, i.request_id, i.actor) = (p_tenant, p_event, p_request, p_actor);
    IF NOT FOUND OR b.binding_sha256 IS DISTINCT FROM p_binding_sha256
       OR NOT sequent_backend.operational_signing_settle(b, p_fence, p_signature_der) THEN
        RETURN false;
    END IF;
    UPDATE sequent_backend.operational_signing_device_grant AS g
    SET settled_at = clock_timestamp()
    WHERE g.reservation_id = p_fence AND g.settled_at IS NULL;
    IF NOT FOUND THEN RAISE EXCEPTION 'operational device grant changed while settling'; END IF;
    IF v_flow = 'initialize' THEN
        UPDATE sequent_backend.operational_initialize_attempt AS t
        SET step = 'settled', settled_at = clock_timestamp()
        WHERE t.challenge_id = p_challenge AND t.step = 'device-claimed';
        IF NOT FOUND THEN
            RAISE EXCEPTION 'operational Initialize attempt changed while settling';
        END IF;
    END IF;
    RETURN true;
END;
$$;

-- Approval caller: the device profile durably recorded when the device
-- claimed this reservation. NULL for anything that was never claimed.
CREATE FUNCTION sequent_backend.operational_signing_recorded_device_profile(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_reservation uuid
) RETURNS text LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT g.device_profile
    FROM sequent_backend.operational_signing_device_grant AS g
    WHERE g.reservation_id = p_reservation
      AND (g.tenant_id, g.event_id, g.request_id, g.actor) = (p_tenant, p_event, p_request, p_actor)
      AND g.claimed_at IS NOT NULL;
$$;

REVOKE ALL ON FUNCTION
    sequent_backend.operational_signing_device_caller(sequent_backend.operational_signing_flow),
    sequent_backend.operational_initialize_ca_version(uuid, uuid, text, text),
    sequent_backend.operational_initialize_get_ca_enrollment(uuid, uuid, uuid, text, uuid, uuid, text),
    sequent_backend.operational_signing_get_device_grant(uuid, uuid, uuid, text, uuid, uuid),
    sequent_backend.operational_initialize_claim_current_device(uuid, uuid, uuid, text, uuid, uuid, text, text, text),
    sequent_backend.operational_signing_claim_device_from_reference(uuid, uuid, uuid, text, uuid, uuid, text),
    sequent_backend.operational_signing_settle_device_from_reference(uuid, uuid, uuid, text, uuid, uuid, text, bytea),
    sequent_backend.operational_signing_recorded_device_profile(uuid, uuid, uuid, text, uuid)
    FROM PUBLIC;

ALTER FUNCTION sequent_backend.operational_signing_device_caller(sequent_backend.operational_signing_flow)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_initialize_ca_version(uuid, uuid, text, text)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_initialize_get_ca_enrollment(uuid, uuid, uuid, text, uuid, uuid, text)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_signing_get_device_grant(uuid, uuid, uuid, text, uuid, uuid)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_initialize_claim_current_device(uuid, uuid, uuid, text, uuid, uuid, text, text, text)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_signing_claim_device_from_reference(uuid, uuid, uuid, text, uuid, uuid, text)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_signing_settle_device_from_reference(uuid, uuid, uuid, text, uuid, uuid, text, bytea)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_signing_recorded_device_profile(uuid, uuid, uuid, text, uuid)
    OWNER TO sequent_os_ledger_api_owner;

GRANT EXECUTE ON FUNCTION
    sequent_backend.operational_initialize_get_ca_enrollment(uuid, uuid, uuid, text, uuid, uuid, text),
    sequent_backend.operational_initialize_ca_version(uuid, uuid, text, text)
    TO sequent_os_initialize_ca;
GRANT EXECUTE ON FUNCTION
    sequent_backend.operational_signing_get_device_grant(uuid, uuid, uuid, text, uuid, uuid),
    sequent_backend.operational_signing_settle_device_from_reference(uuid, uuid, uuid, text, uuid, uuid, text, bytea)
    TO sequent_os_initialize_device, sequent_os_device;
GRANT EXECUTE ON FUNCTION
    sequent_backend.operational_initialize_claim_current_device(uuid, uuid, uuid, text, uuid, uuid, text, text, text)
    TO sequent_os_initialize_device;
GRANT EXECUTE ON FUNCTION
    sequent_backend.operational_signing_claim_device_from_reference(uuid, uuid, uuid, text, uuid, uuid, text)
    TO sequent_os_device;
GRANT EXECUTE ON FUNCTION
    sequent_backend.operational_signing_recorded_device_profile(uuid, uuid, uuid, text, uuid)
    TO sequent_os_approve;

REVOKE CREATE ON SCHEMA sequent_backend FROM sequent_os_ledger_api_owner;
