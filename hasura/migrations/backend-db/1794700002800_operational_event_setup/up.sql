-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Genuine device setup may serve HSM events within a mixed tenant. An
-- explicitly installed YubiHSM profile is still mandatory. Keep the
-- sealed test profile on its original tenant policy boundary.
GRANT CREATE ON SCHEMA sequent_backend TO sequent_os_ledger_api_owner;
CREATE FUNCTION sequent_backend.operational_event_setup_allowed(
    p_tenant uuid, p_event uuid
) RETURNS boolean LANGUAGE sql STABLE SECURITY INVOKER
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT coalesce((
        SELECT CASE i.device_profile
            WHEN 'yubihsm' THEN
                sequent_backend.operational_signing_event_hsm_required(p_tenant, p_event)
            WHEN 'softhsm-test' THEN
                sequent_backend.operational_signing_hsm_required(p_tenant)
            ELSE false
        END
        FROM sequent_backend.operational_setup_installation AS i
        WHERE i.tenant_id = p_tenant
    ), false);
$$;
REVOKE ALL ON FUNCTION sequent_backend.operational_event_setup_allowed(uuid, uuid)
    FROM PUBLIC;
ALTER FUNCTION sequent_backend.operational_event_setup_allowed(uuid, uuid)
    OWNER TO sequent_os_ledger_api_owner;
REVOKE CREATE ON SCHEMA sequent_backend FROM sequent_os_ledger_api_owner;

CREATE OR REPLACE FUNCTION sequent_backend.operational_setup_guard()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
BEGIN
    IF TG_TABLE_NAME = 'operational_setup_installation' THEN
        -- Provisioning is an operator-only INSERT. YubiHSM may be
        -- installed for a mixed tenant; its concrete events are checked on
        -- every operation. SoftHSM never gains that production route.
        IF TG_OP = 'INSERT' THEN
            IF NEW.device_profile = 'yubihsm'
               OR sequent_backend.operational_signing_hsm_required(NEW.tenant_id) THEN
                RETURN NEW;
            END IF;
        END IF;
        RAISE EXCEPTION 'operational setup installation is not authorized or cannot change';
    END IF;
    IF ROW(NEW.attempt, NEW.tenant_id, NEW.event_id, NEW.official, NEW.prepared_by,
           NEW.device_profile, NEW.prepared_at, NEW.claim_deadline)
       IS DISTINCT FROM
       ROW(OLD.attempt, OLD.tenant_id, OLD.event_id, OLD.official, OLD.prepared_by,
           OLD.device_profile, OLD.prepared_at, OLD.claim_deadline)
       OR NOT (
           (OLD.step = 'prepared' AND NEW.step = 'claimed')
           OR (ROW(NEW.fence, NEW.claimed_at, NEW.device_deadline)
                   IS NOT DISTINCT FROM ROW(OLD.fence, OLD.claimed_at, OLD.device_deadline)
               AND (
                   (OLD.step = 'claimed' AND NEW.step = 'completed')
                   OR (OLD.step = 'completed' AND NEW.step = 'attached'
                       AND ROW(NEW.completed_at, NEW.spki_sha256, NEW.evidence_sha256,
                               NEW.public_csr_pem, NEW.enrollment_id)
                           IS NOT DISTINCT FROM
                           ROW(OLD.completed_at, OLD.spki_sha256, OLD.evidence_sha256,
                               OLD.public_csr_pem, OLD.enrollment_id))
               ))
       ) THEN
        RAISE EXCEPTION 'invalid operational setup generation update'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION sequent_backend.operational_setup_management_status(
    p_tenant uuid, p_event uuid, p_official uuid, p_officer uuid
) RETURNS TABLE (phase text, attempt uuid)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
DECLARE
    g sequent_backend.operational_setup_generation%ROWTYPE;
    v_now timestamptz := clock_timestamp();
    v_live boolean;
BEGIN
    IF p_tenant IS NULL OR p_event IS NULL OR p_official IS NULL OR p_officer IS NULL
       OR p_official = p_officer
       OR '00000000-0000-0000-0000-000000000000' IN (p_tenant, p_event, p_official, p_officer)
       OR NOT EXISTS (SELECT FROM sequent_backend.election_event AS ev
                      WHERE (ev.tenant_id, ev.id) = (p_tenant, p_event)) THEN
        RETURN;
    END IF;
    IF NOT sequent_backend.operational_event_setup_allowed(p_tenant, p_event)
       OR NOT EXISTS (SELECT FROM sequent_backend.operational_setup_installation AS s
                      WHERE s.tenant_id = p_tenant) THEN
        RETURN QUERY SELECT 'unavailable'::text, NULL::uuid;
        RETURN;
    END IF;
    SELECT * INTO g FROM sequent_backend.operational_setup_generation AS s
    WHERE (s.tenant_id, s.official) = (p_tenant, p_official);
    IF NOT FOUND THEN
        -- A key enrolled outside this ledger is not managed from here.
        RETURN QUERY SELECT CASE WHEN EXISTS (
            SELECT FROM sequent_backend.operational_signing_official_enrollment AS e
            WHERE (e.tenant_id, e.actor) = (p_tenant, p_official::text)
        ) THEN 'unavailable' ELSE 'not-started' END, NULL::uuid;
        RETURN;
    END IF;
    IF g.event_id <> p_event THEN
        RETURN QUERY SELECT 'unavailable'::text, NULL::uuid;
        RETURN;
    END IF;
    IF g.step = 'prepared' THEN
        RETURN QUERY SELECT CASE WHEN v_now < g.claim_deadline THEN 'pending' ELSE 'reconcile' END,
                            g.attempt;
        RETURN;
    ELSIF g.step = 'claimed' THEN
        RETURN QUERY SELECT CASE WHEN v_now < g.device_deadline THEN 'pending' ELSE 'reconcile' END,
                            g.attempt;
        RETURN;
    END IF;
    v_live := EXISTS (
        SELECT FROM sequent_backend.operational_signing_official_enrollment AS e
        WHERE (e.tenant_id, e.actor, e.id) = (p_tenant, p_official::text, g.enrollment_id)
          AND e.spki = g.spki_sha256 AND e.revoked_at IS NULL
    );
    IF g.step = 'completed' THEN
        RETURN QUERY SELECT CASE WHEN v_live THEN 'certificate-request-issued'
                                 ELSE 'certificate-needs-review' END, g.attempt;
        RETURN;
    END IF;
    RETURN QUERY SELECT CASE WHEN v_live AND EXISTS (
        SELECT FROM sequent_backend.operational_signing_official_certificate AS a
        JOIN sequent_backend.staff_certificate AS c
          ON (c.tenant_id, c.election_event_id, c.id) = (a.tenant_id, a.event_id, a.certificate_id)
        WHERE (a.tenant_id, a.event_id, a.actor, a.enrollment_id, a.certificate_id)
            = (p_tenant, p_event, p_official::text, g.enrollment_id, g.certificate_id)
          AND a.retired_at IS NULL AND a.certificate_fingerprint = g.certificate_fingerprint
          AND c.status = 'active' AND c.fingerprint_sha256 = g.certificate_fingerprint
          AND c.spki_sha256 = g.spki_sha256
          AND c.not_before <= v_now AND c.not_after > v_now
    ) THEN 'certificate-attached' ELSE 'certificate-needs-review' END, g.attempt;
