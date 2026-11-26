-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Apply as one migration transaction. A dedicated local broker login needs
-- the existing Initialize orchestrator and this marker as its two direct,
-- inherited, non-admin memberships. Provision logins separately.
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'sequent_os_initialize_local_broker') THEN
        BEGIN
            CREATE ROLE sequent_os_initialize_local_broker NOLOGIN NOINHERIT NOSUPERUSER
                NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
        EXCEPTION WHEN duplicate_object OR unique_violation THEN
            NULL;
        END;
    END IF;
    IF NOT EXISTS (
        SELECT FROM pg_catalog.pg_roles AS r
        WHERE r.rolname = 'sequent_os_initialize_local_broker'
          AND NOT (r.rolcanlogin OR r.rolinherit OR r.rolsuper OR r.rolcreatedb
                   OR r.rolcreaterole OR r.rolreplication OR r.rolbypassrls)
          AND NOT EXISTS (SELECT FROM pg_catalog.pg_auth_members AS m WHERE m.member = r.oid)
          AND NOT EXISTS (SELECT FROM pg_catalog.pg_auth_members AS m
                          JOIN pg_catalog.pg_roles AS grantee ON grantee.oid = m.member
                          WHERE m.roleid = r.oid AND NOT grantee.rolcanlogin
                            AND (m.inherit_option OR m.set_option))
    ) THEN
        RAISE EXCEPTION 'invalid local Initialize broker group';
    END IF;
END;
$$;

GRANT CREATE ON SCHEMA sequent_backend TO sequent_os_ledger_data_owner, sequent_os_ledger_api_owner;

-- An operator inserts this singleton AFTER provisioning the per-tenant HSM
-- floor, software installation and published synthetic ballot. No defaults
-- select an identity or promote a tenant. The foreign keys restrict deletion.
CREATE TABLE sequent_backend.operational_initialize_local_profile (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    installation_id uuid NOT NULL,
    tenant_id uuid NOT NULL REFERENCES sequent_backend.operational_signing_required_tenant (tenant_id),
    event_id uuid NOT NULL,
    post_id uuid NOT NULL,
    publication_id uuid NOT NULL,
    action text COLLATE "C" NOT NULL CHECK (action = 'initialize-voting'),
    device_profile text COLLATE "C" NOT NULL CHECK (device_profile = 'softhsm-test'),
    CHECK ('00000000-0000-0000-0000-000000000000'::uuid NOT IN
           (installation_id, tenant_id, event_id, post_id, publication_id)),
    CHECK (current_database() <> 'operational_demo_14990'),
    FOREIGN KEY (tenant_id) REFERENCES sequent_backend.operational_setup_installation (tenant_id),
    FOREIGN KEY (tenant_id, event_id, post_id)
        REFERENCES sequent_backend.election (tenant_id, election_event_id, id),
    FOREIGN KEY (tenant_id, event_id, publication_id)
        REFERENCES sequent_backend.ballot_publication (tenant_id, election_event_id, id)
);
ALTER TABLE sequent_backend.operational_initialize_local_profile OWNER TO sequent_os_ledger_data_owner;
REVOKE ALL ON TABLE sequent_backend.operational_initialize_local_profile FROM PUBLIC;
ALTER TABLE sequent_backend.operational_initialize_local_profile ENABLE ROW LEVEL SECURITY;
ALTER TABLE sequent_backend.operational_initialize_local_profile FORCE ROW LEVEL SECURITY;
CREATE POLICY local_initialize_operator ON sequent_backend.operational_initialize_local_profile
    TO sequent_os_ledger_data_owner USING (true) WITH CHECK (true);

