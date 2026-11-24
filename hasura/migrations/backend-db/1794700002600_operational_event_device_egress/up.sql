-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Role names are cluster-wide. Reuse only the exact non-privileged shape.
DO $$
DECLARE
    existing pg_catalog.pg_roles%ROWTYPE;
BEGIN
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'sequent_os_miru_egress_reader') THEN
        BEGIN
            CREATE ROLE sequent_os_miru_egress_reader NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
        EXCEPTION WHEN duplicate_object OR unique_violation THEN NULL;
        END;
    END IF;
    SELECT * INTO STRICT existing FROM pg_catalog.pg_roles WHERE rolname = 'sequent_os_miru_egress_reader';
    IF existing.rolcanlogin OR existing.rolsuper OR existing.rolcreatedb OR existing.rolcreaterole
       OR existing.rolreplication OR existing.rolbypassrls
       OR EXISTS (SELECT FROM pg_catalog.pg_auth_members WHERE member = existing.oid) THEN
        RAISE EXCEPTION 'existing product signing role is not a bounded NOLOGIN role';
    END IF;
END;
$$;
GRANT USAGE ON SCHEMA sequent_backend TO sequent_os_miru_egress_reader;

-- Preserve all existing owner/EXECUTE privileges. Only event-aware
-- MIRU boundaries and the generic device flow read per-event mode; the
-- independently sealed Initialize grant continues using its original floor.

-- Allow only the existing independent inventory operator to classify old
-- files for an unset event immediately before explicit one-way activation.
-- This does not enable any signing or transfer on the event.
GRANT CREATE ON SCHEMA sequent_backend TO sequent_os_ledger_api_owner;
CREATE FUNCTION sequent_backend.operational_miru_staging_inventory(p_tenant uuid, p_event uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY INVOKER
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT pg_has_role(session_user, 'sequent_os_ledger_data_owner', 'MEMBER')
       AND EXISTS (
           SELECT FROM sequent_backend.operational_signing_event_mode_facts(p_tenant,p_event) f
           WHERE NOT f.product_active AND NOT f.tenant_required
             AND f.deployment_mode = 'legacy-p12' AND f.explicit_mode IS NULL
       );
$$;
REVOKE ALL ON FUNCTION sequent_backend.operational_miru_staging_inventory(uuid,uuid) FROM PUBLIC;
ALTER FUNCTION sequent_backend.operational_miru_staging_inventory(uuid,uuid) OWNER TO sequent_os_ledger_api_owner;
REVOKE CREATE ON SCHEMA sequent_backend FROM sequent_os_ledger_api_owner;

CREATE OR REPLACE FUNCTION sequent_backend.operational_miru_guard()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
BEGIN
    IF TG_TABLE_NAME = 'operational_miru_inventory' THEN
        IF TG_OP = 'INSERT' THEN
            IF NEW.state = 'open'
               AND (
                   sequent_backend.operational_signing_event_hsm_required(NEW.tenant_id, NEW.event_id)
                   OR sequent_backend.operational_miru_staging_inventory(NEW.tenant_id, NEW.event_id)
               ) THEN
                RETURN NEW;
            END IF;
        ELSIF OLD.state = 'open' AND NEW.state = 'verified'
              AND ROW(NEW.tenant_id, NEW.event_id, NEW.inventory_id, NEW.opened_at)
                  IS NOT DISTINCT FROM
                  ROW(OLD.tenant_id, OLD.event_id, OLD.inventory_id, OLD.opened_at) THEN
            RETURN NEW;
        END IF;
        RAISE EXCEPTION 'invalid operational MIRU inventory change'
            USING ERRCODE = 'check_violation';
    END IF;
    RAISE EXCEPTION 'operational MIRU registry rows are immutable'
        USING ERRCODE = 'check_violation';
END;
$$;

CREATE OR REPLACE FUNCTION sequent_backend.operational_miru_register_artifact(
    p_tenant uuid, p_event uuid, p_tally_session uuid, p_election uuid, p_area uuid,
    p_revision uuid, p_document uuid, p_kind text, p_content_sha256 text, p_inventory uuid
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
BEGIN
    IF p_tenant IS NULL OR p_event IS NULL OR p_tally_session IS NULL OR p_election IS NULL
       OR p_area IS NULL OR p_revision IS NULL OR p_document IS NULL
       OR p_kind IS NULL OR p_kind NOT IN ('archive', 'eml', 'xz', 'csv')
       OR (p_kind = 'archive') <> (p_document = p_revision)
       OR p_content_sha256 IS NULL OR p_content_sha256 !~ '^[0-9a-f]{64}$'
       OR NOT (
           sequent_backend.operational_signing_event_hsm_required(p_tenant, p_event)
           OR (p_inventory IS NOT NULL AND sequent_backend.operational_miru_staging_inventory(p_tenant,p_event))
       )
       OR NOT EXISTS (SELECT FROM sequent_backend.tally_session AS s
                      WHERE (s.tenant_id, s.election_event_id, s.id)
                          = (p_tenant, p_event, p_tally_session))
       OR NOT EXISTS (SELECT FROM sequent_backend.election AS e
                      WHERE (e.tenant_id, e.election_event_id, e.id)
                          = (p_tenant, p_event, p_election))
       OR NOT EXISTS (SELECT FROM sequent_backend.area AS a
                      WHERE (a.tenant_id, a.election_event_id, a.id)
                          = (p_tenant, p_event, p_area)) THEN
        RETURN false;
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'operational-miru-registry:' || p_tenant::text || ':' || p_event::text, 0));
    IF p_inventory IS NOT NULL
       AND (NOT pg_has_role(session_user, 'sequent_os_ledger_data_owner', 'MEMBER')
            OR NOT EXISTS (SELECT FROM sequent_backend.operational_miru_inventory AS v
                           WHERE (v.tenant_id, v.event_id, v.inventory_id)
                               = (p_tenant, p_event, p_inventory)
                             AND v.state = 'open')) THEN
        RETURN false;
    END IF;

    -- Decide before writing: a refusal leaves no partial classification.
    IF EXISTS (SELECT FROM sequent_backend.operational_miru_artifact AS a
               WHERE (a.tenant_id, a.document_id) = (p_tenant, p_document)
                 AND (a.event_id, a.kind, a.content_sha256)
                     IS DISTINCT FROM (p_event, p_kind, p_content_sha256))
       OR EXISTS (SELECT FROM sequent_backend.operational_miru_revision_artifact AS r
                  WHERE (r.tenant_id, r.event_id, r.revision_id, r.kind)
                      = (p_tenant, p_event, p_revision, p_kind)
                    AND (r.tally_session_id, r.election_id, r.area_id, r.document_id)
                        IS DISTINCT FROM (p_tally_session, p_election, p_area, p_document)) THEN
        RETURN false;
    END IF;
    INSERT INTO sequent_backend.operational_miru_artifact
        (tenant_id, event_id, document_id, kind, content_sha256, inventory_id)
    VALUES (p_tenant, p_event, p_document, p_kind, p_content_sha256, p_inventory)
    ON CONFLICT DO NOTHING;
    INSERT INTO sequent_backend.operational_miru_revision_artifact
        (tenant_id, event_id, tally_session_id, election_id, area_id, revision_id, kind,
         document_id)
    VALUES (p_tenant, p_event, p_tally_session, p_election, p_area, p_revision, p_kind,
            p_document)
    ON CONFLICT DO NOTHING;
    RETURN true;