END;
$$;

CREATE OR REPLACE FUNCTION sequent_backend.operational_setup_prepare_generation(
    p_tenant uuid, p_event uuid, p_official uuid, p_officer uuid, p_attempt uuid
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
DECLARE
    v_profile text;
    v_now timestamptz;
BEGIN
    -- Serialize provider mode transitions before actor locks or writes.
    PERFORM pg_catalog.pg_advisory_xact_lock_shared(pg_catalog.hashtextextended(
        'operational-signing-policy:deployment', 0));
    IF p_tenant IS NULL OR p_event IS NULL OR p_official IS NULL OR p_officer IS NULL
       OR p_attempt IS NULL OR p_official = p_officer
       OR '00000000-0000-0000-0000-000000000000'
          IN (p_tenant, p_event, p_official, p_officer, p_attempt)
       OR NOT sequent_backend.operational_signing_hsm_required(p_tenant) THEN
        RETURN false;
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'operational-signing-official:' || p_tenant::text || ':' || p_official::text, 0));
    SELECT s.device_profile INTO v_profile
    FROM sequent_backend.operational_setup_installation AS s WHERE s.tenant_id = p_tenant;
    IF NOT FOUND
       OR NOT EXISTS (SELECT FROM sequent_backend.election_event AS ev
                      WHERE (ev.tenant_id, ev.id) = (p_tenant, p_event))
       OR EXISTS (SELECT FROM sequent_backend.operational_signing_official_enrollment AS e
                  WHERE (e.tenant_id, e.actor) = (p_tenant, p_official::text)) THEN
        RETURN false;
    END IF;
    v_now := clock_timestamp();
    INSERT INTO sequent_backend.operational_setup_generation
        (attempt, tenant_id, event_id, official, prepared_by, device_profile, prepared_at,
         claim_deadline)
    VALUES (p_attempt, p_tenant, p_event, p_official, p_officer, v_profile, v_now,
            v_now + interval '2 minutes')
    ON CONFLICT DO NOTHING;
    RETURN FOUND;
END;
$$;

CREATE OR REPLACE FUNCTION sequent_backend.operational_setup_claim_generation(
    p_tenant uuid, p_event uuid, p_official uuid, p_attempt uuid
) RETURNS TABLE (fence uuid, deadline timestamptz)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
DECLARE
    v_now timestamptz;
BEGIN
    -- Serialize provider mode transitions before actor locks or writes.
    PERFORM pg_catalog.pg_advisory_xact_lock_shared(pg_catalog.hashtextextended(
        'operational-signing-policy:deployment', 0));
    IF p_tenant IS NULL OR p_official IS NULL
       OR NOT sequent_backend.operational_event_setup_allowed(p_tenant, p_event) THEN
        RETURN;
    END IF;
    -- An ordinary actor ticket is reauthorized before the device touches the
    -- key. The pre-existing independent-officer and sealed demo pathways keep
    -- their original contract when there is no ordinary ticket.
    IF EXISTS (
        SELECT FROM sequent_backend.operational_event_setup_request s WHERE s.attempt = p_attempt
    ) THEN
        IF current_setting('transaction_isolation') <> 'read committed' THEN RETURN; END IF;
        PERFORM pg_advisory_xact_lock(hashtextextended('signing-event:'||p_tenant::text||':'||p_event::text,0));
        PERFORM FROM sequent_backend.operational_event_setup_request s
        JOIN sequent_backend.signing_request r
          ON (r.tenant_id,r.election_event_id,r.id,r.payload_sha256)
           = (s.tenant_id,s.event_id,s.signing_request_id,s.payload_sha256)
        WHERE (s.tenant_id,s.event_id,s.official,s.attempt)=(p_tenant,p_event,p_official,p_attempt)
          AND s.eligible_until > clock_timestamp()
          AND r.election_id IS NOT DISTINCT FROM s.post_id AND r.status = 'waiting'
          AND (r.expires_at IS NULL OR r.expires_at > clock_timestamp())
          AND encode(sha256(convert_to(r.canonical_payload,'UTF8')),'hex') = s.payload_sha256
        FOR UPDATE OF r,s;
        IF NOT FOUND THEN RETURN; END IF;
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'operational-signing-official:' || p_tenant::text || ':' || p_official::text, 0));
    v_now := clock_timestamp();
    RETURN QUERY
    UPDATE sequent_backend.operational_setup_generation AS s
    SET step = 'claimed', fence = gen_random_uuid(), claimed_at = v_now,
        device_deadline = v_now + interval '60 seconds'
    WHERE (s.attempt, s.tenant_id, s.event_id, s.official)
        = (p_attempt, p_tenant, p_event, p_official)
      AND s.step = 'prepared' AND v_now < s.claim_deadline
      AND s.device_profile = (SELECT i.device_profile
                              FROM sequent_backend.operational_setup_installation AS i
                              WHERE i.tenant_id = p_tenant)
      AND NOT EXISTS (SELECT FROM sequent_backend.operational_signing_official_enrollment AS e
                      WHERE (e.tenant_id, e.actor) = (p_tenant, p_official::text))
    RETURNING s.fence, s.device_deadline;
END;
$$;

CREATE OR REPLACE FUNCTION sequent_backend.operational_setup_complete_generation(
    p_tenant uuid, p_event uuid, p_official uuid, p_attempt uuid, p_fence uuid,
    p_spki_sha256 text, p_evidence_sha256 text, p_public_csr_pem text
) RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
DECLARE
    g sequent_backend.operational_setup_generation%ROWTYPE;
    v_enrollment uuid;
