-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Private operator provisioning. Enabling self-setup neither adds a signing
-- permission nor trusts any certificate; attaching the CA certificate remains
-- the distinct security officer's existing operation.
CREATE TABLE sequent_backend.operational_initialize_setup_roster (
    tenant_id uuid NOT NULL,
    event_id uuid NOT NULL,
    official uuid NOT NULL,
    officer uuid NOT NULL,
    eligible_until timestamptz NOT NULL CHECK (isfinite(eligible_until)),
    PRIMARY KEY (tenant_id,event_id,official),
    FOREIGN KEY (tenant_id,event_id) REFERENCES
        sequent_backend.election_event (tenant_id,id) ON DELETE CASCADE,
    CHECK (official <> officer AND
           '00000000-0000-0000-0000-000000000000' NOT IN (official,officer))
);

-- No replacement or replay even when dispatch status is indeterminate.
CREATE TABLE sequent_backend.operational_initialize_self_setup (
    attempt uuid PRIMARY KEY REFERENCES
        sequent_backend.operational_setup_generation(attempt),
    tenant_id uuid NOT NULL,
    event_id uuid NOT NULL,
    official uuid NOT NULL,
    request_id uuid NOT NULL,
    post_id uuid NOT NULL,
    payload_sha256 text NOT NULL CHECK (payload_sha256 ~ '^[0-9a-f]{64}$'),
    auth_time timestamptz NOT NULL,
    UNIQUE (tenant_id,official)
);

REVOKE ALL ON TABLE sequent_backend.operational_initialize_setup_roster,
    sequent_backend.operational_initialize_self_setup FROM PUBLIC;
GRANT CREATE ON SCHEMA sequent_backend TO sequent_os_ledger_api_owner, sequent_os_ledger_data_owner;
ALTER TABLE sequent_backend.operational_initialize_setup_roster OWNER TO sequent_os_ledger_data_owner;
ALTER TABLE sequent_backend.operational_initialize_self_setup OWNER TO sequent_os_ledger_data_owner;
GRANT SELECT ON TABLE sequent_backend.operational_initialize_setup_roster TO sequent_os_ledger_api_owner;
GRANT SELECT, INSERT ON TABLE sequent_backend.operational_initialize_self_setup TO sequent_os_ledger_api_owner;

CREATE FUNCTION sequent_backend.operational_initialize_self_setup_status(
    p_tenant uuid, p_event uuid, p_official uuid
) RETURNS text LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT CASE
        WHEN phase = 'certificate-on-file' THEN phase
        WHEN EXISTS (SELECT FROM sequent_backend.operational_initialize_self_setup
                     WHERE (tenant_id, event_id, official) = (p_tenant,p_event,p_official))
             OR phase IN ('certificate-not-attached', 'certificate-needs-review')
            THEN 'setup-pending'
        WHEN phase = 'key-not-enrolled'
          AND EXISTS (SELECT FROM sequent_backend.operational_setup_installation WHERE tenant_id = p_tenant)
          AND NOT EXISTS (SELECT FROM sequent_backend.operational_setup_generation
                          WHERE (tenant_id,official) = (p_tenant,p_official))
          AND NOT EXISTS (SELECT FROM sequent_backend.operational_signing_official_enrollment
                          WHERE (tenant_id,actor) = (p_tenant,p_official::text))
          AND EXISTS (SELECT FROM sequent_backend.operational_initialize_setup_roster
                      WHERE (tenant_id, event_id, official) = (p_tenant,p_event,p_official)
                        AND eligible_until > statement_timestamp())
            THEN 'setup-available'
        ELSE 'unavailable'
    END
    FROM sequent_backend.operational_official_setup_status(p_tenant,p_event,p_official::text) AS phase;
$$;

