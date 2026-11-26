-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Apply transactionally after 2100. The installer supplies service identities
-- separately. This migration creates no login, memberships, permits or tenant data.
CREATE SCHEMA felix3_initialize_setup AUTHORIZATION sequent_os_ledger_api_owner;
REVOKE ALL ON SCHEMA felix3_initialize_setup FROM PUBLIC;
GRANT USAGE, CREATE ON SCHEMA felix3_initialize_setup TO sequent_os_ledger_data_owner;

CREATE TYPE felix3_initialize_setup.binding AS (
    installation uuid, tenant uuid, event uuid, post uuid, publication uuid,
    request uuid, official uuid, payload_sha256 text COLLATE "C"
);
ALTER TYPE felix3_initialize_setup.binding OWNER TO sequent_os_ledger_api_owner;
REVOKE ALL ON TYPE felix3_initialize_setup.binding FROM PUBLIC;
GRANT USAGE ON TYPE felix3_initialize_setup.binding TO sequent_os_ledger_data_owner;

-- An operator nominates each eligible official and a DIFFERENT responsible
-- officer for one installed publication. The signer's live permissions are
-- independently checked against Keycloak by Harvest before every click/status.
CREATE TABLE felix3_initialize_setup.permit (
    installation uuid NOT NULL, tenant uuid NOT NULL, event uuid NOT NULL,
    post uuid NOT NULL, publication uuid NOT NULL, official uuid NOT NULL,
    officer uuid NOT NULL, eligible_until timestamptz NOT NULL CHECK (isfinite(eligible_until)),
    PRIMARY KEY (installation, tenant, event, post, publication, official),
    CHECK (official <> officer AND '00000000-0000-0000-0000-000000000000'::uuid NOT IN
           (installation, tenant, event, post, publication, official, officer))
);
CREATE TABLE felix3_initialize_setup.attempt (
    id uuid PRIMARY KEY CHECK (id <> '00000000-0000-0000-0000-000000000000'),
    bound felix3_initialize_setup.binding NOT NULL CHECK (bound IS NOT NULL),
    officer uuid NOT NULL,
    nonce_sha256 text COLLATE "C" NOT NULL UNIQUE CHECK (nonce_sha256 ~ '^[0-9a-f]{64}$'),
    opened_at timestamptz NOT NULL, expires_at timestamptz NOT NULL CHECK (opened_at < expires_at),
    exchange_at timestamptz, auth_time bigint, gold_until timestamptz, released_at timestamptz,
    claimed_until timestamptz, abandoned_at timestamptz,
    CHECK ((auth_time IS NULL) = (gold_until IS NULL)),
    CHECK (released_at IS NULL OR abandoned_at IS NULL),
    CHECK (auth_time IS NULL OR exchange_at IS NOT NULL),
    CHECK (released_at IS NULL OR (gold_until IS NOT NULL AND released_at < gold_until)),
    CHECK (claimed_until IS NULL OR (released_at IS NOT NULL AND claimed_until <= gold_until)),
    CHECK (gold_until IS NULL OR gold_until <= expires_at)
);
CREATE UNIQUE INDEX one_local_setup_per_official ON felix3_initialize_setup.attempt
    (((bound).tenant), ((bound).official)) WHERE abandoned_at IS NULL;
ALTER TABLE felix3_initialize_setup.permit OWNER TO sequent_os_ledger_data_owner;
ALTER TABLE felix3_initialize_setup.attempt OWNER TO sequent_os_ledger_data_owner;
REVOKE ALL ON ALL TABLES IN SCHEMA felix3_initialize_setup FROM PUBLIC;
GRANT SELECT, UPDATE (eligible_until) ON felix3_initialize_setup.permit TO sequent_os_ledger_api_owner;
GRANT SELECT, INSERT, UPDATE (exchange_at, auth_time, gold_until, released_at, claimed_until, abandoned_at)
    ON felix3_initialize_setup.attempt TO sequent_os_ledger_api_owner;