BEGIN
    -- Serialize provider mode transitions before actor locks or writes.
    PERFORM pg_catalog.pg_advisory_xact_lock_shared(pg_catalog.hashtextextended(
        'operational-signing-policy:deployment', 0));
    IF p_tenant IS NULL OR p_official IS NULL OR p_fence IS NULL
       OR p_spki_sha256 IS NULL OR p_spki_sha256 !~ '^[0-9a-f]{64}$'
       OR p_evidence_sha256 IS NULL OR p_evidence_sha256 !~ '^[0-9a-f]{64}$'
       OR p_public_csr_pem IS NULL OR octet_length(p_public_csr_pem) NOT BETWEEN 64 AND 16384
       OR p_public_csr_pem !~ '^-----BEGIN CERTIFICATE REQUEST-----\r?\n[A-Za-z0-9+/=\r\n]+-----END CERTIFICATE REQUEST-----\r?\n?$'
       OR NOT sequent_backend.operational_event_setup_allowed(p_tenant, p_event) THEN
        RETURN NULL;
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'operational-signing-official:' || p_tenant::text || ':' || p_official::text, 0));
    SELECT * INTO g FROM sequent_backend.operational_setup_generation AS s
    WHERE (s.attempt, s.tenant_id, s.event_id, s.official, s.fence)
        = (p_attempt, p_tenant, p_event, p_official, p_fence)
      AND s.step = 'claimed'
    FOR UPDATE;
    IF NOT FOUND THEN RETURN NULL; END IF;
    v_enrollment := sequent_backend.operational_signing_enroll_official(
        p_tenant, p_official::text, 'official/' || p_tenant::text || '/' || p_official::text,
        1, p_spki_sha256, g.device_profile, true, p_evidence_sha256);
    IF v_enrollment IS NULL THEN RETURN NULL; END IF;
    UPDATE sequent_backend.operational_setup_generation AS s
    SET step = 'completed', completed_at = clock_timestamp(), spki_sha256 = p_spki_sha256,
        evidence_sha256 = p_evidence_sha256, public_csr_pem = p_public_csr_pem,
        enrollment_id = v_enrollment
    WHERE s.attempt = g.attempt AND s.step = 'claimed';
    RETURN v_enrollment;
END;
$$;

CREATE OR REPLACE FUNCTION sequent_backend.operational_setup_public_csr(
    p_tenant uuid, p_event uuid, p_official uuid, p_officer uuid, p_attempt uuid
) RETURNS text LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT s.public_csr_pem
    FROM sequent_backend.operational_setup_generation AS s
    JOIN sequent_backend.operational_signing_official_enrollment AS e
      ON (e.tenant_id, e.actor, e.id) = (s.tenant_id, s.official::text, s.enrollment_id)
    WHERE (s.attempt, s.tenant_id, s.event_id, s.official)
        = (p_attempt, p_tenant, p_event, p_official)
      AND p_officer IS NOT NULL AND p_officer <> p_official
      AND p_officer <> '00000000-0000-0000-0000-000000000000'
      AND s.step = 'completed' AND e.spki = s.spki_sha256 AND e.revoked_at IS NULL
      AND sequent_backend.operational_event_setup_allowed(p_tenant, p_event);
$$;

CREATE OR REPLACE FUNCTION sequent_backend.operational_setup_pin_for_attachment(
    p_tenant uuid, p_event uuid, p_official uuid, p_authority uuid, p_attempt uuid
) RETURNS TABLE (enrollment_id uuid, expected_spki text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
BEGIN
    -- Serialize provider mode transitions before actor locks or writes.
    PERFORM pg_catalog.pg_advisory_xact_lock_shared(pg_catalog.hashtextextended(
        'operational-signing-policy:deployment', 0));
    IF p_tenant IS NULL OR p_official IS NULL OR p_authority IS NULL
       OR p_authority = p_official OR p_authority = '00000000-0000-0000-0000-000000000000'
       OR NOT sequent_backend.operational_event_setup_allowed(p_tenant, p_event) THEN
        RETURN;
    END IF;
    -- An eventual attach in this transaction takes the signing event first.
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'signing-event:' || p_tenant::text || ':' || p_event::text, 0));
    PERFORM pg_advisory_xact_lock_shared(hashtextextended(
        'operational-signing-official:' || p_tenant::text || ':' || p_official::text, 0));
    RETURN QUERY
    SELECT e.id, e.spki
    FROM sequent_backend.operational_setup_generation AS s
    JOIN sequent_backend.operational_signing_official_enrollment AS e
      ON (e.tenant_id, e.actor, e.id) = (s.tenant_id, s.official::text, s.enrollment_id)
    WHERE (s.attempt, s.tenant_id, s.event_id, s.official)
        = (p_attempt, p_tenant, p_event, p_official)
      AND s.step = 'completed' AND e.spki = s.spki_sha256 AND e.revoked_at IS NULL
      AND e.nonexportable_confirmed AND e.device_profile = s.device_profile;
END;
$$;

CREATE OR REPLACE FUNCTION sequent_backend.operational_setup_attach_certificate(
    p_tenant uuid, p_event uuid, p_official uuid, p_authority uuid, p_attempt uuid,
    p_enrollment uuid, p_certificate uuid, p_fingerprint text, p_chain text[]
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
DECLARE
    g sequent_backend.operational_setup_generation%ROWTYPE;
BEGIN
    -- Serialize provider mode transitions before actor locks or writes.
    PERFORM pg_catalog.pg_advisory_xact_lock_shared(pg_catalog.hashtextextended(
        'operational-signing-policy:deployment', 0));
    IF p_tenant IS NULL OR p_event IS NULL OR p_official IS NULL OR p_authority IS NULL
       OR p_authority = p_official OR p_authority = '00000000-0000-0000-0000-000000000000'
       OR p_enrollment IS NULL OR p_certificate IS NULL
       OR p_fingerprint IS NULL OR p_fingerprint !~ '^[0-9a-f]{64}$'
       OR NOT sequent_backend.operational_event_setup_allowed(p_tenant, p_event) THEN
        RETURN false;
    END IF;
    -- Event then official: the order of the binding function called below.
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'signing-event:' || p_tenant::text || ':' || p_event::text, 0));
    SELECT * INTO g FROM sequent_backend.operational_setup_generation AS s
    WHERE (s.attempt, s.tenant_id, s.event_id, s.official, s.enrollment_id)
        = (p_attempt, p_tenant, p_event, p_official, p_enrollment)
      AND s.step = 'completed'
    FOR UPDATE;
    IF NOT FOUND
       OR NOT EXISTS (
           SELECT FROM sequent_backend.staff_certificate AS c
           WHERE (c.tenant_id, c.election_event_id, c.id) = (p_tenant, p_event, p_certificate)
             AND c.user_id = p_official::text AND c.status = 'active'
             AND c.fingerprint_sha256 = p_fingerprint AND c.spki_sha256 = g.spki_sha256
             AND c.registration = 'security-officer' AND c.registered_by = p_authority::text
             AND NOT EXISTS (
                SELECT FROM sequent_backend.operational_event_setup_request AS ticket
                WHERE ticket.attempt=g.attempt AND ticket.post_id IS DISTINCT FROM c.election_id
             )
       )
       OR NOT sequent_backend.operational_signing_bind_official_certificate(
           p_tenant, p_event, p_official::text, p_enrollment, p_certificate, p_fingerprint, p_chain) THEN
        RETURN false;
    END IF;
    UPDATE sequent_backend.operational_setup_generation AS s
    SET step = 'attached', attached_at = clock_timestamp(), attached_by = p_authority,
        certificate_id = p_certificate, certificate_fingerprint = p_fingerprint
    WHERE s.attempt = g.attempt AND s.step = 'completed';
    RETURN FOUND;
