-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

GRANT CREATE ON SCHEMA sequent_backend TO sequent_os_ledger_api_owner;

-- An authenticated setup request is scoped by Harvest and independently
-- matched to the same live pending Post action and digest in the database.
-- This is read-only. No and unknown states both fail closed.
CREATE FUNCTION sequent_backend.operational_initialize_scoped_setup_status(
    p_tenant uuid, p_event uuid, p_request uuid, p_post uuid, p_official uuid, p_hash text
) RETURNS text LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT CASE
        -- Certificate readiness is not bound to the historical setup
        -- request. Reuse the broker's enrollment/certificate predicate on
        -- the current authorized request and installed backend, without
        -- exposing another request's attempt or bypassing live CA proof.
        WHEN EXISTS (
            SELECT FROM sequent_backend.operational_initialize_current(
                p_tenant,p_event,p_request,p_official::text,p_hash,i.device_profile
            )
        ) THEN 'certificate-on-file'
        WHEN g.step = 'prepared' AND g.claim_deadline > statement_timestamp()
            AND a.officer = g.prepared_by AND a.eligible_until > statement_timestamp()
            AND i.device_profile = g.device_profile
            AND NOT EXISTS (
                SELECT FROM sequent_backend.operational_signing_official_enrollment AS e
                WHERE (e.tenant_id,e.actor) = (p_tenant,p_official::text)
            ) THEN 'setup-retry-available'
        WHEN g.step = 'claimed' AND g.device_deadline > statement_timestamp() THEN 'setup-pending'
        WHEN g.step = 'completed'
            AND sequent_backend.operational_official_setup_status(p_tenant,p_event,p_official::text)
                = 'certificate-not-attached' THEN 'setup-pending'
        WHEN g.attempt IS NOT NULL THEN 'setup-recovery-required'
        -- Do not expose another request or an attempt ID. This generic state
        -- only applies to this official's stranded setup for the same Post.
        -- A still-live operation on another request remains unavailable.
        WHEN EXISTS (
            SELECT FROM sequent_backend.operational_initialize_self_setup AS previous
            JOIN sequent_backend.operational_setup_generation AS old
              ON (old.attempt,old.tenant_id,old.event_id,old.official)
               = (previous.attempt,previous.tenant_id,previous.event_id,previous.official)
            LEFT JOIN sequent_backend.signing_request AS original
              ON (original.tenant_id,original.election_event_id,original.id,original.election_id,original.payload_sha256)
               = (previous.tenant_id,previous.event_id,previous.request_id,previous.post_id,previous.payload_sha256)
            LEFT JOIN sequent_backend.operational_initialize_setup_roster AS roster
              ON (roster.tenant_id,roster.event_id,roster.official,roster.officer)
               = (previous.tenant_id,previous.event_id,previous.official,old.prepared_by)
            WHERE (previous.tenant_id,previous.event_id,previous.post_id,previous.official)
                = (p_tenant,p_event,p_post,p_official)
              AND previous.request_id <> p_request
              AND (
                    old.step = 'attached'
                 OR i.device_profile IS DISTINCT FROM old.device_profile
                 OR roster.eligible_until IS NULL OR roster.eligible_until <= statement_timestamp()
                 OR (old.step = 'prepared' AND old.claim_deadline <= statement_timestamp())
                 OR (old.step = 'claimed' AND old.device_deadline <= statement_timestamp())
                 OR (old.step = 'completed' AND
                    sequent_backend.operational_official_setup_status(p_tenant,p_event,p_official::text)
                        IS DISTINCT FROM 'certificate-not-attached')
                 OR original.id IS NULL OR original.status <> 'waiting'
                 OR original.action <> 'initialize-voting'
                 OR original.executed_at IS NOT NULL
                 OR original.expires_at IS NULL OR original.expires_at <= statement_timestamp()
                 OR original.document_id IS NOT NULL OR original.document_sha256 IS NOT NULL
                 OR encode(sha256(convert_to(original.canonical_payload,'UTF8')),'hex') <> previous.payload_sha256
              )
        ) THEN 'setup-recovery-required'
        -- An independently attached credential with relevant Post scope
        -- but no positive readiness above also needs an operator. Do not
        -- disclose even a historical attachment for a different Post.
        WHEN EXISTS (
            SELECT FROM sequent_backend.operational_signing_official_certificate AS bound
            JOIN sequent_backend.staff_certificate AS cert
              ON (cert.tenant_id,cert.election_event_id,cert.id,cert.user_id)
               = (bound.tenant_id,bound.event_id,bound.certificate_id,bound.actor)
            WHERE (bound.tenant_id,bound.event_id,bound.actor) = (p_tenant,p_event,p_official::text)
              AND (cert.election_id IS NULL OR cert.election_id = p_post)
        ) THEN 'setup-recovery-required'
        WHEN sequent_backend.operational_initialize_self_setup_status(p_tenant,p_event,p_official)
            = 'setup-available' THEN 'setup-available'
        ELSE 'unavailable'
    END
    FROM sequent_backend.signing_request AS r
    LEFT JOIN sequent_backend.operational_initialize_self_setup AS s
        ON (s.tenant_id,s.event_id,s.request_id,s.post_id,s.official,s.payload_sha256)
         = (r.tenant_id,r.election_event_id,r.id,r.election_id,p_official,r.payload_sha256)
    LEFT JOIN sequent_backend.operational_setup_generation AS g
        ON (g.attempt,g.tenant_id,g.event_id,g.official)
         = (s.attempt,s.tenant_id,s.event_id,s.official)
    LEFT JOIN sequent_backend.operational_initialize_setup_roster AS a
        ON (a.tenant_id,a.event_id,a.official) = (s.tenant_id,s.event_id,s.official)
    LEFT JOIN sequent_backend.operational_setup_installation AS i
        ON i.tenant_id = r.tenant_id
    WHERE (r.tenant_id,r.election_event_id,r.id,r.election_id,r.payload_sha256)
        = (p_tenant,p_event,p_request,p_post,p_hash)
      AND p_official IS NOT NULL AND p_official <> '00000000-0000-0000-0000-000000000000'
      AND r.status = 'waiting' AND r.action = 'initialize-voting'
      AND r.expires_at > statement_timestamp()
      AND r.document_id IS NULL AND r.document_sha256 IS NULL
      AND encode(sha256(convert_to(r.canonical_payload,'UTF8')),'hex') = p_hash
      AND sequent_backend.operational_signing_hsm_required(p_tenant);
