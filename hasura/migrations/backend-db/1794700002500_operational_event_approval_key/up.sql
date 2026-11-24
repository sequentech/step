-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

GRANT CREATE ON SCHEMA sequent_backend TO sequent_os_ledger_api_owner;

-- Internal defensive gate for already authenticated generic signing
-- services. Invalid/missing mode and an unknown tenant/event are false,
-- never treated as an absent event choice.
CREATE FUNCTION sequent_backend.operational_signing_event_hsm_required(
    p_tenant uuid, p_event uuid
) RETURNS boolean LANGUAGE sql STABLE SECURITY INVOKER
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT coalesce((
        SELECT f.deployment_mode = 'hsm-required' OR f.tenant_required
               OR f.explicit_mode = 'hsm-required'
               OR (f.product_active AND f.explicit_mode IS NULL)
        FROM sequent_backend.operational_signing_event_mode_facts(p_tenant, p_event) AS f
    ), false);
$$;
REVOKE ALL ON FUNCTION sequent_backend.operational_signing_event_hsm_required(uuid, uuid)
    FROM PUBLIC;
ALTER FUNCTION sequent_backend.operational_signing_event_hsm_required(uuid, uuid)
    OWNER TO sequent_os_ledger_api_owner;

-- Internal approval caller only, after checking the actor's permission and
-- taking the shared policy, event and request locks. The returned public
-- key material is not proof of device/CA readiness and is not a new grant.
CREATE FUNCTION sequent_backend.operational_signing_current_approval_key(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text
) RETURNS TABLE (
    key_id text, key_version integer, cert_fingerprint text, cert_spki text,
    certificate_id uuid, chain_pem text[], device_profile text
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT k.key_id, k.key_version, a.certificate_fingerprint, k.spki,
           a.certificate_id, a.chain_pem, k.device_profile
    FROM sequent_backend.signing_request AS r
    JOIN sequent_backend.operational_signing_official_enrollment AS k
      ON (k.tenant_id, k.actor) = (r.tenant_id, p_actor)
    JOIN sequent_backend.operational_signing_official_certificate AS a
      ON (a.tenant_id, a.event_id, a.actor, a.enrollment_id) =
         (r.tenant_id, r.election_event_id, p_actor, k.id)
    JOIN sequent_backend.staff_certificate AS c
      ON (c.tenant_id, c.election_event_id, c.id, c.user_id) =
         (a.tenant_id, a.event_id, a.certificate_id, p_actor)
    WHERE (r.tenant_id, r.election_event_id, r.id) = (p_tenant, p_event, p_request)
      AND r.status = 'waiting'
      AND (r.expires_at IS NULL OR r.expires_at > clock_timestamp())
      AND k.revoked_at IS NULL AND k.nonexportable_confirmed
      AND k.device_profile = 'yubihsm'
      AND k.key_id = 'official/' || p_tenant::text || '/' || p_actor
      AND a.retired_at IS NULL AND a.certificate_source = 'validated-staff-ca'
      AND c.status = 'active' AND c.pem = a.chain_pem[1]
      AND c.fingerprint_sha256 = a.certificate_fingerprint AND c.spki_sha256 = k.spki
      AND c.not_before <= clock_timestamp() AND c.not_after > clock_timestamp()
      AND (c.election_id IS NULL OR c.election_id = r.election_id)
      AND sequent_backend.operational_signing_event_hsm_required(p_tenant, p_event);
$$;
REVOKE ALL ON FUNCTION sequent_backend.operational_signing_current_approval_key(uuid, uuid, uuid, text)
    FROM PUBLIC;
ALTER FUNCTION sequent_backend.operational_signing_current_approval_key(uuid, uuid, uuid, text)
    OWNER TO sequent_os_ledger_api_owner;
GRANT EXECUTE ON FUNCTION sequent_backend.operational_signing_current_approval_key(uuid, uuid, uuid, text)
    TO sequent_os_approve;
REVOKE CREATE ON SCHEMA sequent_backend FROM sequent_os_ledger_api_owner;