END;
$$;

CREATE OR REPLACE FUNCTION sequent_backend.operational_official_setup_status(
    p_tenant uuid, p_event uuid, p_actor text
) RETURNS text LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT CASE
        WHEN NOT EXISTS (
            SELECT FROM sequent_backend.operational_signing_official_enrollment AS e
            WHERE (e.tenant_id, e.actor) = (p_tenant, p_actor) AND e.revoked_at IS NULL
        ) THEN 'key-not-enrolled'
        WHEN NOT EXISTS (
            SELECT FROM sequent_backend.operational_signing_official_enrollment AS e
            JOIN sequent_backend.operational_signing_official_certificate AS a
              ON (a.tenant_id, a.actor, a.enrollment_id) = (e.tenant_id, e.actor, e.id)
            WHERE (e.tenant_id, e.actor) = (p_tenant, p_actor) AND e.revoked_at IS NULL
              AND a.event_id = p_event
        ) THEN 'certificate-not-attached'
        WHEN EXISTS (
            SELECT FROM sequent_backend.operational_signing_official_enrollment AS e
            JOIN sequent_backend.operational_signing_official_certificate AS a
              ON (a.tenant_id, a.actor, a.enrollment_id) = (e.tenant_id, e.actor, e.id)
            JOIN sequent_backend.staff_certificate AS c
              ON (c.tenant_id, c.election_event_id, c.id) = (a.tenant_id, a.event_id, a.certificate_id)
            WHERE (e.tenant_id, e.actor) = (p_tenant, p_actor) AND e.revoked_at IS NULL
              AND a.event_id = p_event AND a.retired_at IS NULL
              AND c.user_id = p_actor AND c.status = 'active'
              AND c.fingerprint_sha256 = a.certificate_fingerprint AND c.spki_sha256 = e.spki
              AND c.not_before <= clock_timestamp() AND c.not_after > clock_timestamp()
        ) THEN 'certificate-on-file'
        ELSE 'certificate-needs-review'
    END
    WHERE p_actor IS NOT NULL AND btrim(p_actor) <> ''
      AND (
          sequent_backend.operational_signing_hsm_required(p_tenant)
          OR sequent_backend.operational_signing_event_hsm_required(p_tenant,p_event)
      )
      AND EXISTS (SELECT FROM sequent_backend.election_event AS ev
                  WHERE (ev.tenant_id, ev.id) = (p_tenant, p_event));
$$;

-- An official's immutable YubiHSM key belongs to the tenant/account, not to
-- the first event. A different HSM event still needs its own independently
-- registered, CA-verified staff certificate bound to that event and Post.
GRANT CREATE ON SCHEMA sequent_backend TO sequent_os_ledger_api_owner;
CREATE FUNCTION sequent_backend.operational_setup_attach_existing_event_certificate(
    p_tenant uuid, p_event uuid, p_post uuid, p_official uuid,
    p_authority uuid, p_enrollment uuid, p_certificate uuid, p_fingerprint text,
    p_chain text[], p_ticket uuid
) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
BEGIN
    IF p_tenant IS NULL OR p_event IS NULL OR p_ticket IS NULL
       OR p_official IS NULL OR p_authority IS NULL OR p_official = p_authority
       OR p_enrollment IS NULL OR p_certificate IS NULL OR p_fingerprint IS NULL
       OR p_fingerprint !~ '^[0-9a-f]{64}$'
       OR '00000000-0000-0000-0000-000000000000' IN
          (p_tenant,p_event,p_post,p_official,p_authority,p_enrollment,p_certificate,p_ticket)
       OR current_setting('transaction_isolation') <> 'read committed' THEN
        RETURN false;
    END IF;
    PERFORM pg_advisory_xact_lock_shared(hashtextextended(
        'operational-signing-policy:deployment', 0));
    IF NOT sequent_backend.operational_signing_event_hsm_required(p_tenant,p_event)
       OR NOT EXISTS (
           SELECT FROM sequent_backend.operational_setup_installation i
           WHERE i.tenant_id = p_tenant AND i.device_profile = 'yubihsm'
       ) THEN RETURN false; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'signing-event:' || p_tenant::text || ':' || p_event::text, 0));
    -- Lock the live actor ticket and current request before the official;
    -- the inner binder reacquires the same event and shared actor locks.
    PERFORM FROM sequent_backend.operational_event_setup_request s
    JOIN sequent_backend.signing_request r
      ON (s.tenant_id,s.event_id,s.signing_request_id,s.payload_sha256) =
         (r.tenant_id,r.election_event_id,r.id,r.payload_sha256)
    WHERE (s.id,s.tenant_id,s.event_id,s.official)=(p_ticket,p_tenant,p_event,p_official)
      AND s.post_id IS NOT DISTINCT FROM p_post
      AND r.election_id IS NOT DISTINCT FROM p_post
      AND s.attempt IS NULL AND s.superseded_at IS NULL AND s.attached_certificate IS NULL
      AND s.eligible_until > clock_timestamp()
      AND r.status='waiting' AND (r.expires_at IS NULL OR r.expires_at > clock_timestamp())
      AND encode(sha256(convert_to(r.canonical_payload,'UTF8')),'hex') = s.payload_sha256
    FOR UPDATE OF r,s;
    IF NOT FOUND THEN RETURN false; END IF;
    PERFORM pg_advisory_xact_lock_shared(hashtextextended(
        'operational-signing-official:' || p_tenant::text || ':' || p_official::text, 0));
    IF NOT EXISTS (
        SELECT FROM sequent_backend.operational_signing_official_enrollment k
        JOIN sequent_backend.operational_setup_generation g
          ON (g.tenant_id,g.official,g.enrollment_id,g.spki_sha256)
           = (k.tenant_id,p_official,k.id,k.spki)
        JOIN sequent_backend.staff_certificate c
          ON (c.tenant_id,c.election_event_id,c.user_id,c.id,c.spki_sha256)
           = (k.tenant_id,p_event,k.actor,p_certificate,k.spki)
        WHERE (k.tenant_id,k.actor,k.id) = (p_tenant,p_official::text,p_enrollment)
          AND k.key_id = 'official/' || p_tenant::text || '/' || p_official::text
          AND k.revoked_at IS NULL AND k.nonexportable_confirmed
          AND k.device_profile = 'yubihsm' AND g.device_profile = k.device_profile
          AND g.step IN ('completed','attached')
          AND c.registration = 'security-officer' AND c.registered_by = p_authority::text
          AND c.election_id IS NOT DISTINCT FROM p_post AND c.fingerprint_sha256 = p_fingerprint
          AND c.status='active' AND c.not_before <= clock_timestamp() AND c.not_after > clock_timestamp()
    ) THEN RETURN false; END IF;
    -- The pre-existing private authority validates CA chain and revocation;
    -- the binder independently checks leaf identity, live enrollment,
    -- registration validity and uniqueness in the same transaction.
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'operational-enrollment-fingerprint:' || p_tenant::text || ':' || p_fingerprint, 0));
    IF EXISTS (
        SELECT FROM sequent_backend.operational_signing_official_certificate c
        WHERE c.tenant_id=p_tenant AND c.certificate_fingerprint=p_fingerprint
          AND (c.event_id,c.certificate_id)<>(p_event,p_certificate)
    ) THEN RETURN false; END IF;
    IF NOT sequent_backend.operational_signing_bind_official_certificate(
        p_tenant,p_event,p_official::text,p_enrollment,p_certificate,p_fingerprint,p_chain
    ) THEN RETURN false; END IF;
    -- If an earlier request in this same event closed while awaiting CA
    -- issuance, the replacement ticket completes the original immutable
    -- generation record as well as the replacement attachment ledger.
    UPDATE sequent_backend.operational_setup_generation g
    SET step='attached', attached_by=p_authority, attached_at=clock_timestamp(),
        certificate_id=p_certificate, certificate_fingerprint=p_fingerprint
    WHERE (g.tenant_id,g.event_id,g.official,g.enrollment_id)=
          (p_tenant,p_event,p_official,p_enrollment) AND g.step='completed';
    UPDATE sequent_backend.operational_event_setup_request
    SET attached_certificate=p_certificate, attached_by=p_authority,
        attached_at=clock_timestamp()
    WHERE id=p_ticket AND attached_certificate IS NULL;
    RETURN FOUND;