REVOKE CREATE ON SCHEMA felix3_initialize_setup FROM sequent_os_ledger_data_owner;

SET LOCAL ROLE sequent_os_ledger_api_owner;

-- Each public entry also checks the real authenticated socket login. SET ROLE
-- cannot impersonate a service. A login with ordinary setup/device/approval
-- memberships or direct operational ledger access is unsuitable for this role.
CREATE FUNCTION felix3_initialize_setup.principal(expected name)
RETURNS boolean LANGUAGE sql STABLE SET search_path = pg_catalog, pg_temp AS $$
    SELECT EXISTS (
        SELECT FROM pg_roles AS r WHERE r.rolname = session_user AND r.rolname = expected
          AND inet_client_addr() IS NULL AND r.rolcanlogin
          AND NOT (r.rolsuper OR r.rolcreatedb OR r.rolcreaterole OR r.rolreplication OR r.rolbypassrls)
          AND NOT EXISTS (SELECT FROM pg_auth_members m WHERE m.member = r.oid)
          AND NOT has_database_privilege(r.oid, current_database(), 'CREATE')
          AND NOT has_schema_privilege(r.oid, 'sequent_backend', 'CREATE')
          AND NOT has_schema_privilege(r.oid, 'felix3_initialize_setup', 'CREATE')
          AND NOT EXISTS (
              SELECT FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
              WHERE (n.nspname = 'felix3_initialize_setup' OR (n.nspname = 'sequent_backend'
                  AND (c.relname ~ '^(operational_|signing_|staff_)' OR c.relname = 'certificate_authority')))
                AND c.relkind IN ('r','p','v','m','f')
                AND (has_table_privilege(r.oid,c.oid,'SELECT,INSERT,UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER')
                     OR has_any_column_privilege(r.oid,c.oid,'SELECT,INSERT,UPDATE,REFERENCES')))
          AND NOT EXISTS (
              SELECT FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace
              WHERE n.nspname = 'sequent_backend' AND p.proname ~ '^operational_'
                AND p.prosecdef AND p.provolatile = 'v'
                AND has_schema_privilege(r.oid,n.oid,'USAGE') AND has_function_privilege(r.oid,p.oid,'EXECUTE'))
    );
$$;

-- The ordinary Initialize request freezes subject.publication_id. Match the
-- original UTF-8 bytes, the stored subject, and every signed scope identifier.
CREATE FUNCTION felix3_initialize_setup.payload_matches(
    b felix3_initialize_setup.binding, stored_subject jsonb, canonical text, request_expiry timestamptz
) RETURNS boolean LANGUAGE sql STABLE STRICT SET search_path = pg_catalog, pg_temp AS $$
    WITH document AS (
        SELECT CASE WHEN octet_length(canonical) BETWEEN 1 AND 524288
             AND canonical IS JSON OBJECT WITH UNIQUE KEYS AND pg_input_is_valid(canonical, 'jsonb')
             THEN canonical::jsonb END AS p
    ), expiry AS (
        SELECT p, regexp_replace(p->>'expires_at','[.][0-9]+','') AS signed_second FROM document
    )
    SELECT coalesce(encode(sha256(convert_to(canonical,'UTF8')),'hex') = (b).payload_sha256
         AND p @> jsonb_build_object('domain','step-signing/v1','action','initialize-voting',
               'tenant_id',(b).tenant,'election_event_id',(b).event,'election_id',(b).post,'request_id',(b).request)
         AND p->'subject' = stored_subject AND stored_subject = jsonb_build_object('publication_id',(b).publication)
         AND CASE WHEN p->>'expires_at' ~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}([.][0-9]+)?(Z|[+-][0-9]{2}:[0-9]{2})$'
                    AND pg_input_is_valid(signed_second,'timestamptz')
                  THEN signed_second::timestamptz = date_trunc('second',request_expiry) ELSE false END,
         false) FROM expiry;