-- One witness per live ballot style of the installed Post. The style has
-- composite FKs to both the Post and the publication, including tenant/event.
-- This read-only join keeps the migration owner's existing base privileges.
-- The API owner can row-lock a witness through the view without receiving
-- UPDATE on the publication, style, deployment policy or installed row.
CREATE VIEW sequent_backend.operational_initialize_local_authority WITH (security_barrier = true) AS
    SELECT s.installation_id, s.tenant_id, s.event_id, s.post_id, s.publication_id, s.device_profile
    FROM sequent_backend.operational_initialize_local_profile AS s
    JOIN sequent_backend.operational_signing_policy AS policy
      ON policy.singleton AND policy.mode = 'legacy-p12'
    JOIN sequent_backend.operational_signing_required_tenant AS floor ON floor.tenant_id = s.tenant_id
    JOIN sequent_backend.operational_setup_installation AS device
      ON device.tenant_id = s.tenant_id AND device.device_profile = s.device_profile
    JOIN sequent_backend.ballot_publication AS pub
      ON (pub.tenant_id, pub.election_event_id, pub.id) = (s.tenant_id, s.event_id, s.publication_id)
    JOIN sequent_backend.ballot_style AS style
      ON (style.tenant_id, style.election_event_id, style.ballot_publication_id, style.election_id)
       = (s.tenant_id, s.event_id, s.publication_id, s.post_id)
    WHERE current_database() <> 'operational_demo_14990'
      AND s.singleton AND s.action = 'initialize-voting' AND s.device_profile = 'softhsm-test'
      AND pub.is_generated AND pub.deleted_at IS NULL AND style.deleted_at IS NULL
      AND isfinite(pub.published_at) AND pub.published_at <= statement_timestamp()
      AND (pub.election_id IS NULL OR pub.election_id = s.post_id)
      AND (coalesce(cardinality(pub.election_ids), 0) = 0 OR s.post_id = ANY(pub.election_ids))
      -- Use the same winner as published_publication_of_post, even when
      -- only an older publication would pass the local ballot checks.
      AND pub.id = (
          SELECT p.id FROM sequent_backend.ballot_publication AS p
          WHERE p.tenant_id = s.tenant_id AND p.election_event_id = s.event_id
            AND p.published_at IS NOT NULL AND p.deleted_at IS NULL
            AND (p.election_id = s.post_id OR s.post_id = ANY(p.election_ids))
          ORDER BY p.published_at DESC, p.id
          LIMIT 1
      );
REVOKE ALL ON TABLE sequent_backend.operational_initialize_local_authority FROM PUBLIC;
GRANT SELECT, UPDATE (installation_id) ON TABLE sequent_backend.operational_initialize_local_authority
    TO sequent_os_ledger_api_owner;

CREATE FUNCTION sequent_backend.operational_signing_local_initialize_installation_id(
    p_tenant uuid, p_event uuid, p_post uuid, p_publication uuid, p_profile text
) RETURNS uuid LANGUAGE sql STABLE STRICT SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT a.installation_id FROM sequent_backend.operational_initialize_local_authority AS a
    WHERE (a.tenant_id, a.event_id, a.post_id, a.publication_id, a.device_profile)
        = (p_tenant, p_event, p_post, p_publication, p_profile COLLATE "C")
    LIMIT 1;
$$;
ALTER FUNCTION sequent_backend.operational_signing_local_initialize_installation_id(uuid,uuid,uuid,uuid,text)
    OWNER TO sequent_os_ledger_api_owner;
REVOKE ALL ON FUNCTION sequent_backend.operational_signing_local_initialize_installation_id(uuid,uuid,uuid,uuid,text)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION sequent_backend.operational_signing_local_initialize_installation_id(uuid,uuid,uuid,uuid,text)
    TO sequent_os_approve, sequent_os_initialize_local_broker, sequent_os_initialize_device;