END;
$$;
REVOKE ALL ON FUNCTION sequent_backend.operational_setup_attach_existing_event_certificate(
    uuid, uuid, uuid, uuid, uuid, uuid, uuid, text, text[], uuid
) FROM PUBLIC;
ALTER FUNCTION sequent_backend.operational_setup_attach_existing_event_certificate(
    uuid, uuid, uuid, uuid, uuid, uuid, uuid, text, text[], uuid
) OWNER TO sequent_os_ledger_api_owner;
GRANT EXECUTE ON FUNCTION sequent_backend.operational_setup_attach_existing_event_certificate(
    uuid, uuid, uuid, uuid, uuid, uuid, uuid, text, text[], uuid
) TO sequent_os_setup_authority;
REVOKE CREATE ON SCHEMA sequent_backend FROM sequent_os_ledger_api_owner;

-- This actor-bound request is only a request for setup. It is never a
-- hardware claim, CA decision, device grant, receipt, approval, or vote.
GRANT CREATE ON SCHEMA sequent_backend TO sequent_os_ledger_data_owner, sequent_os_ledger_api_owner;
CREATE TABLE sequent_backend.operational_event_setup_request (
    id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL,
    event_id uuid NOT NULL,
    signing_request_id uuid NOT NULL,
    official uuid NOT NULL,
    post_id uuid,
    payload_sha256 text NOT NULL CHECK (payload_sha256 ~ '^[0-9a-f]{64}$'),
    auth_time timestamptz NOT NULL,
    eligible_until timestamptz NOT NULL,
    attempt uuid UNIQUE REFERENCES sequent_backend.operational_setup_generation(attempt),
    superseded_at timestamptz,
    attached_certificate uuid,
    attached_by uuid,
    attached_at timestamptz,
    dispatch_officer uuid,
    dispatch_auth_time timestamptz,
    dispatched_at timestamptz,
    CHECK ((dispatch_officer IS NULL) = (dispatch_auth_time IS NULL)),
    CHECK ((dispatch_officer IS NULL) = (dispatched_at IS NULL)),
    CHECK (dispatch_officer IS NULL OR (dispatch_officer <> official AND attempt IS NOT NULL)),
    CHECK ((attached_certificate IS NULL) = (attached_by IS NULL)),
    CHECK ((attached_certificate IS NULL) = (attached_at IS NULL)),
    CHECK (attached_by IS NULL OR attached_by <> official),
    CHECK (attached_certificate IS NULL OR attempt IS NULL),
    FOREIGN KEY (tenant_id,event_id) REFERENCES sequent_backend.election_event(tenant_id,id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX operational_event_setup_request_one_current
    ON sequent_backend.operational_event_setup_request(tenant_id,event_id,signing_request_id,official)
    WHERE superseded_at IS NULL;
REVOKE ALL ON TABLE sequent_backend.operational_event_setup_request FROM PUBLIC;
ALTER TABLE sequent_backend.operational_event_setup_request OWNER TO sequent_os_ledger_data_owner;
GRANT SELECT, INSERT ON TABLE sequent_backend.operational_event_setup_request TO sequent_os_ledger_api_owner;
GRANT UPDATE (attempt, superseded_at, attached_certificate, attached_by, attached_at,
    dispatch_officer, dispatch_auth_time, dispatched_at)
    ON TABLE sequent_backend.operational_event_setup_request TO sequent_os_ledger_api_owner;

CREATE FUNCTION sequent_backend.operational_request_event_setup(
    p_tenant uuid, p_event uuid, p_request uuid, p_official uuid, p_post uuid,
    p_hash text, p_auth_time timestamptz, p_ticket uuid
) RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
DECLARE v_now timestamptz; v_ticket uuid;
BEGIN
    IF p_tenant IS NULL OR p_event IS NULL OR p_request IS NULL
       OR p_official IS NULL OR p_ticket IS NULL
       OR p_hash IS NULL OR p_hash !~ '^[0-9a-f]{64}$'
       OR p_auth_time IS NULL OR NOT isfinite(p_auth_time)
       OR '00000000-0000-0000-0000-000000000000' IN
          (p_tenant,p_event,p_request,p_official,p_ticket,p_post)
       OR current_setting('transaction_isolation') <> 'read committed' THEN RETURN NULL; END IF;
    PERFORM pg_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment',0));
    IF NOT sequent_backend.operational_signing_event_hsm_required(p_tenant,p_event)
       OR NOT EXISTS (
           SELECT FROM sequent_backend.operational_setup_installation
           WHERE tenant_id = p_tenant AND device_profile = 'yubihsm'
       ) THEN RETURN NULL; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended('signing-event:'||p_tenant::text||':'||p_event::text,0));
    v_now := clock_timestamp();
    IF p_auth_time > v_now OR p_auth_time <= v_now - interval '60 seconds' THEN RETURN NULL; END IF;
    PERFORM FROM sequent_backend.signing_request r
    WHERE (r.tenant_id,r.election_event_id,r.id,r.payload_sha256)=(p_tenant,p_event,p_request,p_hash)
      AND r.election_id IS NOT DISTINCT FROM p_post AND r.status = 'waiting'
      AND (r.expires_at IS NULL OR r.expires_at > v_now)
      AND encode(sha256(convert_to(r.canonical_payload,'UTF8')),'hex') = p_hash
    FOR UPDATE;
    IF NOT FOUND OR p_auth_time <= clock_timestamp() - interval '60 seconds'
       THEN RETURN NULL; END IF;
    -- Keep audited ids immutable. An expired, unclaimed ticket may be
    -- superseded by freshly authenticated intent while preserving history.
    UPDATE sequent_backend.operational_event_setup_request s
    SET superseded_at = clock_timestamp()
    WHERE (s.tenant_id,s.event_id,s.signing_request_id,s.official) =
          (p_tenant,p_event,p_request,p_official)
      AND s.superseded_at IS NULL AND s.attempt IS NULL AND s.attached_certificate IS NULL
      AND s.eligible_until <= clock_timestamp() AND s.auth_time < p_auth_time
      AND s.payload_sha256=p_hash AND s.post_id IS NOT DISTINCT FROM p_post;
    INSERT INTO sequent_backend.operational_event_setup_request
        (id,tenant_id,event_id,signing_request_id,official,post_id,payload_sha256,auth_time,eligible_until)
    VALUES (p_ticket,p_tenant,p_event,p_request,p_official,p_post,p_hash,p_auth_time,v_now + interval '10 minutes')
    ON CONFLICT (tenant_id,event_id,signing_request_id,official) WHERE superseded_at IS NULL DO NOTHING;
    IF FOUND THEN RETURN p_ticket; END IF;

    -- A lost browser response recovers the same live ticket, never a new
    -- generation. Fresh Gold and the exact scope were still checked above.
    SELECT s.id INTO v_ticket FROM sequent_backend.operational_event_setup_request s
    WHERE (s.tenant_id,s.event_id,s.signing_request_id,s.official) =
          (p_tenant,p_event,p_request,p_official)
      AND s.superseded_at IS NULL AND s.payload_sha256=p_hash
      AND s.post_id IS NOT DISTINCT FROM p_post
      AND (s.eligible_until > clock_timestamp() OR s.attempt IS NOT NULL
           OR s.attached_certificate IS NOT NULL);
    RETURN v_ticket;
END;
$$;
REVOKE ALL ON FUNCTION sequent_backend.operational_request_event_setup(uuid,uuid,uuid,uuid,uuid,text,timestamptz,uuid) FROM PUBLIC;
ALTER FUNCTION sequent_backend.operational_request_event_setup(uuid,uuid,uuid,uuid,uuid,text,timestamptz,uuid) OWNER TO sequent_os_ledger_api_owner;
DO $role$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname='sequent_os_product_setup_actor') THEN
        CREATE ROLE sequent_os_product_setup_actor NOLOGIN;
    ELSIF EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname='sequent_os_product_setup_actor'
        AND (rolcanlogin OR rolsuper OR rolcreaterole OR rolcreatedb OR rolreplication OR rolbypassrls)) THEN
        RAISE EXCEPTION 'existing product setup role is not a limited capability';
    END IF;