$$;

-- Mutations hold the same policy/event/request locks as Initialize and pin
-- the current publication witness and the operator permit through commit.
-- Status runs the identical read predicates without acquiring write locks.
CREATE FUNCTION felix3_initialize_setup.eligible(b felix3_initialize_setup.binding, lock_rows boolean)
RETURNS TABLE (officer uuid, until_at timestamptz) LANGUAGE plpgsql
SET search_path = pg_catalog, pg_temp AS $$
BEGIN
    IF NOT (b IS NOT NULL) OR b.request = '00000000-0000-0000-0000-000000000000'
       OR b.payload_sha256 !~ '^[0-9a-f]{64}$' THEN RETURN; END IF;
    IF lock_rows THEN
        IF current_setting('transaction_isolation') <> 'read committed' THEN RETURN; END IF;
        PERFORM pg_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment',0));
        PERFORM pg_advisory_xact_lock(hashtextextended('signing-event:' || b.tenant::text || ':' || b.event::text,0));
        PERFORM FROM sequent_backend.signing_request r
          WHERE (r.tenant_id,r.election_event_id,r.election_id,r.id) = (b.tenant,b.event,b.post,b.request)
          FOR UPDATE OF r;
        IF NOT FOUND THEN RETURN; END IF;
        PERFORM FROM sequent_backend.operational_initialize_local_authority a
          WHERE (a.installation_id,a.tenant_id,a.event_id,a.post_id,a.publication_id,a.device_profile)
              = (b.installation,b.tenant,b.event,b.post,b.publication,'softhsm-test')
          LIMIT 1 FOR SHARE OF a NOWAIT;
        IF NOT FOUND THEN RETURN; END IF;
        PERFORM FROM felix3_initialize_setup.permit p
          WHERE (p.installation,p.tenant,p.event,p.post,p.publication,p.official)
              = (b.installation,b.tenant,b.event,b.post,b.publication,b.official) FOR SHARE OF p;
        IF NOT FOUND THEN RETURN; END IF;
    END IF;
    RETURN QUERY
      SELECT p.officer, least(p.eligible_until,date_trunc('second',r.expires_at))
      FROM felix3_initialize_setup.permit p JOIN sequent_backend.signing_request r
        ON (r.tenant_id,r.election_event_id,r.election_id,r.id) = (p.tenant,p.event,p.post,b.request)
      WHERE (p.installation,p.tenant,p.event,p.post,p.publication,p.official)
          = (b.installation,b.tenant,b.event,b.post,b.publication,b.official)
        AND p.installation = sequent_backend.operational_signing_local_initialize_installation_id(
              b.tenant,b.event,b.post,b.publication,'softhsm-test')
        AND p.eligible_until > clock_timestamp()
        AND r.status = 'waiting' AND r.action = 'initialize-voting' AND r.executed_at IS NULL
        AND isfinite(r.expires_at) AND date_trunc('second',r.expires_at) > clock_timestamp()
        AND r.document_id IS NULL AND r.document_sha256 IS NULL AND r.payload_sha256 = b.payload_sha256
        AND felix3_initialize_setup.payload_matches(b,r.subject,r.canonical_payload,r.expires_at);
END;
$$;

