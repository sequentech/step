-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-- Restore 2250's original authority. No enrollment or registration is removed.
SET LOCAL ROLE sequent_os_ledger_api_owner;
CREATE OR REPLACE FUNCTION felix3_initialize_attachment.generation(
    b felix3_initialize_setup.binding, ticket uuid, who uuid, officer_labels text[], nominee_labels text[]
) RETURNS TABLE (enrollment uuid, spki text, csr text, certificate uuid, fingerprint text,
                 attached_at timestamptz, deadline timestamptz)
LANGUAGE plpgsql STRICT SET search_path = pg_catalog, pg_temp AS $$
DECLARE
    permit record;
    g sequent_backend.operational_setup_generation%ROWTYPE;
    pin record;
    original text;
BEGIN
    IF NOT EXISTS (SELECT FROM felix3_initialize_setup.attempt a
        WHERE (a.id,a.bound,a.officer) = (ticket,b,who) AND a.abandoned_at IS NULL
          AND a.released_at IS NOT NULL AND a.claimed_until IS NOT NULL) THEN RETURN; END IF;
    SELECT * INTO permit FROM felix3_initialize_setup.eligible(b,true);
    IF NOT FOUND OR permit.officer <> who OR who = b.official THEN RETURN; END IF;
    -- Both verified sessions must see the request's frozen AND live Post labels.
    IF NOT felix3_initialize_attachment.post_access(b,officer_labels,nominee_labels) THEN RETURN; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'operational-signing-official:' || b.tenant::text || ':' || b.official::text,0));
    SELECT s.* INTO g FROM sequent_backend.operational_setup_generation s
      JOIN felix3_initialize_setup.attempt a ON a.id = s.attempt
      JOIN sequent_backend.operational_signing_official_enrollment e
        ON (e.tenant_id,e.actor,e.id,e.spki,e.device_profile)
         = (s.tenant_id,s.official::text,s.enrollment_id,s.spki_sha256,s.device_profile)
      JOIN sequent_backend.operational_setup_installation i
        ON (i.tenant_id,i.device_profile) = (s.tenant_id,s.device_profile)
      WHERE (s.attempt,s.tenant_id,s.event_id,s.official,s.prepared_by,s.device_profile)
          = (ticket,b.tenant,b.event,b.official,who,'softhsm-test')
        AND (a.bound,a.officer) = (b,who) AND a.abandoned_at IS NULL
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
DROP FUNCTION felix3_initialize_bridge.nominee_recovery(felix3_initialize_setup.binding,text);
DROP FUNCTION felix3_initialize_bridge.recovery_origin(felix3_initialize_setup.binding);
RESET ROLE;