END;
$role$;
GRANT USAGE ON SCHEMA sequent_backend TO sequent_os_product_setup_actor;
GRANT EXECUTE ON FUNCTION sequent_backend.operational_request_event_setup(uuid,uuid,uuid,uuid,uuid,text,timestamptz,uuid)
    TO sequent_os_product_setup_actor;

CREATE FUNCTION sequent_backend.operational_prepare_requested_event_setup(
    p_tenant uuid,p_event uuid,p_official uuid,p_ticket uuid,p_officer uuid,p_attempt uuid
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
DECLARE v_now timestamptz;
BEGIN
    IF p_tenant IS NULL OR p_event IS NULL OR p_official IS NULL OR p_ticket IS NULL
       OR p_officer IS NULL OR p_attempt IS NULL OR p_official = p_officer
       OR current_setting('transaction_isolation') <> 'read committed' THEN RETURN false; END IF;
    PERFORM pg_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment',0));
    IF NOT sequent_backend.operational_signing_event_hsm_required(p_tenant,p_event)
       OR NOT EXISTS (
           SELECT FROM sequent_backend.operational_setup_installation
           WHERE tenant_id = p_tenant AND device_profile = 'yubihsm'
       ) THEN RETURN false; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended('signing-event:'||p_tenant::text||':'||p_event::text,0));
    v_now := clock_timestamp();
    PERFORM FROM sequent_backend.operational_event_setup_request s
    JOIN sequent_backend.signing_request r
      ON (r.tenant_id,r.election_event_id,r.id,r.payload_sha256)
       = (s.tenant_id,s.event_id,s.signing_request_id,s.payload_sha256)
    WHERE (s.tenant_id,s.event_id,s.official,s.id)=(p_tenant,p_event,p_official,p_ticket)
      AND s.superseded_at IS NULL AND s.attempt IS NULL AND s.eligible_until > v_now
      AND r.election_id IS NOT DISTINCT FROM s.post_id AND r.status = 'waiting'
      AND (r.expires_at IS NULL OR r.expires_at > v_now)
      AND encode(sha256(convert_to(r.canonical_payload,'UTF8')),'hex') = s.payload_sha256
    FOR UPDATE OF r,s;
    IF NOT FOUND THEN RETURN false; END IF;
    IF '00000000-0000-0000-0000-000000000000' IN
        (p_tenant,p_event,p_official,p_officer,p_attempt) THEN RETURN false; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'operational-signing-official:' || p_tenant::text || ':' || p_official::text, 0));
    IF EXISTS (
        SELECT FROM sequent_backend.operational_signing_official_enrollment e
        WHERE (e.tenant_id,e.actor)=(p_tenant,p_official::text)
    ) THEN RETURN false; END IF;
    v_now := clock_timestamp();
    INSERT INTO sequent_backend.operational_setup_generation
        (attempt,tenant_id,event_id,official,prepared_by,device_profile,prepared_at,claim_deadline)
    VALUES (p_attempt,p_tenant,p_event,p_official,p_officer,'yubihsm',v_now,v_now + interval '2 minutes')
    ON CONFLICT DO NOTHING;
    IF NOT FOUND THEN RETURN false; END IF;
    UPDATE sequent_backend.operational_event_setup_request SET attempt = p_attempt WHERE id = p_ticket;
    RETURN FOUND;
END;
$$;
REVOKE ALL ON FUNCTION sequent_backend.operational_prepare_requested_event_setup(uuid,uuid,uuid,uuid,uuid,uuid) FROM PUBLIC;
ALTER FUNCTION sequent_backend.operational_prepare_requested_event_setup(uuid,uuid,uuid,uuid,uuid,uuid) OWNER TO sequent_os_ledger_api_owner;
-- Internal definer operation; only the authenticated dispatch wrapper calls it.
REVOKE CREATE ON SCHEMA sequent_backend FROM sequent_os_ledger_api_owner, sequent_os_ledger_data_owner;

