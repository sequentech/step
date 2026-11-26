-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-- Local overlay, after 2250 and 2260, transactionally. No session, request,
-- key, certificate, identity or approval row is written by this migration.
SET LOCAL ROLE sequent_os_ledger_api_owner;

-- Original enrollment and current approval are deliberately separate.
-- The current binding is the existing 2200 waiting/canonical/live marker
-- check. Resolve the original completed generation only by immutable scope.
-- The current request is never substituted into the original Gold grant.
CREATE FUNCTION felix3_initialize_bridge.recovery_origin(b felix3_initialize_setup.binding)
RETURNS TABLE (attempt uuid, responsible uuid, original_request uuid, original_digest text,
               session_hash text, enrollment uuid, original_spki text, original_csr_hash text,
               authenticated_after timestamptz, certificate uuid)
LANGUAGE sql VOLATILE STRICT SET search_path = pg_catalog, pg_temp AS $$
    SELECT a.id,p.officer,(a.bound).request,(a.bound).payload_sha256,a.portal_session_sha256,
           g.enrollment_id,g.spki_sha256,encode(sha256(convert_to(g.public_csr_pem,'UTF8')),'hex'),
           greatest(g.completed_at,live.created_at),g.certificate_id
      FROM felix3_initialize_setup.eligible(b,false) p
      JOIN felix3_initialize_setup.attempt a
        ON ((a.bound).installation,(a.bound).tenant,(a.bound).event,(a.bound).post,
            (a.bound).publication,(a.bound).official,a.officer)
         = (b.installation,b.tenant,b.event,b.post,b.publication,b.official,p.officer)
      JOIN sequent_backend.operational_setup_generation g ON
           (g.attempt,g.tenant_id,g.event_id,g.official,g.prepared_by,g.device_profile)
         = (a.id,b.tenant,b.event,b.official,p.officer,'softhsm-test')
      JOIN sequent_backend.operational_signing_official_enrollment e ON
           (e.id,e.tenant_id,e.actor,e.spki,e.device_profile,e.key_version,e.device_evidence_hash)
         = (g.enrollment_id,b.tenant,b.official::text,g.spki_sha256,'softhsm-test',1,g.evidence_sha256)
      JOIN sequent_backend.signing_request old ON
           (old.tenant_id,old.election_event_id,old.election_id,old.id,old.payload_sha256,old.action)
         = (b.tenant,b.event,b.post,(a.bound).request,(a.bound).payload_sha256,'initialize-voting')
      JOIN sequent_backend.signing_request live ON
           (live.tenant_id,live.election_event_id,live.election_id,live.id,live.payload_sha256,live.action,live.scope_key)
         = (b.tenant,b.event,b.post,b.request,b.payload_sha256,'initialize-voting',old.scope_key)
     WHERE p.until_at > clock_timestamp() AND p.officer <> b.official
       AND a.abandoned_at IS NULL AND a.portal_session_sha256 IS NOT NULL
       AND a.exchange_at IS NOT NULL AND a.auth_time IS NOT NULL AND a.gold_until IS NOT NULL
       AND a.released_at IS NOT NULL AND a.released_at < a.gold_until
       AND a.claimed_until IS NOT NULL AND a.claimed_until <= a.gold_until
       AND g.step IN ('completed','attached') AND g.fence IS NOT NULL AND g.evidence_sha256 IS NOT NULL
       AND g.completed_at >= a.released_at AND g.spki_sha256 ~ '^[0-9a-f]{64}$'
       AND octet_length(g.public_csr_pem) BETWEEN 64 AND 16384
       AND e.nonexportable_confirmed AND e.revoked_at IS NULL
       AND e.key_id = 'official/' || b.tenant::text || '/' || b.official::text
       AND old.document_id IS NULL AND old.document_sha256 IS NULL AND isfinite(old.expires_at)
       AND felix3_initialize_setup.payload_matches(a.bound,old.subject,old.canonical_payload,old.expires_at)
       AND ((a.bound).request = b.request OR
            (live.created_at > old.created_at AND old.executed_at IS NULL AND old.completed_at IS NULL
             AND (old.status IN ('expired','cancelled') OR
                  (old.status = 'waiting' AND date_trunc('second',old.expires_at) <= clock_timestamp()))))
       AND sequent_backend.operational_signing_hsm_required(b.tenant)
       AND NOT EXISTS (SELECT FROM sequent_backend.signing_approval v
            WHERE (v.tenant_id,v.election_event_id,v.user_id) = (b.tenant,b.event,b.official::text)
              AND v.request_id IN ((a.bound).request,b.request));
$$;
REVOKE ALL ON FUNCTION felix3_initialize_bridge.recovery_origin(felix3_initialize_setup.binding) FROM PUBLIC;

-- Session comparison stays inside the setup role's read-only view. Neither
-- the original session hash nor the original public request leaves this gate.
-- Returned pins and auth threshold are used only inside authenticated Harvest.
CREATE FUNCTION felix3_initialize_bridge.nominee_recovery(
    b felix3_initialize_setup.binding, session_digest text
) RETURNS TABLE (attempt uuid, officer uuid, enrollment uuid, csr_sha256 text, spki_sha256 text,
                 needs_gold boolean, authenticated_after timestamptz, certificate uuid)
LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path = pg_catalog, pg_temp AS $$
BEGIN
    IF felix3_initialize_setup.principal('felix3_local_setup_portal') IS NOT TRUE
       OR session_digest !~ '^[0-9a-f]{64}$' THEN RETURN; END IF;
    RETURN QUERY SELECT o.attempt,o.responsible,o.enrollment,o.original_csr_hash,o.original_spki,
        (o.original_request,o.original_digest,o.session_hash) IS DISTINCT FROM (b.request,b.payload_sha256,session_digest),
        o.authenticated_after,o.certificate
      FROM felix3_initialize_bridge.recovery_origin(b) o;
END;
$$;
REVOKE ALL ON FUNCTION felix3_initialize_bridge.nominee_recovery(felix3_initialize_setup.binding,text) FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_proc p
        CROSS JOIN LATERAL aclexplode(coalesce(p.proacl,acldefault('f',p.proowner))) a
        WHERE p.pronamespace = 'felix3_initialize_bridge'::regnamespace
          AND p.proname IN ('recovery_origin','nominee_recovery') AND a.grantee <> p.proowner)
       THEN RAISE EXCEPTION 'unexpected original enrollment recovery privileges'; END IF;
END;
$$;

CREATE OR REPLACE FUNCTION felix3_initialize_attachment.generation(
    b felix3_initialize_setup.binding, ticket uuid, who uuid, officer_labels text[], nominee_labels text[]
) RETURNS TABLE (enrollment uuid, spki text, csr text, certificate uuid, fingerprint text,
                 attached_at timestamptz, deadline timestamptz)
LANGUAGE plpgsql STRICT SET search_path = pg_catalog, pg_temp AS $$
DECLARE
    permit record;
    provenance record;
    g sequent_backend.operational_setup_generation%ROWTYPE;
    pin record;
    original text;
BEGIN
    SELECT * INTO permit FROM felix3_initialize_setup.eligible(b,true);
    IF NOT FOUND OR permit.officer <> who OR who = b.official THEN RETURN; END IF;
    -- Both verified sessions must see the request's frozen AND live Post labels.
    IF NOT felix3_initialize_attachment.post_access(b,officer_labels,nominee_labels) THEN RETURN; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'operational-signing-official:' || b.tenant::text || ':' || b.official::text,0));
    -- 'b' is the live authorization. Provenance still names the original
    -- attempt and original signed request, including its old expiry/digest.
    SELECT * INTO provenance FROM felix3_initialize_bridge.recovery_origin(b) o
      WHERE (o.attempt,o.responsible) = (ticket,who);
    IF NOT FOUND THEN RETURN; END IF;
    SELECT s.* INTO g FROM sequent_backend.operational_setup_generation s
      JOIN felix3_initialize_setup.attempt a ON a.id = s.attempt
      JOIN sequent_backend.operational_signing_official_enrollment e
        ON (e.tenant_id,e.actor,e.id,e.spki,e.device_profile)
         = (s.tenant_id,s.official::text,s.enrollment_id,s.spki_sha256,s.device_profile)
      JOIN sequent_backend.operational_setup_installation i
        ON (i.tenant_id,i.device_profile) = (s.tenant_id,s.device_profile)
      WHERE (s.attempt,s.tenant_id,s.event_id,s.official,s.prepared_by,s.device_profile)
          = (ticket,b.tenant,b.event,b.official,who,'softhsm-test')
        AND ((a.bound).request,(a.bound).payload_sha256,a.officer)
            = (provenance.original_request,provenance.original_digest,who)
        AND a.abandoned_at IS NULL
        AND a.released_at IS NOT NULL AND a.claimed_until IS NOT NULL
        AND s.step IN ('completed','attached') AND s.fence IS NOT NULL AND s.evidence_sha256 IS NOT NULL
        AND e.nonexportable_confirmed AND e.revoked_at IS NULL
        AND sequent_backend.operational_signing_hsm_required(b.tenant)
        AND NOT EXISTS (SELECT FROM sequent_backend.signing_approval v
             WHERE (v.tenant_id,v.election_event_id,v.request_id,v.user_id)
                 = (b.tenant,b.event,b.request,b.official::text))
      FOR UPDATE OF s FOR SHARE OF a NOWAIT;
    IF NOT FOUND OR permit.until_at <= clock_timestamp() THEN RETURN; END IF;
    IF g.step = 'completed' THEN
        original := sequent_backend.operational_setup_public_csr(b.tenant,b.event,b.official,who,ticket);
        SELECT * INTO pin FROM sequent_backend.operational_setup_pin_for_attachment(b.tenant,b.event,b.official,who,ticket);
        IF NOT FOUND OR (original,pin.enrollment_id,pin.expected_spki)
            IS DISTINCT FROM (g.public_csr_pem,g.enrollment_id,g.spki_sha256)
            OR sequent_backend.operational_official_setup_status(b.tenant,b.event,b.official::text)
                IS DISTINCT FROM 'certificate-not-attached' THEN RETURN; END IF;
    ELSE
        IF g.attached_by IS DISTINCT FROM who THEN RETURN; END IF;
        original := g.public_csr_pem;
    END IF;
    RETURN QUERY SELECT g.enrollment_id,g.spki_sha256,original,g.certificate_id,g.certificate_fingerprint,
                        g.attached_at,permit.until_at;
END;
$$;
RESET ROLE;