-- Harvest calls only on the user's explicit setup click. The attempt and
-- nonce hash come from the setup-only ceremony, never from browser JSON.
CREATE FUNCTION felix3_initialize_setup.begin_setup(
    b felix3_initialize_setup.binding, challenge uuid, nonce_hash text
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path = pg_catalog, pg_temp AS $$
DECLARE
    permit record;
    opened timestamptz;
BEGIN
    IF NOT felix3_initialize_setup.principal('felix3_local_setup_portal')
       OR challenge = '00000000-0000-0000-0000-000000000000' OR nonce_hash !~ '^[0-9a-f]{64}$'
       THEN RETURN false; END IF;
    SELECT * INTO permit FROM felix3_initialize_setup.eligible(b,true);
    IF NOT FOUND THEN RETURN false; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'operational-signing-official:' || b.tenant::text || ':' || b.official::text,0));
    opened := clock_timestamp();
    IF permit.until_at <= opened OR EXISTS (SELECT FROM sequent_backend.operational_setup_generation g
        WHERE (g.tenant_id,g.official) = (b.tenant,b.official))
       OR EXISTS (SELECT FROM sequent_backend.operational_signing_official_enrollment e
        WHERE (e.tenant_id,e.actor) = (b.tenant,b.official::text)) THEN RETURN false; END IF;
    -- A fresh login may replace a known unused challenge. Keep any uncertain
    -- code exchange and every released device attempt for reconciliation.
    UPDATE felix3_initialize_setup.attempt a SET abandoned_at = opened
      WHERE ((a.bound).tenant,(a.bound).official) = (b.tenant,b.official)
        AND a.abandoned_at IS NULL AND a.released_at IS NULL
        AND ((a.exchange_at IS NULL AND a.expires_at <= opened) OR a.gold_until <= opened);
    INSERT INTO felix3_initialize_setup.attempt (id,bound,officer,nonce_sha256,opened_at,expires_at)
      VALUES (challenge,b,permit.officer,nonce_hash,opened,least(permit.until_at,opened + interval '5 minutes'))
      ON CONFLICT DO NOTHING;
    IF FOUND THEN RETURN true; END IF;
    RETURN EXISTS (SELECT FROM felix3_initialize_setup.attempt a
      WHERE (a.id,a.bound,a.officer,a.nonce_sha256) = (challenge,b,permit.officer,nonce_hash)
        AND a.abandoned_at IS NULL AND a.exchange_at IS NULL AND a.expires_at > clock_timestamp());
END;
$$;

-- A redirect reads the durable challenge. A validated state/code return
-- must take it exactly once and commit before the private token exchange.
CREATE FUNCTION felix3_initialize_setup.gold_challenge(
    b felix3_initialize_setup.binding, challenge uuid, nonce_hash text, take_return boolean
) RETURNS TABLE (opened_at timestamptz, expires_at timestamptz)
LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path = pg_catalog, pg_temp AS $$
DECLARE
    permit record;
BEGIN
    IF NOT felix3_initialize_setup.principal('felix3_local_setup_gold') THEN RETURN; END IF;
    SELECT * INTO permit FROM felix3_initialize_setup.eligible(b,take_return);
    IF NOT FOUND THEN RETURN; END IF;
    IF take_return THEN
        RETURN QUERY UPDATE felix3_initialize_setup.attempt a SET exchange_at = clock_timestamp()
          WHERE (a.id,a.bound,a.officer,a.nonce_sha256) = (challenge,b,permit.officer,nonce_hash)
            AND a.abandoned_at IS NULL AND a.exchange_at IS NULL AND a.expires_at > clock_timestamp()
          RETURNING a.opened_at,least(a.expires_at,permit.until_at);
    ELSE
        RETURN QUERY SELECT a.opened_at,least(a.expires_at,permit.until_at) FROM felix3_initialize_setup.attempt a
          WHERE (a.id,a.bound,a.officer,a.nonce_sha256) = (challenge,b,permit.officer,nonce_hash)
            AND a.abandoned_at IS NULL AND a.exchange_at IS NULL AND a.expires_at > clock_timestamp();
    END IF;
END;
$$;

-- NumericDate inputs retain Keycloak's seconds resolution. Authentication
-- in the challenge's own second cannot prove it followed challenge creation.
CREATE FUNCTION felix3_initialize_setup.fresh_gold(
    opened bigint, authenticated bigint, issued bigint, expires bigint, observed bigint
) RETURNS boolean LANGUAGE sql IMMUTABLE STRICT SET search_path = pg_catalog, pg_temp AS $$
    SELECT opened < authenticated AND authenticated <= issued AND issued <= observed
       AND authenticated > observed - 60 AND observed < expires;