-- Run on a dedicated ordinary setup worker credential. Unlike the sealed
-- fixture's setup role, this principal cannot directly invoke any legacy
-- generation runner, device signing gate, or privilege to attach a cert.
DO $role$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname='sequent_os_product_setup_device') THEN
        CREATE ROLE sequent_os_product_setup_device NOLOGIN;
    ELSIF EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname='sequent_os_product_setup_device'
        AND (rolcanlogin OR rolsuper OR rolcreaterole OR rolcreatedb OR rolreplication OR rolbypassrls)) THEN
        RAISE EXCEPTION 'existing product setup role is not a limited capability';
    END IF;
END;
$role$;
GRANT USAGE ON SCHEMA sequent_backend TO sequent_os_product_setup_device;
GRANT CREATE ON SCHEMA sequent_backend TO sequent_os_ledger_api_owner;
CREATE FUNCTION sequent_backend.operational_claim_requested_event_setup(
    p_tenant uuid,p_event uuid,p_official uuid,p_attempt uuid
) RETURNS TABLE(fence uuid,deadline timestamptz) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
BEGIN
    IF NOT EXISTS (
        SELECT FROM sequent_backend.operational_event_setup_request s
        JOIN sequent_backend.operational_setup_generation g ON g.attempt=s.attempt
        WHERE (s.tenant_id,s.event_id,s.official,s.attempt)=(p_tenant,p_event,p_official,p_attempt)
          AND g.device_profile='yubihsm' AND s.dispatch_officer=g.prepared_by AND s.dispatch_auth_time IS NOT NULL
    ) THEN RETURN; END IF;
    RETURN QUERY SELECT c.fence,c.deadline
    FROM sequent_backend.operational_setup_claim_generation(p_tenant,p_event,p_official,p_attempt) c;
END;
$$;
CREATE FUNCTION sequent_backend.operational_complete_requested_event_setup(
    p_tenant uuid,p_event uuid,p_official uuid,p_attempt uuid,p_fence uuid,
    p_spki text,p_evidence text,p_csr text
) RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
BEGIN
    IF NOT EXISTS (
        SELECT FROM sequent_backend.operational_event_setup_request s
        JOIN sequent_backend.operational_setup_generation g ON g.attempt=s.attempt
        WHERE (s.tenant_id,s.event_id,s.official,s.attempt)=(p_tenant,p_event,p_official,p_attempt)
          AND g.device_profile='yubihsm' AND s.dispatch_officer=g.prepared_by AND s.dispatch_auth_time IS NOT NULL AND g.fence=p_fence
    ) THEN RETURN NULL; END IF;
    RETURN sequent_backend.operational_setup_complete_generation(
        p_tenant,p_event,p_official,p_attempt,p_fence,p_spki,p_evidence,p_csr
    );
END;
$$;
REVOKE ALL ON FUNCTION sequent_backend.operational_claim_requested_event_setup(uuid,uuid,uuid,uuid),
    sequent_backend.operational_complete_requested_event_setup(uuid,uuid,uuid,uuid,uuid,text,text,text)
    FROM PUBLIC;
ALTER FUNCTION sequent_backend.operational_claim_requested_event_setup(uuid,uuid,uuid,uuid)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_complete_requested_event_setup(uuid,uuid,uuid,uuid,uuid,text,text,text)
    OWNER TO sequent_os_ledger_api_owner;
GRANT EXECUTE ON FUNCTION sequent_backend.operational_claim_requested_event_setup(uuid,uuid,uuid,uuid),
    sequent_backend.operational_complete_requested_event_setup(uuid,uuid,uuid,uuid,uuid,text,text,text)
    TO sequent_os_product_setup_device;
REVOKE CREATE ON SCHEMA sequent_backend FROM sequent_os_ledger_api_owner;

-- The independently authorized CA workflow reads only public CSR data and
-- the pinned attested SPKI. It cannot elect a different actor/event/Post.
GRANT CREATE ON SCHEMA sequent_backend TO sequent_os_ledger_api_owner;
CREATE FUNCTION sequent_backend.operational_setup_certificate_material(
    p_tenant uuid,p_event uuid,p_ticket uuid,p_officer uuid
) RETURNS TABLE(official uuid, post uuid, initial_attempt uuid, enrollment uuid, spki text, csr text)
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp
AS $$
BEGIN
    IF p_officer IS NULL OR p_officer='00000000-0000-0000-0000-000000000000'
       OR current_setting('transaction_isolation') <> 'read committed' THEN RETURN; END IF;
    PERFORM pg_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment',0));
    IF NOT sequent_backend.operational_signing_event_hsm_required(p_tenant,p_event) THEN RETURN; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended('signing-event:'||p_tenant::text||':'||p_event::text,0));
    RETURN QUERY
    SELECT s.official,s.post_id,s.attempt,k.id,k.spki,g.public_csr_pem
    FROM sequent_backend.operational_event_setup_request s
    JOIN sequent_backend.signing_request r
      ON (r.tenant_id,r.election_event_id,r.id,r.payload_sha256)
       = (s.tenant_id,s.event_id,s.signing_request_id,s.payload_sha256)
    JOIN sequent_backend.operational_signing_official_enrollment k
      ON (k.tenant_id,k.actor)=(s.tenant_id,s.official::text)
    JOIN sequent_backend.operational_setup_generation g
      ON (g.tenant_id,g.official,g.enrollment_id,g.spki_sha256)=(k.tenant_id,s.official,k.id,k.spki)
    WHERE (s.id,s.tenant_id,s.event_id)=(p_ticket,p_tenant,p_event)
      AND s.official <> p_officer AND s.superseded_at IS NULL
      AND s.attached_certificate IS NULL AND r.election_id IS NOT DISTINCT FROM s.post_id
      AND r.status='waiting' AND (r.expires_at IS NULL OR r.expires_at > clock_timestamp())
      AND encode(sha256(convert_to(r.canonical_payload,'UTF8')),'hex')=s.payload_sha256
      AND k.revoked_at IS NULL AND k.nonexportable_confirmed
      AND k.device_profile='yubihsm' AND g.device_profile=k.device_profile
      AND ((s.attempt=g.attempt AND g.event_id=s.event_id AND g.step='completed')
           OR (s.attempt IS NULL
               AND g.step IN ('completed','attached') AND s.eligible_until>clock_timestamp()))
    FOR UPDATE OF r,s;
END;
$$;
REVOKE ALL ON FUNCTION sequent_backend.operational_setup_certificate_material(uuid,uuid,uuid,uuid) FROM PUBLIC;
ALTER FUNCTION sequent_backend.operational_setup_certificate_material(uuid,uuid,uuid,uuid)
    OWNER TO sequent_os_ledger_api_owner;
GRANT EXECUTE ON FUNCTION sequent_backend.operational_setup_certificate_material(uuid,uuid,uuid,uuid)
    TO sequent_os_setup_authority;
REVOKE CREATE ON SCHEMA sequent_backend FROM sequent_os_ledger_api_owner;