$$;

-- A new explicit user action with verified fresh Gold can deliver ONLY the
-- original attempt, while it is still unclaimed. It never extends deadlines.
-- Both this path and the device CAS serialize on the same request row.
-- A delivery racing the claim is harmless: only the CAS winner invokes a key.
CREATE FUNCTION sequent_backend.operational_initialize_redispatch_self_setup(
    p_tenant uuid, p_event uuid, p_request uuid, p_post uuid, p_official uuid,
    p_hash text, p_auth_time timestamptz
) RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
DECLARE
    result uuid;
BEGIN
    PERFORM pg_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment', 0));
    PERFORM FROM sequent_backend.signing_request AS r
        WHERE (r.tenant_id,r.election_event_id,r.id,r.election_id,r.payload_sha256)
            = (p_tenant,p_event,p_request,p_post,p_hash)
        FOR UPDATE;
    IF NOT FOUND THEN RETURN NULL; END IF;
    IF p_auth_time IS NULL OR NOT isfinite(p_auth_time)
       OR p_auth_time > clock_timestamp() OR p_auth_time <= clock_timestamp() - interval '60 seconds'
       OR sequent_backend.operational_initialize_scoped_setup_status(
           p_tenant,p_event,p_request,p_post,p_official,p_hash
       ) IS DISTINCT FROM 'setup-retry-available' THEN
        RETURN NULL;
    END IF;
    SELECT s.attempt INTO result
    FROM sequent_backend.operational_initialize_self_setup AS s
    JOIN sequent_backend.operational_setup_generation AS g ON g.attempt = s.attempt
    JOIN sequent_backend.signing_request AS r
      ON (r.tenant_id,r.election_event_id,r.id) = (s.tenant_id,s.event_id,s.request_id)
    JOIN sequent_backend.operational_initialize_setup_roster AS a
      ON (a.tenant_id,a.event_id,a.official,a.officer) = (s.tenant_id,s.event_id,s.official,g.prepared_by)
    WHERE (s.tenant_id,s.event_id,s.request_id,s.post_id,s.official,s.payload_sha256)
        = (p_tenant,p_event,p_request,p_post,p_official,p_hash)
      AND g.step = 'prepared' AND g.claim_deadline > clock_timestamp()
      AND r.expires_at > clock_timestamp() AND a.eligible_until > clock_timestamp();
    RETURN result;
END;
$$;

ALTER FUNCTION sequent_backend.operational_initialize_scoped_setup_status(uuid,uuid,uuid,uuid,uuid,text)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_initialize_redispatch_self_setup(uuid,uuid,uuid,uuid,uuid,text,timestamptz)
    OWNER TO sequent_os_ledger_api_owner;
REVOKE ALL ON FUNCTION
    sequent_backend.operational_initialize_scoped_setup_status(uuid,uuid,uuid,uuid,uuid,text),
    sequent_backend.operational_initialize_redispatch_self_setup(uuid,uuid,uuid,uuid,uuid,text,timestamptz)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION
    sequent_backend.operational_initialize_scoped_setup_status(uuid,uuid,uuid,uuid,uuid,text),
    sequent_backend.operational_initialize_redispatch_self_setup(uuid,uuid,uuid,uuid,uuid,text,timestamptz)
    TO sequent_os_approve;
REVOKE CREATE ON SCHEMA sequent_backend FROM sequent_os_ledger_api_owner;