$$;

-- Only the confidential setup-code verifier may attest these signed claims.
-- Its pinned PAR/code exchange verifies RS256, the sole audience, Gold and
-- both required executions. Its login cannot open a ticket or claim a fence.
CREATE FUNCTION felix3_initialize_setup.confirm_gold(
    b felix3_initialize_setup.binding, challenge uuid, subject_id uuid, issuer text,
    client_id text, nonce_hash text, authenticated bigint, issued bigint, expires bigint
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path = pg_catalog, pg_temp AS $$
DECLARE
    permit record;
    observed bigint;
BEGIN
    IF NOT felix3_initialize_setup.principal('felix3_local_setup_gold') OR subject_id <> b.official
       OR issuer COLLATE "C" <> 'http://localhost:18090/realms/tenant-' || b.tenant::text
       OR client_id COLLATE "C" <> 'felix3-local-initialize-setup' THEN RETURN false; END IF;
    SELECT * INTO permit FROM felix3_initialize_setup.eligible(b,true);
    IF NOT FOUND THEN RETURN false; END IF;
    observed := floor(extract(epoch FROM clock_timestamp()))::bigint;
    UPDATE felix3_initialize_setup.attempt a
      SET auth_time = authenticated,
          gold_until = least(a.expires_at,permit.until_at,to_timestamp(least(expires,authenticated + 60)))
      WHERE (a.id,a.bound,a.officer,a.nonce_sha256) = (challenge,b,permit.officer,nonce_hash)
        AND a.exchange_at IS NOT NULL AND a.auth_time IS NULL AND a.abandoned_at IS NULL
        AND a.expires_at > clock_timestamp()
        AND felix3_initialize_setup.fresh_gold(floor(extract(epoch FROM a.opened_at))::bigint,
                                               authenticated,issued,expires,observed);
    RETURN FOUND;
END;
$$;

-- After the user returns from Gold, the portal rechecks the current signer
-- and accepts the explicit registration click. Redelivery retains all deadlines.
CREATE FUNCTION felix3_initialize_setup.release(b felix3_initialize_setup.binding, challenge uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path = pg_catalog, pg_temp AS $$
DECLARE
    permit record;
    clicked timestamptz;
BEGIN
    IF NOT felix3_initialize_setup.principal('felix3_local_setup_portal') THEN RETURN false; END IF;
    SELECT * INTO permit FROM felix3_initialize_setup.eligible(b,true);
    IF NOT FOUND THEN RETURN false; END IF;
    clicked := clock_timestamp();
    UPDATE felix3_initialize_setup.attempt a SET released_at = coalesce(a.released_at,clicked)
      WHERE (a.id,a.bound,a.officer) = (challenge,b,permit.officer)
        AND a.abandoned_at IS NULL AND a.claimed_until IS NULL AND a.gold_until > clicked AND permit.until_at > clicked
        AND NOT EXISTS (SELECT FROM sequent_backend.operational_setup_generation g
                        WHERE (g.tenant_id,g.official) = (b.tenant,b.official));
    RETURN FOUND;
END;
$$;

-- Both 1200 operations run in this ONE transaction. A prepared local
-- generation is never visible to the general 1200/1700 device claim paths.
CREATE FUNCTION felix3_initialize_setup.claim(b felix3_initialize_setup.binding, challenge uuid)
RETURNS TABLE (fence uuid, deadline timestamptz) LANGUAGE plpgsql SECURITY DEFINER STRICT
SET search_path = pg_catalog, pg_temp AS $$
DECLARE
    permit record;
    authorized_until timestamptz;
    generation record;
BEGIN
    IF NOT felix3_initialize_setup.principal('felix3_local_setup_device') THEN RETURN; END IF;
    SELECT * INTO permit FROM felix3_initialize_setup.eligible(b,true);
    IF NOT FOUND THEN RETURN; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'operational-signing-official:' || b.tenant::text || ':' || b.official::text,0));
    SELECT least(a.gold_until,permit.until_at) INTO authorized_until FROM felix3_initialize_setup.attempt a
      WHERE (a.id,a.bound,a.officer) = (challenge,b,permit.officer) AND a.abandoned_at IS NULL
        AND a.released_at IS NOT NULL AND a.claimed_until IS NULL AND a.gold_until > clock_timestamp() FOR UPDATE OF a;
    IF NOT FOUND OR authorized_until <= clock_timestamp() THEN RETURN; END IF;
    IF NOT sequent_backend.operational_setup_prepare_generation(
        b.tenant,b.event,b.official,permit.officer,challenge) THEN RETURN; END IF;
    SELECT * INTO generation FROM sequent_backend.operational_setup_claim_generation(
        b.tenant,b.event,b.official,challenge);
    IF NOT FOUND OR least(authorized_until,generation.deadline) <= clock_timestamp() THEN
        RAISE EXCEPTION 'local setup claim expired or lost' USING ERRCODE = '40001';
    END IF;
    UPDATE felix3_initialize_setup.attempt SET claimed_until = least(authorized_until,generation.deadline)
      WHERE id = challenge;
    RETURN QUERY SELECT generation.fence, least(authorized_until,generation.deadline);