CREATE FUNCTION sequent_backend.operational_initialize_local_profile_guard()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
BEGIN
    IF TG_OP IN ('UPDATE', 'TRUNCATE') THEN
        RAISE EXCEPTION 'retire the installed local Initialize scope explicitly';
    END IF;
    -- A row trigger already owns relation/row locks. Never wait for a
    -- promoter, prepare or rollback in the opposite order. Retirement must
    -- see work committed by the previous holder of the policy lock.
    IF current_setting('transaction_isolation') <> 'read committed' THEN
        RAISE EXCEPTION 'provision or retire the local Initialize scope at read committed';
    END IF;
    IF NOT pg_try_advisory_xact_lock(hashtextextended('operational-signing-policy:deployment', 0)) THEN
        RAISE EXCEPTION 'signing policy is busy' USING ERRCODE = '55P03';
    END IF;
    IF TG_OP = 'INSERT' THEN
        -- AFTER INSERT sees the candidate. Failure rolls back that INSERT.
        PERFORM FROM sequent_backend.operational_initialize_local_authority AS a
        WHERE a.installation_id = NEW.installation_id LIMIT 1 FOR SHARE OF a NOWAIT;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'local Initialize scope requires the configured tenant and its published Post ballot'
                USING ERRCODE = 'check_violation';
        END IF;
    ELSIF EXISTS (
        SELECT FROM sequent_backend.operational_signing_intent AS i
        WHERE i.tenant_id = OLD.tenant_id AND i.event_id = OLD.event_id
          AND i.issuer COLLATE "C" = 'http://localhost:18090/realms/tenant-' || OLD.tenant_id::text
          AND (i.binding).action = 'initialize-voting' AND i.purpose = 'approval-payload'
          AND i.phase <> 'applied'
    ) THEN
        RAISE EXCEPTION 'reconcile pending local Initialize evidence before retiring the scope'
            USING ERRCODE = 'object_in_use';
    END IF;
    RETURN NULL;
END;
$$;
ALTER FUNCTION sequent_backend.operational_initialize_local_profile_guard() OWNER TO sequent_os_ledger_api_owner;
REVOKE ALL ON FUNCTION sequent_backend.operational_initialize_local_profile_guard() FROM PUBLIC;
CREATE TRIGGER local_initialize_profile_check AFTER INSERT OR DELETE
    ON sequent_backend.operational_initialize_local_profile
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.operational_initialize_local_profile_guard();
CREATE TRIGGER local_initialize_profile_immutable BEFORE UPDATE OR TRUNCATE
    ON sequent_backend.operational_initialize_local_profile
    FOR EACH STATEMENT EXECUTE FUNCTION sequent_backend.operational_initialize_local_profile_guard();