-- Dispatch controller role is distinct from the CA registrar and device.
-- The server derives the official from the trusted ticket, not officer input.
DO $role$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname='sequent_os_product_setup_dispatch') THEN
        CREATE ROLE sequent_os_product_setup_dispatch NOLOGIN;
    ELSIF EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname='sequent_os_product_setup_dispatch'
        AND (rolcanlogin OR rolsuper OR rolcreaterole OR rolcreatedb OR rolreplication OR rolbypassrls)) THEN
        RAISE EXCEPTION 'existing product setup role is not a limited capability';
    END IF;
END;
$role$;
GRANT USAGE ON SCHEMA sequent_backend TO sequent_os_product_setup_dispatch;
GRANT CREATE ON SCHEMA sequent_backend TO sequent_os_ledger_api_owner;
CREATE FUNCTION sequent_backend.operational_dispatch_event_setup(
    p_tenant uuid,p_event uuid,p_ticket uuid,p_officer uuid,p_attempt uuid,p_auth_time timestamptz
) RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp
AS $$
DECLARE v_official uuid; v_now timestamptz;
BEGIN
    IF p_auth_time IS NULL OR NOT isfinite(p_auth_time)
       OR current_setting('transaction_isolation') <> 'read committed' THEN RETURN NULL; END IF;
    PERFORM pg_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment',0));
    IF NOT sequent_backend.operational_signing_event_hsm_required(p_tenant,p_event) THEN RETURN NULL; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended('signing-event:'||p_tenant::text||':'||p_event::text,0));
    v_now := clock_timestamp();
    IF p_auth_time > v_now OR p_auth_time <= v_now - interval '60 seconds' THEN RETURN NULL; END IF;
    SELECT s.official INTO v_official FROM sequent_backend.operational_event_setup_request s
    WHERE (s.tenant_id,s.event_id,s.id)=(p_tenant,p_event,p_ticket)
      AND s.superseded_at IS NULL AND s.attempt IS NULL AND s.attached_certificate IS NULL
      AND s.official<>p_officer AND s.eligible_until>v_now;
    IF NOT FOUND THEN RETURN NULL; END IF;
    IF NOT sequent_backend.operational_prepare_requested_event_setup(
        p_tenant,p_event,v_official,p_ticket,p_officer,p_attempt
    ) THEN RETURN NULL; END IF;
    -- Durable officer authorization is written by the same narrow definer in
    -- this transaction. The dispatcher is never an audit-table writer.
    UPDATE sequent_backend.operational_event_setup_request
    SET dispatch_officer=p_officer,dispatch_auth_time=p_auth_time,dispatched_at=clock_timestamp()
    WHERE id=p_ticket AND attempt=p_attempt AND dispatch_officer IS NULL;
    IF NOT FOUND THEN RAISE EXCEPTION 'exact dispatch audit not recorded'; END IF;
    RETURN v_official;
END;
$$;
REVOKE ALL ON FUNCTION sequent_backend.operational_dispatch_event_setup(uuid,uuid,uuid,uuid,uuid,timestamptz)
    FROM PUBLIC;
ALTER FUNCTION sequent_backend.operational_dispatch_event_setup(uuid,uuid,uuid,uuid,uuid,timestamptz)
    OWNER TO sequent_os_ledger_api_owner;
GRANT EXECUTE ON FUNCTION sequent_backend.operational_dispatch_event_setup(uuid,uuid,uuid,uuid,uuid,timestamptz)
    TO sequent_os_product_setup_dispatch;
REVOKE CREATE ON SCHEMA sequent_backend FROM sequent_os_ledger_api_owner;

-- Opaque phase, never a proof that an operational signing certificate/CA
-- currently authorizes an approval. A claimed timeout is never a retry.
GRANT CREATE ON SCHEMA sequent_backend TO sequent_os_ledger_api_owner;
CREATE FUNCTION sequent_backend.operational_setup_request_phase(
    p_tenant uuid,p_event uuid,p_request uuid,p_official uuid,p_ticket uuid
) RETURNS text LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog,pg_temp
AS $$
    SELECT CASE
        WHEN EXISTS (
            SELECT FROM sequent_backend.operational_signing_official_certificate a
            JOIN sequent_backend.operational_signing_official_enrollment k
              ON (k.tenant_id,k.id,k.actor)=(a.tenant_id,a.enrollment_id,a.actor)
            JOIN sequent_backend.staff_certificate c
              ON (c.tenant_id,c.election_event_id,c.id)=(a.tenant_id,a.event_id,a.certificate_id)
            WHERE (a.tenant_id,a.event_id,a.actor)=(s.tenant_id,s.event_id,s.official::text)
              AND a.retired_at IS NULL AND k.revoked_at IS NULL AND c.status='active'
              AND c.fingerprint_sha256=a.certificate_fingerprint AND c.spki_sha256=k.spki
              AND c.election_id IS NOT DISTINCT FROM s.post_id
              AND c.not_before <= clock_timestamp() AND c.not_after > clock_timestamp()
        ) THEN 'certificate-on-file'
        WHEN g.step IN ('completed','attached') OR (
            s.attempt IS NULL AND EXISTS (
                SELECT FROM sequent_backend.operational_signing_official_enrollment k
                WHERE (k.tenant_id,k.actor)=(s.tenant_id,s.official::text)
                  AND k.device_profile='yubihsm' AND k.revoked_at IS NULL
            )
        ) THEN 'certificate-pending'
        WHEN (g.step='prepared' AND g.claim_deadline > clock_timestamp())
          OR (g.step='claimed' AND g.device_deadline > clock_timestamp()) THEN 'hardware-generation-pending'
        WHEN s.attempt IS NOT NULL THEN 'reconciliation-required'
        ELSE 'awaiting-independent-officer'
    END
    FROM sequent_backend.operational_event_setup_request s
    LEFT JOIN sequent_backend.operational_setup_generation g ON g.attempt=s.attempt
    WHERE (s.tenant_id,s.event_id,s.signing_request_id,s.official,s.id)=
          (p_tenant,p_event,p_request,p_official,p_ticket)
      AND s.superseded_at IS NULL
      AND sequent_backend.operational_signing_event_hsm_required(p_tenant,p_event);
$$;
REVOKE ALL ON FUNCTION sequent_backend.operational_setup_request_phase(uuid,uuid,uuid,uuid,uuid) FROM PUBLIC;
ALTER FUNCTION sequent_backend.operational_setup_request_phase(uuid,uuid,uuid,uuid,uuid)
    OWNER TO sequent_os_ledger_api_owner;
GRANT EXECUTE ON FUNCTION sequent_backend.operational_setup_request_phase(uuid,uuid,uuid,uuid,uuid)
    TO sequent_os_product_setup_actor;
REVOKE CREATE ON SCHEMA sequent_backend FROM sequent_os_ledger_api_owner;