END;
$$;

-- A device settles only the exact local attempt/fence with its own verified
-- public CSR and device evidence. Lost replies can replay identical evidence.
CREATE FUNCTION felix3_initialize_setup.complete(
    b felix3_initialize_setup.binding, challenge uuid, device_fence uuid,
    spki text, evidence text, csr text
) RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path = pg_catalog, pg_temp AS $$
DECLARE
    permit record;
    g sequent_backend.operational_setup_generation%ROWTYPE;
BEGIN
    IF NOT felix3_initialize_setup.principal('felix3_local_setup_device') THEN RETURN NULL; END IF;
    SELECT * INTO permit FROM felix3_initialize_setup.eligible(b,true);
    IF NOT FOUND THEN RETURN NULL; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'operational-signing-official:' || b.tenant::text || ':' || b.official::text,0));
    SELECT s.* INTO g FROM sequent_backend.operational_setup_generation s
      JOIN felix3_initialize_setup.attempt a ON a.id = s.attempt
      WHERE (a.id,a.bound,a.officer) = (challenge,b,permit.officer) AND a.claimed_until IS NOT NULL
        AND (s.tenant_id,s.event_id,s.official,s.prepared_by,s.fence,s.device_profile)
          = (b.tenant,b.event,b.official,permit.officer,device_fence,'softhsm-test')
      FOR UPDATE OF s;
    IF NOT FOUND THEN RETURN NULL; END IF;
    IF g.step IN ('completed','attached') AND (g.spki_sha256,g.evidence_sha256,g.public_csr_pem)
        = (spki,evidence,csr) THEN RETURN g.enrollment_id; END IF;
    IF g.step <> 'claimed' THEN RETURN NULL; END IF;
    RETURN sequent_backend.operational_setup_complete_generation(
        b.tenant,b.event,b.official,challenge,device_fence,spki,evidence,csr);
END;
$$;

CREATE FUNCTION felix3_initialize_setup.status(b felix3_initialize_setup.binding)
RETURNS text LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path = pg_catalog, pg_temp AS $$
DECLARE
    permit record;
    a felix3_initialize_setup.attempt%ROWTYPE;
    g sequent_backend.operational_setup_generation%ROWTYPE;
