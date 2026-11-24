-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

CREATE OR REPLACE FUNCTION sequent_backend.operational_setup_guard()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
BEGIN
    IF TG_TABLE_NAME = 'operational_setup_installation' THEN
        IF TG_OP = 'INSERT'
           AND sequent_backend.operational_signing_hsm_required(NEW.tenant_id) THEN
            RETURN NEW;
        END IF;
        RAISE EXCEPTION 'operational setup installation needs an HSM-required tenant and cannot change';
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
    IF NOT sequent_backend.operational_signing_hsm_required(p_tenant)
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
    IF p_tenant IS NULL OR p_official IS NULL
       OR NOT sequent_backend.operational_signing_hsm_required(p_tenant) THEN
        RETURN;
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
    IF p_tenant IS NULL OR p_official IS NULL OR p_fence IS NULL
       OR p_spki_sha256 IS NULL OR p_spki_sha256 !~ '^[0-9a-f]{64}$'
       OR p_evidence_sha256 IS NULL OR p_evidence_sha256 !~ '^[0-9a-f]{64}$'
       OR p_public_csr_pem IS NULL OR octet_length(p_public_csr_pem) NOT BETWEEN 64 AND 16384
       OR p_public_csr_pem !~ '^-----BEGIN CERTIFICATE REQUEST-----\r?\n[A-Za-z0-9+/=\r\n]+-----END CERTIFICATE REQUEST-----\r?\n?$'
       OR NOT sequent_backend.operational_signing_hsm_required(p_tenant) THEN
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
      AND sequent_backend.operational_signing_hsm_required(p_tenant);
$$;

CREATE OR REPLACE FUNCTION sequent_backend.operational_setup_pin_for_attachment(
    p_tenant uuid, p_event uuid, p_official uuid, p_authority uuid, p_attempt uuid
) RETURNS TABLE (enrollment_id uuid, expected_spki text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
BEGIN
    IF p_tenant IS NULL OR p_official IS NULL OR p_authority IS NULL
       OR p_authority = p_official OR p_authority = '00000000-0000-0000-0000-000000000000'
       OR NOT sequent_backend.operational_signing_hsm_required(p_tenant) THEN
        RETURN;
    END IF;
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
    IF p_tenant IS NULL OR p_event IS NULL OR p_official IS NULL OR p_authority IS NULL
       OR p_authority = p_official OR p_authority = '00000000-0000-0000-0000-000000000000'
       OR p_enrollment IS NULL OR p_certificate IS NULL
       OR p_fingerprint IS NULL OR p_fingerprint !~ '^[0-9a-f]{64}$'
       OR NOT sequent_backend.operational_signing_hsm_required(p_tenant) THEN
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
      AND sequent_backend.operational_signing_hsm_required(p_tenant)
      AND EXISTS (SELECT FROM sequent_backend.election_event AS ev
                  WHERE (ev.tenant_id, ev.id) = (p_tenant, p_event));
$$;

DROP FUNCTION sequent_backend.operational_setup_attach_existing_event_certificate(
    uuid, uuid, uuid, uuid, uuid, uuid, uuid, text, text[], uuid
);
DROP FUNCTION sequent_backend.operational_event_setup_allowed(uuid, uuid);
DROP FUNCTION sequent_backend.operational_dispatch_event_setup(uuid,uuid,uuid,uuid,uuid,timestamptz);
DROP FUNCTION sequent_backend.operational_prepare_requested_event_setup(uuid,uuid,uuid,uuid,uuid,uuid);
DROP FUNCTION sequent_backend.operational_request_event_setup(uuid,uuid,uuid,uuid,uuid,text,timestamptz,uuid);
DROP FUNCTION sequent_backend.operational_claim_requested_event_setup(uuid,uuid,uuid,uuid);
DROP FUNCTION sequent_backend.operational_complete_requested_event_setup(uuid,uuid,uuid,uuid,uuid,text,text,text);
DROP FUNCTION sequent_backend.operational_setup_certificate_material(uuid,uuid,uuid,uuid);
DROP FUNCTION sequent_backend.operational_setup_request_phase(uuid,uuid,uuid,uuid,uuid);
DROP TABLE sequent_backend.operational_event_setup_request;
REVOKE USAGE ON SCHEMA sequent_backend FROM sequent_os_product_setup_device;
REVOKE USAGE ON SCHEMA sequent_backend FROM sequent_os_product_setup_actor;
REVOKE USAGE ON SCHEMA sequent_backend FROM sequent_os_product_setup_dispatch;