CREATE FUNCTION sequent_backend.operational_initialize_start_self_setup(
    p_tenant uuid, p_event uuid, p_request uuid, p_post uuid, p_official uuid,
    p_hash text, p_auth_time timestamptz, p_attempt uuid
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
DECLARE
    officer_id uuid;
    now_at timestamptz := clock_timestamp();
BEGIN
    IF p_official IS NULL OR p_post IS NULL OR p_attempt IS NULL
       OR p_auth_time IS NULL OR NOT isfinite(p_auth_time)
       OR p_auth_time > now_at OR p_auth_time <= now_at - interval '60 seconds'
       OR NOT sequent_backend.operational_signing_hsm_required(p_tenant) THEN
        RETURN false;
    END IF;
    -- Same row and order as the app: approval request first, then official.
    PERFORM FROM sequent_backend.signing_request AS r
        WHERE (r.tenant_id,r.election_event_id,r.id,r.election_id) = (p_tenant,p_event,p_request,p_post)
          AND r.status = 'waiting' AND r.action = 'initialize-voting'
          AND r.expires_at > now_at AND r.document_id IS NULL AND r.document_sha256 IS NULL
          AND r.payload_sha256 = p_hash
          AND encode(sha256(convert_to(r.canonical_payload,'UTF8')),'hex') = p_hash
        FOR UPDATE;
    IF NOT FOUND THEN RETURN false; END IF;
    SELECT officer INTO officer_id FROM sequent_backend.operational_initialize_setup_roster
        WHERE (tenant_id,event_id,official) = (p_tenant,p_event,p_official)
          AND eligible_until > clock_timestamp();
    IF NOT FOUND OR p_auth_time <= clock_timestamp() - interval '60 seconds' THEN
        RETURN false;
    END IF;
    IF NOT sequent_backend.operational_setup_prepare_generation(
        p_tenant,p_event,p_official,officer_id,p_attempt) THEN RETURN false; END IF;
    INSERT INTO sequent_backend.operational_initialize_self_setup
        (attempt,tenant_id,event_id,official,request_id,post_id,payload_sha256,auth_time)
    VALUES (p_attempt,p_tenant,p_event,p_official,p_request,p_post,p_hash,p_auth_time);
    RETURN true;
END;
$$;
-- Independently recheck the live, exact browser intent before device use.
CREATE FUNCTION sequent_backend.operational_initialize_claim_self_setup(
    p_tenant uuid, p_event uuid, p_official uuid, p_attempt uuid
) RETURNS TABLE (fence uuid, deadline timestamptz)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
BEGIN
    PERFORM pg_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment', 0));
    PERFORM FROM sequent_backend.signing_request AS r
        JOIN sequent_backend.operational_initialize_self_setup AS s
          ON (r.tenant_id,r.election_event_id,r.id,r.election_id,r.payload_sha256)
           = (s.tenant_id,s.event_id,s.request_id,s.post_id,s.payload_sha256)
        JOIN sequent_backend.operational_initialize_setup_roster AS a
          ON (a.tenant_id,a.event_id,a.official) = (s.tenant_id,s.event_id,s.official)
        JOIN sequent_backend.operational_setup_generation AS g
          ON g.attempt = s.attempt AND g.prepared_by = a.officer
        WHERE (s.tenant_id,s.event_id,s.official,s.attempt) = (p_tenant,p_event,p_official,p_attempt)
          AND r.status = 'waiting' AND r.action = 'initialize-voting'
          AND r.expires_at > clock_timestamp() AND a.eligible_until > clock_timestamp()
          AND r.document_id IS NULL AND r.document_sha256 IS NULL
          AND encode(sha256(convert_to(r.canonical_payload,'UTF8')),'hex') = s.payload_sha256
        FOR UPDATE OF r;
    IF NOT FOUND THEN RETURN; END IF;
    RETURN QUERY SELECT c.fence,c.deadline
      FROM sequent_backend.operational_setup_claim_generation(p_tenant,p_event,p_official,p_attempt) AS c;
END;
$$;
ALTER FUNCTION sequent_backend.operational_initialize_claim_self_setup(uuid,uuid,uuid,uuid)
    OWNER TO sequent_os_ledger_api_owner;
REVOKE ALL ON FUNCTION sequent_backend.operational_initialize_claim_self_setup(uuid,uuid,uuid,uuid)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION sequent_backend.operational_initialize_claim_self_setup(uuid,uuid,uuid,uuid)
    TO sequent_os_setup_device;

ALTER FUNCTION sequent_backend.operational_initialize_self_setup_status(uuid,uuid,uuid)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_initialize_start_self_setup(uuid,uuid,uuid,uuid,uuid,text,timestamptz,uuid)
    OWNER TO sequent_os_ledger_api_owner;
GRANT EXECUTE ON FUNCTION
    sequent_backend.operational_setup_prepare_generation(uuid,uuid,uuid,uuid,uuid),
    sequent_backend.operational_official_setup_status(uuid,uuid,text)
    TO sequent_os_ledger_api_owner;
REVOKE ALL ON FUNCTION
    sequent_backend.operational_initialize_self_setup_status(uuid,uuid,uuid),
    sequent_backend.operational_initialize_start_self_setup(uuid,uuid,uuid,uuid,uuid,text,timestamptz,uuid)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION
    sequent_backend.operational_initialize_self_setup_status(uuid,uuid,uuid),
    sequent_backend.operational_initialize_start_self_setup(uuid,uuid,uuid,uuid,uuid,text,timestamptz,uuid)
    TO sequent_os_approve;
REVOKE CREATE ON SCHEMA sequent_backend FROM sequent_os_ledger_api_owner, sequent_os_ledger_data_owner;