BEGIN
    IF NOT felix3_initialize_setup.principal('felix3_local_setup_portal') THEN RETURN 'unavailable'; END IF;
    SELECT * INTO permit FROM felix3_initialize_setup.eligible(b,false);
    IF NOT FOUND THEN RETURN 'unavailable'; END IF;
    IF EXISTS (SELECT FROM sequent_backend.operational_initialize_current(
        b.tenant,b.event,b.request,b.official::text,b.payload_sha256,'softhsm-test')) THEN
        RETURN 'certificate-on-file';
    END IF;
    SELECT * INTO a FROM felix3_initialize_setup.attempt t
      WHERE ((t.bound).tenant,(t.bound).official) = (b.tenant,b.official) AND t.abandoned_at IS NULL;
    SELECT * INTO g FROM sequent_backend.operational_setup_generation s
      WHERE (s.tenant_id,s.official) = (b.tenant,b.official);
    IF g.attempt IS NOT NULL THEN
        IF (a.id,a.bound,a.officer) IS DISTINCT FROM (g.attempt,b,permit.officer) THEN RETURN 'unavailable'; END IF;
        IF (g.step = 'claimed' AND a.claimed_until > clock_timestamp()) OR (g.step = 'completed'
            AND sequent_backend.operational_official_setup_status(b.tenant,b.event,b.official::text)
                = 'certificate-not-attached') THEN RETURN 'setup-pending'; END IF;
        RETURN 'setup-recovery-required';
    END IF;
    IF EXISTS (SELECT FROM sequent_backend.operational_signing_official_enrollment e
        WHERE (e.tenant_id,e.actor) = (b.tenant,b.official::text)) THEN RETURN 'unavailable'; END IF;
    IF a.id IS NULL OR (a.released_at IS NULL AND ((a.exchange_at IS NULL AND a.expires_at <= clock_timestamp())
          OR a.gold_until <= clock_timestamp())) THEN RETURN 'setup-available'; END IF;
    IF (a.bound,a.officer) IS DISTINCT FROM (b,permit.officer) THEN RETURN 'unavailable'; END IF;
    IF a.claimed_until IS NOT NULL THEN RETURN 'setup-recovery-required'; END IF;
    IF a.exchange_at IS NULL THEN RETURN 'waiting-for-setup-gold'; END IF;
    IF a.auth_time IS NULL AND a.expires_at > clock_timestamp() THEN RETURN 'setup-pending'; END IF;
    IF a.released_at IS NULL AND a.gold_until > clock_timestamp() THEN RETURN 'setup-confirmation-required'; END IF;
    IF a.gold_until > clock_timestamp() THEN RETURN 'setup-retry-available'; END IF;
    RETURN 'setup-recovery-required';
END;
$$;

REVOKE ALL ON ALL FUNCTIONS IN SCHEMA felix3_initialize_setup FROM PUBLIC;
RESET ROLE;

-- Reject custom default ACLs that would expose the private tables, helper
-- predicates or entry points before a reviewed installation grants access.
DO $$
BEGIN
    IF EXISTS (
        SELECT FROM pg_proc p CROSS JOIN LATERAL aclexplode(coalesce(p.proacl,acldefault('f',p.proowner))) g
        WHERE p.pronamespace = 'felix3_initialize_setup'::regnamespace AND g.grantee <> p.proowner
    ) OR EXISTS (
        SELECT FROM pg_class c CROSS JOIN LATERAL aclexplode(c.relacl) g
        WHERE c.relnamespace = 'felix3_initialize_setup'::regnamespace
          AND g.grantee NOT IN (c.relowner,'sequent_os_ledger_api_owner'::regrole)
    ) OR EXISTS (
        SELECT FROM pg_namespace n CROSS JOIN LATERAL aclexplode(n.nspacl) g
        WHERE n.oid = 'felix3_initialize_setup'::regnamespace
          AND g.grantee NOT IN (n.nspowner,'sequent_os_ledger_data_owner'::regrole)
    ) THEN RAISE EXCEPTION 'unexpected local setup default privileges'; END IF;
END;
$$;
