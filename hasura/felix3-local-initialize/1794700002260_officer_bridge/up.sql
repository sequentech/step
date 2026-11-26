-- SPDX-License-Identifier: AGPL-3.0-only
-- Local overlay only, after 2200, 2210 and 2250. These lookups do not grant a
-- database login or execute privilege, mint an identity, disclose a CSR, or
-- write enrollment, trust, permit or registry state. No caller selects a root.
CREATE SCHEMA felix3_initialize_bridge AUTHORIZATION sequent_os_ledger_api_owner;
REVOKE ALL ON SCHEMA felix3_initialize_bridge FROM PUBLIC;
SET LOCAL ROLE sequent_os_ledger_api_owner;

-- The nominee's verified setup session may recover only the hashes and IDs
-- for its original released, device-committed enrollment, including after a
-- process restart. The fresh human/Keycloak/Post check stays in Harvest.
CREATE FUNCTION felix3_initialize_bridge.nominee_generation(
    b felix3_initialize_setup.binding, session_digest text
) RETURNS TABLE (attempt uuid, officer uuid, enrollment uuid, csr_sha256 text, spki_sha256 text)
LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path = pg_catalog, pg_temp AS $$
BEGIN
    IF NOT felix3_initialize_setup.principal('felix3_local_setup_portal')
       OR session_digest !~ '^[0-9a-f]{64}$' THEN RETURN; END IF;
    RETURN QUERY
        SELECT a.id, a.officer, g.enrollment_id,
            encode(sha256(convert_to(g.public_csr_pem,'UTF8')),'hex'), g.spki_sha256
          FROM felix3_initialize_setup.eligible(b,false) p
          JOIN felix3_initialize_setup.attempt a ON (a.bound,a.officer) = (b,p.officer)
          JOIN sequent_backend.operational_setup_generation g ON
               (g.attempt,g.tenant_id,g.event_id,g.official,g.prepared_by,g.device_profile)
             = (a.id,b.tenant,b.event,b.official,a.officer,'softhsm-test')
          JOIN sequent_backend.operational_signing_official_enrollment e ON
               (e.id,e.tenant_id,e.actor,e.spki,e.device_profile)
             = (g.enrollment_id,b.tenant,b.official::text,g.spki_sha256,'softhsm-test')
         WHERE a.portal_session_sha256 = session_digest AND a.abandoned_at IS NULL
           AND a.released_at IS NOT NULL AND a.claimed_until IS NOT NULL AND p.officer <> b.official
           AND p.until_at > clock_timestamp() AND g.step IN ('completed','attached')
           AND g.fence IS NOT NULL AND g.evidence_sha256 IS NOT NULL
           AND g.spki_sha256 ~ '^[0-9a-f]{64}$' AND octet_length(g.public_csr_pem) BETWEEN 1 AND 16384
           AND e.nonexportable_confirmed AND e.revoked_at IS NULL
           AND sequent_backend.operational_signing_hsm_required(b.tenant)
           AND NOT EXISTS (SELECT FROM sequent_backend.signing_approval v
               WHERE (v.tenant_id,v.election_event_id,v.request_id,v.user_id)
                   = (b.tenant,b.event,b.request,b.official::text));
END;
$$;

-- A bounded in-process candidate is never enough to show a nominee to an
-- officer. Recheck 2250's actual source, including the independent officer,
-- both sets of Post labels, unchanged request and open authorization window.
CREATE FUNCTION felix3_initialize_bridge.officer_scope(
    b felix3_initialize_setup.binding, ticket uuid, who uuid, ol text[], nl text[], authenticated timestamptz
) RETURNS TABLE (request_id uuid, official_id uuid, election_id uuid, payload_sha256 text, certificate uuid)
LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path = pg_catalog, pg_temp AS $$
BEGIN
    IF felix3_initialize_attachment.officer_service() IS NOT TRUE
       OR NOT (authenticated <= clock_timestamp() AND clock_timestamp() < authenticated + interval '60 seconds')
       THEN RETURN; END IF;
    RETURN QUERY SELECT b.request, b.official, b.post, b.payload_sha256, g.certificate
        FROM felix3_initialize_attachment.generation(b,ticket,who,ol,nl) g
        WHERE clock_timestamp() < least(g.deadline, authenticated + interval '60 seconds');
END;
$$;

-- Local one-root profile: exactly one currently valid event-scoped staff
-- authority and one of its CRLs. Ambiguity, stale data, replacement or a
-- malformed source means no selection. 2250 independently locks the actual
-- trust snapshot and validates the genuine leaf and CRL before any write.
CREATE FUNCTION felix3_initialize_bridge.issuer_pins(
    b felix3_initialize_setup.binding, ticket uuid, who uuid, ol text[], nl text[], authenticated timestamptz
) RETURNS TABLE (issuer uuid, root_sha256 text, crl_sha256 text)
LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path = pg_catalog, pg_temp AS $$
BEGIN
    IF NOT EXISTS (SELECT FROM felix3_initialize_bridge.officer_scope(b,ticket,who,ol,nl,authenticated))
       THEN RETURN; END IF;
    RETURN QUERY WITH roots AS MATERIALIZED (
        SELECT ca.id, ca.fingerprint_sha256 FROM sequent_backend.certificate_authority ca
        WHERE (ca.tenant_id,ca.election_event_id) = (b.tenant,b.event)
          AND ca.purpose = 'staff-signatures' AND ca.not_before <= clock_timestamp()
          AND ca.not_after > clock_timestamp() LIMIT 2
    ), candidate AS MATERIALIZED (
        SELECT r.id, r.fingerprint_sha256, encode(sha256(l.der),'hex') AS crl
        FROM roots r JOIN sequent_backend.staff_crl l
          ON (l.tenant_id,l.election_event_id,l.issuer_id,l.issuer_fingerprint)
           = (b.tenant,b.event,r.id,r.fingerprint_sha256)
        WHERE (SELECT count(*) FROM roots) = 1 AND octet_length(l.der) BETWEEN 1 AND 4194304 LIMIT 2
    )
    SELECT k.id, k.fingerprint_sha256::text, k.crl FROM candidate k
     CROSS JOIN LATERAL felix3_initialize_attachment.trust(b,jsonb_build_object(
        'issuer',k.id,'root_sha256',k.fingerprint_sha256,'crl_sha256',k.crl)) t
     WHERE (SELECT count(*) FROM candidate) = 1
       AND clock_timestamp() < least(t.deadline,authenticated + interval '60 seconds');
END;
$$;

REVOKE ALL ON ALL FUNCTIONS IN SCHEMA felix3_initialize_bridge FROM PUBLIC;
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_proc p
        CROSS JOIN LATERAL aclexplode(coalesce(p.proacl,acldefault('f',p.proowner))) a
        WHERE p.pronamespace = 'felix3_initialize_bridge'::regnamespace AND a.grantee <> p.proowner)
       THEN RAISE EXCEPTION 'unexpected local bridge privileges'; END IF;
END;
$$;
RESET ROLE;