-- CREATE OR REPLACE retains the 1100 owner and EXECUTE ACL.
CREATE OR REPLACE FUNCTION sequent_backend.operational_initialize_prepare(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_payload_sha256 text,
    p_profile text, p_attempt uuid, p_issuer text, p_nonce_sha256 text, p_binding_sha256 text
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
DECLARE
    c record;
    b sequent_backend.operational_signing_binding;
    stale sequent_backend.operational_signing_binding;
    local_issuer boolean := p_issuer COLLATE "C" = 'http://localhost:18090/realms/tenant-' || p_tenant::text;
    request_post uuid;
    request_subject jsonb;
    signed_payload jsonb;
    publication_text text;
BEGIN
    IF p_tenant IS NULL OR p_event IS NULL OR p_request IS NULL OR p_actor IS NULL
       OR p_attempt IS NULL OR p_attempt = '00000000-0000-0000-0000-000000000000'
       OR p_issuer IS NULL OR (p_issuer !~ '^https://[^[:space:][:cntrl:]]+$' AND NOT local_issuer)
       OR octet_length(p_issuer) > 1024
       OR p_nonce_sha256 IS NULL OR p_nonce_sha256 !~ '^[0-9a-f]{64}$'
       OR p_binding_sha256 IS NULL OR p_binding_sha256 !~ '^[0-9a-f]{64}$' THEN
        RETURN false;
    END IF;
    IF local_issuer THEN
        -- SECURITY DEFINER changes current_user. Only the authenticated login
        -- and its direct, inherited grants identify the local service here.
        IF p_profile COLLATE "C" IS DISTINCT FROM 'softhsm-test'
           OR p_actor = '00000000-0000-0000-0000-000000000000'
           OR NOT EXISTS (
               SELECT FROM pg_catalog.pg_roles AS login
               JOIN pg_catalog.pg_auth_members AS m ON m.member = login.oid
               WHERE login.rolname = session_user AND login.rolcanlogin
                 AND NOT (login.rolsuper OR login.rolcreaterole OR login.rolcreatedb
                          OR login.rolreplication OR login.rolbypassrls)
               GROUP BY login.oid
               HAVING count(DISTINCT m.roleid) = 2
                  AND bool_and(m.inherit_option AND NOT m.admin_option AND m.roleid IN
                      ('sequent_os_initialize_orchestrator'::regrole, 'sequent_os_initialize_local_broker'::regrole))
           ) THEN
            RETURN false;
        END IF;
        PERFORM pg_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment', 0));
    ELSIF EXISTS (
        SELECT FROM pg_catalog.pg_auth_members AS m
        JOIN pg_catalog.pg_roles AS login ON login.oid = m.member
        WHERE login.rolname = session_user AND m.roleid = 'sequent_os_initialize_local_broker'::regrole
          AND m.inherit_option
    ) THEN
        -- This explicitly marked login is dedicated to the local issuer.
        RETURN false;
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'signing-event:' || p_tenant::text || ':' || p_event::text, 0));
    IF local_issuer THEN
        -- Lock in the same event -> request order as operational_signing_validate.
        SELECT r.election_id, r.subject INTO request_post, request_subject
        FROM sequent_backend.signing_request AS r
        WHERE (r.tenant_id, r.election_event_id, r.id) = (p_tenant, p_event, p_request)
          AND r.status = 'waiting' AND r.action = 'initialize-voting' AND r.payload_sha256 = p_payload_sha256
        FOR UPDATE OF r;
        IF NOT FOUND OR request_post IS NULL THEN RETURN false; END IF;
    END IF;
    SELECT * INTO c FROM sequent_backend.operational_initialize_context(
        p_tenant, p_event, p_request, p_actor, p_payload_sha256, p_profile);
    IF NOT FOUND
       OR EXISTS (SELECT FROM sequent_backend.operational_initialize_attempt AS t
                  WHERE t.challenge_id = p_attempt) THEN
        RETURN false;
    END IF;

    IF local_issuer THEN
        -- Context has checked the hash of these exact bytes, their size, the
        -- waiting request and this actor's current enrollment and certificate.
        -- Refuse unreadable JSON, duplicate keys and any split between the
        -- stored request subject and the signed subject.
        IF NOT (c.canonical_payload IS JSON OBJECT WITH UNIQUE KEYS)
           OR NOT pg_input_is_valid(c.canonical_payload, 'jsonb') THEN
            RETURN false;
        END IF;
        signed_payload := c.canonical_payload::jsonb;
        -- At least one signed election_id must name the locked Post. Every
        -- occurrence in the envelope and its subject must agree with it.
        IF signed_payload->'subject' IS DISTINCT FROM request_subject
           OR (signed_payload->>'action') COLLATE "C" IS DISTINCT FROM 'initialize-voting'
           OR NOT (signed_payload ? 'election_id' OR request_subject ? 'election_id')
           OR (signed_payload ? 'election_id'
               AND signed_payload->'election_id' IS DISTINCT FROM to_jsonb(request_post))
           OR (request_subject ? 'election_id'
               AND request_subject->'election_id' IS DISTINCT FROM to_jsonb(request_post)) THEN
            RETURN false;
        END IF;
        publication_text := signed_payload #>> '{subject,publication_id}';
        IF publication_text IS NULL OR (signed_payload->'subject') ? 'ballot_publication_id'
           OR publication_text COLLATE "C" !~ '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$' THEN
            RETURN false;
        END IF;
        -- Pin the same full witness used by the read API through the ledger
        -- inserts. A concurrent publication edit can own a parent row first.
        -- NOWAIT avoids waiting for it while holding the event/request locks.
        PERFORM FROM sequent_backend.operational_initialize_local_authority AS a
        WHERE a.installation_id = sequent_backend.operational_signing_local_initialize_installation_id(
            p_tenant, p_event, request_post, publication_text::uuid, p_profile)
        LIMIT 1 FOR SHARE OF a NOWAIT;
        IF NOT FOUND THEN RETURN false; END IF;
    END IF;

    SELECT (i.binding).* INTO stale
    FROM sequent_backend.operational_signing_intent AS i
    JOIN sequent_backend.operational_initialize_attempt AS t ON t.challenge_id = i.challenge_id
    WHERE (i.tenant_id, i.event_id, i.request_id, i.actor, i.purpose)
        = (p_tenant, p_event, p_request, p_actor, 'approval-payload')
      AND i.phase = 'prepared' AND t.step IN ('issued', 'redeemed')
      AND (i.binding).intent_expires_at <= clock_timestamp();
    IF FOUND THEN
        UPDATE sequent_backend.operational_initialize_attempt AS t
        SET step = 'discarded', discarded_at = clock_timestamp()
        WHERE t.challenge_id = stale.challenge_id AND t.step IN ('issued', 'redeemed');
        IF NOT FOUND OR NOT sequent_backend.operational_signing_discard_prepared(stale) THEN
            RAISE EXCEPTION 'cannot supersede the expired Initialize challenge';
        END IF;
    END IF;

    b.tenant_id := p_tenant;
    b.event_id := p_event;
    b.request_id := p_request;
    b.actor := p_actor;
    b.action := 'initialize-voting';
    b.purpose := 'approval-payload';
    b.key_id := c.key_id;
    b.key_version := c.key_version;
    b.cert_fingerprint := c.cert_fingerprint;
    b.cert_spki := c.cert_spki;
    b.payload_hash := p_payload_sha256;
    b.message_hash := p_payload_sha256;
    b.artifact_hash := p_payload_sha256;
    b.binding_sha256 := p_binding_sha256;
    b.issuer := p_issuer;
    b.nonce_hash := p_nonce_sha256;
    b.challenge_id := p_attempt;
    b.challenge_issued_at := c.issued_at;
    b.request_expires_at := c.request_expiry;
    b.intent_expires_at := c.intent_expiry;
    IF NOT sequent_backend.operational_signing_prepare(b) THEN
        RETURN false;
    END IF;
    INSERT INTO sequent_backend.operational_initialize_attempt
        (challenge_id, tenant_id, event_id, request_id, actor, payload_sha256, binding_sha256,
         device_profile, issued_at, expires_at)
    VALUES (p_attempt, p_tenant, p_event, p_request, p_actor, p_payload_sha256, p_binding_sha256,
            p_profile, c.issued_at, c.intent_expiry);
    RETURN true;