END;
$$;

CREATE OR REPLACE FUNCTION sequent_backend.operational_miru_export_exclusions(
    p_tenant uuid, p_event uuid
) RETURNS uuid[] LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT CASE
        WHEN p_tenant IS NOT NULL AND p_event IS NOT NULL
             AND EXISTS (
                 SELECT FROM sequent_backend.operational_signing_event_mode_facts(p_tenant,p_event)
             )
             AND (
               NOT sequent_backend.operational_signing_event_hsm_required(p_tenant,p_event)
               OR EXISTS (SELECT FROM sequent_backend.operational_miru_inventory AS v
                          WHERE (v.tenant_id, v.event_id) = (p_tenant, p_event)
                            AND v.state = 'verified')
             )
        THEN (SELECT coalesce(array_agg(a.document_id ORDER BY a.document_id), '{}'::uuid[])
              FROM sequent_backend.operational_miru_artifact AS a
              WHERE (a.tenant_id, a.event_id) = (p_tenant, p_event))
        ELSE NULL
    END;
$$;

CREATE OR REPLACE FUNCTION sequent_backend.operational_miru_generic_withhold(
    p_tenant uuid, p_event uuid, p_document uuid
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT CASE
        WHEN p_tenant IS NULL OR p_document IS NULL THEN NULL
        WHEN EXISTS (SELECT FROM sequent_backend.operational_miru_artifact AS a
                     WHERE (a.tenant_id, a.document_id) = (p_tenant, p_document)) THEN true
        -- A document outside any event is never operational MIRU output.
        WHEN p_event IS NULL THEN false
        WHEN NOT EXISTS (
            SELECT FROM sequent_backend.operational_signing_event_mode_facts(p_tenant,p_event)
        ) THEN NULL
        WHEN NOT sequent_backend.operational_signing_event_hsm_required(p_tenant,p_event) THEN false
        WHEN EXISTS (SELECT FROM sequent_backend.operational_miru_inventory AS v
                     WHERE (v.tenant_id, v.event_id) = (p_tenant, p_event)
                       AND v.state = 'verified') THEN false
        ELSE NULL
    END;
$$;

CREATE OR REPLACE FUNCTION sequent_backend.operational_signing_get_device_grant(
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
      AND CASE g.flow
            WHEN 'initialize' THEN sequent_backend.operational_signing_hsm_required(p_tenant)
            WHEN 'generic' THEN sequent_backend.operational_signing_event_hsm_required(p_tenant, p_event)
            ELSE false
          END;
$$;

CREATE OR REPLACE FUNCTION sequent_backend.operational_signing_claim_device_from_reference(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_challenge uuid,
    p_fence uuid, p_binding_sha256 text
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
DECLARE
    b sequent_backend.operational_signing_binding;
    v_profile text;
BEGIN
    -- A reserve/read may be old, but the irreversible device claim cannot
    -- cross a policy update. Take the same shared lock as the Rust gates
    -- before the event lock; READ COMMITTED gives the next statement a
    -- fresh policy snapshot after waiting.
    IF current_setting('transaction_isolation') <> 'read committed' THEN
        RETURN false;
    END IF;
    PERFORM pg_advisory_xact_lock_shared(
        hashtextextended('operational-signing-policy:deployment', 0));
    IF p_tenant IS NULL OR p_event IS NULL OR p_fence IS NULL
       OR p_binding_sha256 IS NULL OR p_binding_sha256 !~ '^[0-9a-f]{64}$'
       OR NOT sequent_backend.operational_signing_device_caller('generic')
       OR NOT sequent_backend.operational_signing_event_hsm_required(p_tenant, p_event) THEN
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

GRANT EXECUTE ON FUNCTION
    sequent_backend.operational_miru_generic_withhold(uuid, uuid, uuid),
    sequent_backend.operational_miru_export_exclusions(uuid, uuid)
    TO sequent_os_miru_egress_reader;