END;
$$;

-- A deployment may customize the migrator's default ACL. Refuse unexpected
-- grants on these new objects inside this transaction, including a direct
-- SELECT on the authority view that would otherwise bypass the scoped API.
DO $$
BEGIN
    IF EXISTS (
        SELECT FROM pg_catalog.pg_class AS c
        CROSS JOIN LATERAL pg_catalog.aclexplode(c.relacl) AS g
        WHERE c.oid IN ('sequent_backend.operational_initialize_local_profile'::regclass,
                        'sequent_backend.operational_initialize_local_authority'::regclass)
          AND g.grantee <> c.relowner
          AND NOT (c.oid = 'sequent_backend.operational_initialize_local_authority'::regclass
                   AND g.grantee = 'sequent_os_ledger_api_owner'::regrole
                   AND g.privilege_type = 'SELECT' AND NOT g.is_grantable)
    ) OR EXISTS (
        SELECT FROM pg_catalog.pg_proc AS p
        CROSS JOIN LATERAL pg_catalog.aclexplode(p.proacl) AS g
        WHERE p.oid IN ('sequent_backend.operational_initialize_local_profile_guard()'::regprocedure,
                        'sequent_backend.operational_signing_local_initialize_installation_id(uuid,uuid,uuid,uuid,text)'::regprocedure)
          AND g.grantee <> p.proowner
          AND NOT (p.proname = 'operational_signing_local_initialize_installation_id'
                   AND g.grantee IN ('sequent_os_approve'::regrole, 'sequent_os_initialize_local_broker'::regrole,
                                     'sequent_os_initialize_device'::regrole)
                   AND g.privilege_type = 'EXECUTE' AND NOT g.is_grantable)
    ) THEN
        RAISE EXCEPTION 'unexpected default grants on the private local Initialize objects';
    END IF;
END;
$$;

REVOKE CREATE ON SCHEMA sequent_backend FROM sequent_os_ledger_data_owner, sequent_os_ledger_api_owner;
