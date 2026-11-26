-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Hasura keys migrations by version, not name. 1600 on a released branch
-- can mean either cast_vote_seal_key (canonical) or the original demo
-- installation. A database migrated with file-based tooling can have both.
-- Never UPDATE the migration catalogue, an installation, an event or a key.
-- Hasura runs SQL on the backend server (no psql file includes). Keep the
-- two historical SQL texts literal; the regression test verifies their bytes.
DO $reconcile$
DECLARE
    cast_sql constant text := $cast_sql$
-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- HSM outer encryption of stored cast votes (hsm-seal-v1). Not tracked in
-- Hasura.
--
-- An election event that requires sealing has a row in
-- cast_vote_sealing_required, set before any of its elections opens voting.
-- From then on every cast vote of the event is stored sealed to the event's
-- key, whose public point is in cast_vote_seal_key; with the flag set and no
-- key, casting is refused. Only the flag decides: an event without it keeps
-- storing plaintext ballots exactly as before. Both tables are insert-only,
-- one row per event, so neither the flag nor the key can be withdrawn or
-- replaced. They are written only through the SECURITY DEFINER functions
-- below, which only the publisher's group role may call. Every cast, legacy
-- events included, reads the state through cast_vote_seal_state: the flag
-- and the event's public key (also pinned in the electoral log) are public,
-- so any login that can use the schema may call it.
--
-- Requiring sealing and the first opening of a voting channel of the event
-- take the event's cast-vote-seal lock, so they serialize: either the flag
-- commits before any channel opens, or the requirement sees the channel
-- open and is refused. A cast reads the state after it saw voting open.
--
--   sequent_os_cast_vote_key_publisher  creates the event key in the HSM and
--                                       publishes its public point
DO $$
DECLARE
    v_role pg_catalog.pg_roles%ROWTYPE;
BEGIN
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles
                   WHERE rolname = 'sequent_os_cast_vote_key_publisher') THEN
        -- Roles are cluster-wide: another database migrating at the same
        -- time may create it first, which is checked below like any other.
        BEGIN
            CREATE ROLE sequent_os_cast_vote_key_publisher
                NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
        EXCEPTION WHEN duplicate_object OR unique_violation THEN
            NULL;
        END;
    END IF;
    SELECT * INTO STRICT v_role FROM pg_catalog.pg_roles
    WHERE rolname = 'sequent_os_cast_vote_key_publisher';
    IF v_role.rolcanlogin OR v_role.rolsuper OR v_role.rolcreatedb OR v_role.rolcreaterole
          OR v_role.rolreplication OR v_role.rolbypassrls THEN
        RAISE EXCEPTION 'role % exists with login or administrative attributes', v_role.rolname;
    END IF;
    IF EXISTS (
        SELECT FROM pg_catalog.pg_auth_members AS m
        JOIN pg_catalog.pg_roles AS member ON member.oid = m.member
        JOIN pg_catalog.pg_roles AS granted ON granted.oid = m.roleid
        WHERE (member.rolname = 'sequent_os_cast_vote_key_publisher'
               AND granted.rolname LIKE 'sequent\_os\_%')
           OR (granted.rolname = 'sequent_os_cast_vote_key_publisher'
               AND member.rolname LIKE 'sequent\_os\_%')
    ) THEN
        RAISE EXCEPTION 'operational group roles must not be members of each other';
    END IF;
END;
$$;

GRANT USAGE ON SCHEMA sequent_backend TO sequent_os_cast_vote_key_publisher;
GRANT CREATE ON SCHEMA sequent_backend
    TO sequent_os_ledger_data_owner, sequent_os_ledger_api_owner;

CREATE TABLE sequent_backend.cast_vote_sealing_required (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    required_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (tenant_id, election_event_id),
    FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE
);

-- The event's key: the HSM's EcdhP256 key cast-vote-seal/<tenant>/<event>,
-- one version for the whole event. The fingerprint is SHA-256 of the
-- uncompressed public point; the event's electoral log pins it.
CREATE TABLE sequent_backend.cast_vote_seal_key (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    key_id text NOT NULL,
    key_version integer NOT NULL CHECK (key_version >= 1),
    public_key_sec1 bytea NOT NULL,
    format text NOT NULL CHECK (format = 'hsm-seal-v1'),
    fingerprint text NOT NULL,
    published_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (tenant_id, election_event_id),
    FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.cast_vote_sealing_required (tenant_id, election_event_id)
        ON DELETE CASCADE,
    CHECK (key_id = 'cast-vote-seal/' || tenant_id::text || '/' || election_event_id::text),
    CHECK (octet_length(public_key_sec1) = 65 AND get_byte(public_key_sec1, 0) = 4),
    CHECK (fingerprint = encode(sha256(public_key_sec1), 'hex'))
);

-- Insert-only. A row goes only with its election event (the cascade runs
-- after the event's row is deleted).
CREATE FUNCTION sequent_backend.cast_vote_seal_guard()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        IF NOT EXISTS (SELECT FROM sequent_backend.election_event AS e
                       WHERE (e.tenant_id, e.id) = (OLD.tenant_id, OLD.election_event_id)) THEN
            RETURN OLD;
        END IF;
    END IF;
    RAISE EXCEPTION 'cast vote sealing rows are insert-only'
        USING ERRCODE = 'check_violation';
END;
$$;
CREATE TRIGGER cast_vote_sealing_required_guard
BEFORE UPDATE OR DELETE ON sequent_backend.cast_vote_sealing_required
FOR EACH ROW EXECUTE FUNCTION sequent_backend.cast_vote_seal_guard();
CREATE TRIGGER cast_vote_sealing_required_guard_truncate
BEFORE TRUNCATE ON sequent_backend.cast_vote_sealing_required
FOR EACH STATEMENT EXECUTE FUNCTION sequent_backend.cast_vote_seal_guard();
CREATE TRIGGER cast_vote_seal_key_guard
BEFORE UPDATE OR DELETE ON sequent_backend.cast_vote_seal_key
FOR EACH ROW EXECUTE FUNCTION sequent_backend.cast_vote_seal_guard();
CREATE TRIGGER cast_vote_seal_key_guard_truncate
BEFORE TRUNCATE ON sequent_backend.cast_vote_seal_key
FOR EACH STATEMENT EXECUTE FUNCTION sequent_backend.cast_vote_seal_guard();

-- Internal: whether a status has opened any voting channel. A status or
-- channel that is missing counts as not started.
CREATE FUNCTION sequent_backend.cast_vote_seal_voting_started(p_status jsonb)
RETURNS boolean LANGUAGE sql IMMUTABLE SECURITY INVOKER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT EXISTS (
        SELECT FROM unnest(ARRAY['voting_status', 'kiosk_voting_status',
                                 'early_voting_status', 'telephone_voting_status']) AS c(channel)
        WHERE coalesce(p_status ->> c.channel, 'NOT_STARTED') <> 'NOT_STARTED'
    );
$$;

-- Internal: whether no election of the event, nor the event itself, has
-- opened any voting channel yet.
CREATE FUNCTION sequent_backend.cast_vote_seal_voting_not_started(p_tenant uuid, p_event uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY INVOKER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT NOT EXISTS (
        SELECT FROM (
            SELECT e.status FROM sequent_backend.election AS e
            WHERE (e.tenant_id, e.election_event_id) = (p_tenant, p_event)
            UNION ALL
            SELECT v.status FROM sequent_backend.election_event AS v
            WHERE (v.tenant_id, v.id) = (p_tenant, p_event)
        ) AS s
        WHERE sequent_backend.cast_vote_seal_voting_started(s.status::jsonb)
    );
$$;

-- Internal: the event's cast-vote-seal lock, until the transaction ends.
CREATE FUNCTION sequent_backend.cast_vote_seal_lock(p_tenant uuid, p_event uuid)
RETURNS void LANGUAGE sql VOLATILE SECURITY INVOKER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT pg_advisory_xact_lock(hashtextextended(
        'cast-vote-seal:' || p_tenant::text || ':' || p_event::text, 0));
$$;

-- The first opening of a voting channel of an election, or of the event,
-- takes the event's lock: it waits for a requirement in flight, and one
-- that starts after it waits for its commit and then sees the channel open.
CREATE FUNCTION sequent_backend.cast_vote_seal_voting_status_lock()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
DECLARE
    v_event uuid;
BEGIN
    IF NOT sequent_backend.cast_vote_seal_voting_started(NEW.status::jsonb)
       OR (TG_OP = 'UPDATE'
           AND sequent_backend.cast_vote_seal_voting_started(OLD.status::jsonb)) THEN
        RETURN NEW;
    END IF;
    IF TG_TABLE_NAME = 'election_event' THEN
        v_event := NEW.id;
    ELSE
        v_event := NEW.election_event_id;
    END IF;
    PERFORM sequent_backend.cast_vote_seal_lock(NEW.tenant_id, v_event);
    RETURN NEW;
END;
$$;
CREATE TRIGGER cast_vote_seal_voting_status_lock
BEFORE INSERT OR UPDATE OF status ON sequent_backend.election
FOR EACH ROW EXECUTE FUNCTION sequent_backend.cast_vote_seal_voting_status_lock();
CREATE TRIGGER cast_vote_seal_voting_status_lock
BEFORE INSERT OR UPDATE OF status ON sequent_backend.election_event
FOR EACH ROW EXECUTE FUNCTION sequent_backend.cast_vote_seal_voting_status_lock();

-- Publisher: require sealing for an event before it opens voting. From then
-- on its casts are refused until its key is published. Idempotent. Only in
-- a READ COMMITTED transaction: the check after the lock must see what
-- committed while it waited.
CREATE FUNCTION sequent_backend.cast_vote_seal_require(p_tenant uuid, p_event uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
BEGIN
    IF p_tenant IS NULL OR p_event IS NULL
       OR NOT EXISTS (SELECT FROM sequent_backend.election_event AS e
                      WHERE (e.tenant_id, e.id) = (p_tenant, p_event)) THEN
        RETURN false;
    END IF;
    IF current_setting('transaction_isolation') <> 'read committed' THEN
        RAISE EXCEPTION 'cast vote sealing is required only in a READ COMMITTED transaction'
            USING ERRCODE = 'invalid_transaction_state';
    END IF;
    PERFORM sequent_backend.cast_vote_seal_lock(p_tenant, p_event);
    IF EXISTS (SELECT FROM sequent_backend.cast_vote_sealing_required AS r
               WHERE (r.tenant_id, r.election_event_id) = (p_tenant, p_event)) THEN
        RETURN true;
    END IF;
    IF NOT sequent_backend.cast_vote_seal_voting_not_started(p_tenant, p_event) THEN
        RETURN false;
    END IF;
    INSERT INTO sequent_backend.cast_vote_sealing_required (tenant_id, election_event_id)
    VALUES (p_tenant, p_event);
    RETURN true;
END;
$$;

-- Publisher: publish the event's key, requiring sealing if it was not
-- required yet (only before voting opens). Idempotent for the same key;
-- another key, version or point for the event is refused (false).
CREATE FUNCTION sequent_backend.cast_vote_seal_publish(
    p_tenant uuid, p_event uuid, p_key_id text, p_key_version integer, p_public_key bytea
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
BEGIN
    IF p_tenant IS NULL OR p_event IS NULL OR p_key_id IS NULL OR p_key_version IS NULL
       OR p_public_key IS NULL
       OR p_key_id <> 'cast-vote-seal/' || p_tenant::text || '/' || p_event::text
       OR p_key_version < 1
       OR octet_length(p_public_key) <> 65 OR get_byte(p_public_key, 0) <> 4 THEN
        RETURN false;
    END IF;
    IF NOT sequent_backend.cast_vote_seal_require(p_tenant, p_event) THEN
        RETURN false;
    END IF;
    INSERT INTO sequent_backend.cast_vote_seal_key
        (tenant_id, election_event_id, key_id, key_version, public_key_sec1, format, fingerprint)
    VALUES (p_tenant, p_event, p_key_id, p_key_version, p_public_key, 'hsm-seal-v1',
            encode(sha256(p_public_key), 'hex'))
    ON CONFLICT DO NOTHING;
    RETURN EXISTS (SELECT FROM sequent_backend.cast_vote_seal_key AS k
                   WHERE (k.tenant_id, k.election_event_id) = (p_tenant, p_event)
                     AND (k.key_id, k.key_version, k.public_key_sec1)
                         = (p_key_id, p_key_version, p_public_key));
END;
$$;

-- Runtime (Harvest, Windmill) and publisher: always one row. `required` is
-- the flag; the key columns are NULL until the key is published.
CREATE FUNCTION sequent_backend.cast_vote_seal_state(p_tenant uuid, p_event uuid)
RETURNS TABLE (required boolean, key_id text, key_version integer,
               public_key_sec1 bytea, format text, fingerprint text)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT r.tenant_id IS NOT NULL, k.key_id, k.key_version, k.public_key_sec1, k.format,
           k.fingerprint
    FROM (SELECT p_tenant AS tenant_id, p_event AS election_event_id) AS q
    LEFT JOIN sequent_backend.cast_vote_sealing_required AS r
        ON (r.tenant_id, r.election_event_id) = (q.tenant_id, q.election_event_id)
    LEFT JOIN sequent_backend.cast_vote_seal_key AS k
        ON (k.tenant_id, k.election_event_id) = (q.tenant_id, q.election_event_id);
$$;

REVOKE ALL ON TABLE sequent_backend.cast_vote_sealing_required,
    sequent_backend.cast_vote_seal_key FROM PUBLIC;
REVOKE ALL ON FUNCTION
    sequent_backend.cast_vote_seal_guard(),
    sequent_backend.cast_vote_seal_voting_started(jsonb),
    sequent_backend.cast_vote_seal_voting_not_started(uuid, uuid),
    sequent_backend.cast_vote_seal_lock(uuid, uuid),
    sequent_backend.cast_vote_seal_voting_status_lock(),
    sequent_backend.cast_vote_seal_require(uuid, uuid),
    sequent_backend.cast_vote_seal_publish(uuid, uuid, text, integer, bytea),
    sequent_backend.cast_vote_seal_state(uuid, uuid)
    FROM PUBLIC;

ALTER TABLE sequent_backend.cast_vote_sealing_required
    OWNER TO sequent_os_ledger_data_owner;
ALTER TABLE sequent_backend.cast_vote_seal_key
    OWNER TO sequent_os_ledger_data_owner;
GRANT SELECT, INSERT ON TABLE sequent_backend.cast_vote_sealing_required,
    sequent_backend.cast_vote_seal_key TO sequent_os_ledger_api_owner;

ALTER FUNCTION sequent_backend.cast_vote_seal_guard()
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.cast_vote_seal_voting_started(jsonb)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.cast_vote_seal_voting_not_started(uuid, uuid)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.cast_vote_seal_lock(uuid, uuid)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.cast_vote_seal_voting_status_lock()
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.cast_vote_seal_require(uuid, uuid)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.cast_vote_seal_publish(uuid, uuid, text, integer, bytea)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.cast_vote_seal_state(uuid, uuid)
    OWNER TO sequent_os_ledger_api_owner;

GRANT EXECUTE ON FUNCTION
    sequent_backend.cast_vote_seal_require(uuid, uuid),
    sequent_backend.cast_vote_seal_publish(uuid, uuid, text, integer, bytea)
    TO sequent_os_cast_vote_key_publisher;
GRANT EXECUTE ON FUNCTION sequent_backend.cast_vote_seal_state(uuid, uuid) TO PUBLIC;

REVOKE CREATE ON SCHEMA sequent_backend
    FROM sequent_os_ledger_data_owner, sequent_os_ledger_api_owner;
$cast_sql$;
    demo_sql constant text := $demo_sql$
-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Inert everywhere by default. Only an operator can opt a dedicated,
-- offline demo database into this policy; no JSON tenant setting, import,
-- API, header or environment variable can activate it.
CREATE TABLE sequent_backend.operational_signing_demo_install (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    run_id uuid NOT NULL UNIQUE CHECK (run_id <> '00000000-0000-0000-0000-000000000000'),
    device_profile text NOT NULL CHECK (device_profile IN ('softhsm-test','yubihsm')),
    installed_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    CHECK (current_database() = 'operational_demo_14990')
);
CREATE TABLE sequent_backend.operational_signing_demo_scope (
    tenant_id uuid NOT NULL,
    event_id uuid NOT NULL,
    run_id uuid NOT NULL REFERENCES sequent_backend.operational_signing_demo_install(run_id),
    PRIMARY KEY (tenant_id,event_id),
    FOREIGN KEY (tenant_id,event_id) REFERENCES sequent_backend.election_event(tenant_id,id)
);
REVOKE ALL ON TABLE sequent_backend.operational_signing_demo_install,
    sequent_backend.operational_signing_demo_scope FROM PUBLIC;

CREATE FUNCTION sequent_backend.operational_demo_immutable()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog AS $$
BEGIN
    RAISE EXCEPTION 'operational demo installation is immutable; create a fresh isolated installation';
END $$;
REVOKE ALL ON FUNCTION sequent_backend.operational_demo_immutable() FROM PUBLIC;
CREATE TRIGGER operational_demo_install_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON sequent_backend.operational_signing_demo_install FOR EACH STATEMENT
    EXECUTE FUNCTION sequent_backend.operational_demo_immutable();
CREATE TRIGGER operational_demo_scope_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON sequent_backend.operational_signing_demo_scope FOR EACH STATEMENT
    EXECUTE FUNCTION sequent_backend.operational_demo_immutable();

-- This trigger is never executed on a hosted database. In the dedicated
-- database, it remains inactive until the installation row exists. Existing
-- unrelated tenants stay unchanged. A fresh tenant is HSM-required before
-- its creating transaction can finish, but cannot sign in an unadmitted event.
CREATE FUNCTION sequent_backend.operational_demo_new_tenant()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog AS $$
DECLARE v_profile text;
BEGIN
    IF current_database() <> 'operational_demo_14990' THEN RETURN NEW; END IF;
    SELECT device_profile INTO v_profile FROM sequent_backend.operational_signing_demo_install WHERE singleton;
    IF v_profile IS NOT NULL THEN
        INSERT INTO sequent_backend.operational_signing_required_tenant(tenant_id) VALUES (NEW.id);
        INSERT INTO sequent_backend.operational_setup_installation(tenant_id,device_profile) VALUES (NEW.id,v_profile);
    END IF;
    RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION sequent_backend.operational_demo_new_tenant() FROM PUBLIC;
CREATE TRIGGER operational_demo_new_tenant AFTER INSERT ON sequent_backend.tenant
    FOR EACH ROW WHEN (current_database() = 'operational_demo_14990')
    EXECUTE FUNCTION sequent_backend.operational_demo_new_tenant();

-- Operator-only transaction. No SECURITY DEFINER: the caller needs the actual
-- deployment DDL owner's rights. Enrolling oneself/creating a tenant is not
-- permission to mark a database or register an event.
CREATE FUNCTION sequent_backend.operational_register_demo_scope(
    p_tenant uuid,p_event uuid,p_run uuid,p_profile text
) RETURNS void LANGUAGE plpgsql SECURITY INVOKER SET search_path = pg_catalog AS $$
BEGIN
    IF current_database() IS DISTINCT FROM 'operational_demo_14990' THEN
        RAISE EXCEPTION 'dedicated demo database only';
    END IF;
    IF p_run IS NULL OR p_run='00000000-0000-0000-0000-000000000000'
       OR p_profile IS NULL OR p_profile NOT IN ('softhsm-test','yubihsm')
       OR p_tenant IS NULL OR p_event IS NULL THEN RAISE EXCEPTION 'invalid demo installation'; END IF;
    -- Block concurrent tenant writers across the first installation so none
    -- can miss the default. Legacy approve holds the matching policy lock.
    LOCK TABLE sequent_backend.tenant IN SHARE ROW EXCLUSIVE MODE;
    PERFORM pg_advisory_xact_lock(hashtextextended('operational-signing-policy:deployment',0));
    IF NOT EXISTS (SELECT FROM sequent_backend.election_event WHERE (tenant_id,id)=(p_tenant,p_event)) THEN
        RAISE EXCEPTION 'unknown demo event';
    END IF;
    IF EXISTS (SELECT FROM sequent_backend.operational_signing_demo_install
        WHERE (run_id,device_profile) IS DISTINCT FROM (p_run,p_profile)) THEN
        RAISE EXCEPTION 'installation identity cannot change';
    END IF;
    IF EXISTS (SELECT FROM sequent_backend.operational_setup_installation WHERE tenant_id=p_tenant AND device_profile<>p_profile) THEN
        RAISE EXCEPTION 'conflicting tenant device installation';
    END IF;
    IF to_regprocedure('sequent_backend.operational_initialize_claim_self_setup(uuid,uuid,uuid,uuid)') IS NULL THEN
        RAISE EXCEPTION 'install the request-bound enrollment migration before enabling the demo';
    END IF;
    -- Never alter privileges for hosted installations. In this explicitly
    -- dedicated database the device must use the request-bound claim API,
    -- not the existing general Security Officer workflow. Its definer still
    -- owns the original function. This operation also reasserts on rerun.
    REVOKE EXECUTE ON FUNCTION sequent_backend.operational_setup_claim_generation(uuid,uuid,uuid,uuid)
        FROM sequent_os_setup_device;
    IF EXISTS (SELECT FROM sequent_backend.operational_signing_demo_scope
               WHERE (tenant_id,event_id,run_id)=(p_tenant,p_event,p_run)) THEN RETURN; END IF;
    -- Don't strand legacy work or convert software approvals from an earlier
    -- context. An operator must first finish/cancel the pending requests using
    -- the ordinary audited workflow. Never cancel anything from this function.
    IF EXISTS (SELECT FROM sequent_backend.signing_request r WHERE r.tenant_id=p_tenant
           AND (r.status='waiting' OR (r.status='completed' AND r.executed_at IS NULL))
           AND (r.election_event_id=p_event OR NOT EXISTS (
                SELECT FROM sequent_backend.operational_signing_demo_scope WHERE tenant_id=p_tenant))) THEN
        RAISE EXCEPTION 'finish or cancel existing signing work before enabling the demo policy';
    END IF;
    INSERT INTO sequent_backend.operational_signing_demo_install(run_id,device_profile)
        VALUES (p_run,p_profile) ON CONFLICT (singleton) DO NOTHING;
    INSERT INTO sequent_backend.operational_signing_required_tenant(tenant_id)
        VALUES (p_tenant) ON CONFLICT DO NOTHING;
    INSERT INTO sequent_backend.operational_setup_installation(tenant_id,device_profile)
        VALUES (p_tenant,p_profile) ON CONFLICT DO NOTHING;
    INSERT INTO sequent_backend.operational_signing_demo_scope(tenant_id,event_id,run_id)
        VALUES (p_tenant,p_event,p_run);
END $$;
REVOKE ALL ON FUNCTION sequent_backend.operational_register_demo_scope(uuid,uuid,uuid,text) FROM PUBLIC;

-- Only return the installed run for the exact exposed operation. This grants
-- neither signing nor table access. Service binaries compare the result to
-- an independently provisioned launch manifest/run-id and pinned device.
CREATE FUNCTION sequent_backend.operational_signing_demo_fixture_id(
    p_tenant uuid,p_event uuid,p_action text,p_profile text
) RETURNS uuid LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog AS $$
    SELECT s.run_id FROM sequent_backend.operational_signing_demo_scope s
      JOIN sequent_backend.operational_signing_demo_install i ON i.run_id=s.run_id
      JOIN sequent_backend.operational_setup_installation p ON p.tenant_id=s.tenant_id
      WHERE current_database()='operational_demo_14990' AND i.singleton
        AND (s.tenant_id,s.event_id)=(p_tenant,p_event) AND p_action='initialize-voting'
        AND p.device_profile=p_profile AND i.device_profile=p_profile
        AND sequent_backend.operational_signing_hsm_required(p_tenant);
$$;
GRANT CREATE ON SCHEMA sequent_backend TO sequent_os_ledger_api_owner;
GRANT SELECT ON sequent_backend.operational_signing_demo_install,
    sequent_backend.operational_signing_demo_scope TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_signing_demo_fixture_id(uuid,uuid,text,text)
    OWNER TO sequent_os_ledger_api_owner;
REVOKE CREATE ON SCHEMA sequent_backend FROM sequent_os_ledger_api_owner;
REVOKE ALL ON FUNCTION sequent_backend.operational_signing_demo_fixture_id(uuid,uuid,text,text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION sequent_backend.operational_signing_demo_fixture_id(uuid,uuid,text,text)
    TO sequent_os_approve,sequent_os_initialize_orchestrator,sequent_os_initialize_device,
       sequent_os_initialize_ca,sequent_os_setup_device,sequent_os_setup_authority;
$demo_sql$;
    ns constant oid := 'sequent_backend'::regnamespace;
    api constant name := 'sequent_os_ledger_api_owner';
    data_owner constant name := 'sequent_os_ledger_data_owner';
    publisher constant name := 'sequent_os_cast_vote_key_publisher';
    policy_owner constant oid := (SELECT relowner FROM pg_catalog.pg_class WHERE oid =
        'sequent_backend.operational_signing_policy'::regclass);
    demo_owner oid;
    has_cast boolean;
    has_demo boolean;
    r record;
    tbl pg_catalog.pg_class%ROWTYPE;
    fn pg_catalog.pg_proc%ROWTYPE;
    actual text[];
    expected text[];
BEGIN
    PERFORM pg_catalog.set_config('search_path', 'pg_catalog, sequent_backend, pg_temp', true);
    -- Freeze trigger attachment points while classifying/installing. As for
    -- the original DDL, apply during the normal exclusive upgrade window.
    LOCK TABLE sequent_backend.tenant, sequent_backend.election_event, sequent_backend.election
        IN SHARE ROW EXCLUSIVE MODE;
    -- The publisher ROLE is cluster-wide: another database may have created
    -- it. Only database-local relations, routines and trigger names are evidence.
    WITH objects(name) AS (
        SELECT relname FROM pg_class WHERE relnamespace = ns
        UNION ALL SELECT proname FROM pg_proc WHERE pronamespace = ns
        UNION ALL SELECT t.tgname FROM pg_trigger t JOIN pg_class c ON c.oid = t.tgrelid
            WHERE c.relnamespace = ns AND NOT t.tgisinternal)
    SELECT EXISTS (SELECT FROM objects WHERE name ~ '^cast_vote_seal'),
           EXISTS (SELECT FROM objects WHERE name ~ '^(operational_signing_demo_|operational_demo_|operational_register_demo_scope$)')
    INTO has_cast, has_demo;
    IF NOT has_cast AND NOT has_demo THEN
        RAISE EXCEPTION 'Unrecognized backend-db 1794700001600: neither cast sealing nor operational demo schema exists';
    END IF;
    IF has_cast AND (to_regclass('sequent_backend.cast_vote_sealing_required') IS NULL
                  OR to_regclass('sequent_backend.cast_vote_seal_key') IS NULL) THEN
        RAISE EXCEPTION 'Incomplete or unexpected 1600 table: cast sealing';
    END IF;
    IF has_demo AND (to_regclass('sequent_backend.operational_signing_demo_install') IS NULL
                  OR to_regclass('sequent_backend.operational_signing_demo_scope') IS NULL) THEN
        RAISE EXCEPTION 'Incomplete or unexpected 1600 table: operational demo';
    END IF;
    -- Both originals temporarily grant CREATE to these roles, then revoke
    -- it. Do not undo an unexpected existing privilege during a repair.
    IF has_schema_privilege(api, ns, 'CREATE') OR has_schema_privilege(data_owner, ns, 'CREATE') THEN
        RAISE EXCEPTION 'Unsafe backend-db 1794700001600: ledger owner has schema CREATE';
    END IF;
    IF NOT has_cast THEN EXECUTE cast_sql; END IF;
    IF NOT has_demo THEN EXECUTE demo_sql; END IF;

    -- Adopt a pre-existing feature without touching its rows, ACLs, triggers
    -- or function definitions. Check the resulting boundaries for *both*
    -- features before committing anything. No IF NOT EXISTS patch-up of
    -- partially installed objects; any exception rolls the whole step back.
    IF (SELECT count(*) FROM pg_roles WHERE rolname IN (api, data_owner, publisher)
          AND NOT (rolcanlogin OR rolsuper OR rolcreatedb OR rolcreaterole OR rolreplication OR rolbypassrls)) <> 3
       OR NOT has_schema_privilege(publisher, ns, 'USAGE')
       OR has_schema_privilege(publisher, ns, 'CREATE')
       OR EXISTS (SELECT FROM pg_auth_members m JOIN pg_roles g ON g.oid = m.roleid
                  JOIN pg_roles u ON u.oid = m.member
                  WHERE u.rolname IN (api, data_owner, publisher)
                     OR (g.rolname IN (api, data_owner, publisher) AND u.rolname LIKE 'sequent\_os\_%')) THEN
        RAISE EXCEPTION 'Unsafe protected ledger role in backend-db 1794700001600';
    END IF;
    SELECT relowner INTO demo_owner FROM pg_class WHERE oid = to_regclass('sequent_backend.operational_signing_demo_install');
    IF demo_owner IS NULL OR demo_owner NOT IN (policy_owner, current_user::regrole::oid) THEN
        RAISE EXCEPTION 'Incomplete or unexpected operational demo installation owner in backend-db 1794700001600';
    END IF;
    FOR r IN SELECT * FROM (VALUES
        ('cast_vote_sealing_required', data_owner::regrole::oid,
         ARRAY['tenant_id:uuid','election_event_id:uuid','required_at:timestamp with time zone:clock_timestamp()'],
         ARRAY['PRIMARY KEY (tenant_id, election_event_id)',
               'FOREIGN KEY (tenant_id, election_event_id) REFERENCES election_event(tenant_id, id) ON DELETE CASCADE']),
        ('cast_vote_seal_key', data_owner::regrole::oid,
         ARRAY['tenant_id:uuid','election_event_id:uuid','key_id:text','key_version:integer',
               'public_key_sec1:bytea','format:text','fingerprint:text','published_at:timestamp with time zone:clock_timestamp()'],
         ARRAY['PRIMARY KEY (tenant_id, election_event_id)',
               'FOREIGN KEY (tenant_id, election_event_id) REFERENCES cast_vote_sealing_required(tenant_id, election_event_id) ON DELETE CASCADE',
               'CHECK ((key_version >= 1))', 'CHECK ((format = ''hsm-seal-v1''::text))',
               'CHECK ((key_id = (((''cast-vote-seal/''::text || (tenant_id)::text) || ''/''::text) || (election_event_id)::text)))',
               'CHECK (((octet_length(public_key_sec1) = 65) AND (get_byte(public_key_sec1, 0) = 4)))',
               'CHECK ((fingerprint = encode(sha256(public_key_sec1), ''hex''::text)))']),
        ('operational_signing_demo_install', demo_owner,
         ARRAY['singleton:boolean:true','run_id:uuid','device_profile:text','installed_at:timestamp with time zone:clock_timestamp()'],
         ARRAY['PRIMARY KEY (singleton)', 'UNIQUE (run_id)', 'CHECK (singleton)',
               'CHECK ((run_id <> ''00000000-0000-0000-0000-000000000000''::uuid))',
               'CHECK ((device_profile = ANY (ARRAY[''softhsm-test''::text, ''yubihsm''::text])))',
               'CHECK ((current_database() = ''operational_demo_14990''::name))']),
        ('operational_signing_demo_scope', demo_owner,
         ARRAY['tenant_id:uuid','event_id:uuid','run_id:uuid'],
         ARRAY['PRIMARY KEY (tenant_id, event_id)',
               'FOREIGN KEY (run_id) REFERENCES operational_signing_demo_install(run_id)',
               'FOREIGN KEY (tenant_id, event_id) REFERENCES election_event(tenant_id, id)'])
    ) AS tables(name, owner_id, cols, constraints) LOOP
        SELECT * INTO tbl FROM pg_class WHERE relnamespace = ns AND relname = r.name;
        IF NOT FOUND OR tbl.relkind <> 'r' OR tbl.relpersistence <> 'p' OR tbl.relowner <> r.owner_id
           OR tbl.relrowsecurity OR tbl.relforcerowsecurity OR tbl.relhasrules
           OR EXISTS (SELECT FROM pg_inherits WHERE inhrelid = tbl.oid OR inhparent = tbl.oid) THEN
            RAISE EXCEPTION 'Incomplete or unexpected 1600 table: %', r.name;
        END IF;
        EXECUTE format('LOCK TABLE sequent_backend.%I IN SHARE ROW EXCLUSIVE MODE', r.name);
        SELECT array_agg(a.attname || ':' || format_type(a.atttypid, a.atttypmod)
                         || coalesce(':' || pg_get_expr(d.adbin,d.adrelid), '') ORDER BY a.attnum)
            FILTER (WHERE a.attnotnull AND a.attacl IS NULL AND a.attidentity = '' AND a.attgenerated = '')
        INTO actual FROM pg_attribute a LEFT JOIN pg_attrdef d ON (d.adrelid,d.adnum) = (a.attrelid,a.attnum)
        WHERE a.attrelid = tbl.oid AND a.attnum > 0 AND NOT a.attisdropped;
        IF actual IS DISTINCT FROM r.cols OR cardinality(r.cols) <> (SELECT count(*) FROM pg_attribute
               WHERE attrelid = tbl.oid AND attnum > 0 AND NOT attisdropped) THEN
            RAISE EXCEPTION 'Unexpected columns, defaults or column privileges on %', r.name;
        END IF;
        SELECT array_agg(pg_get_constraintdef(c.oid,false)) INTO actual FROM pg_constraint c
        WHERE c.conrelid = tbl.oid AND c.contype <> 'n' -- PG18 exposes NOT NULL here as well as in attnotnull
          AND c.convalidated AND coalesce((to_jsonb(c)->>'conenforced')::boolean, true);
        IF actual IS NULL OR NOT (actual @> r.constraints AND r.constraints @> actual)
           OR EXISTS (SELECT FROM pg_constraint c WHERE c.conrelid = tbl.oid AND
                      (NOT c.convalidated OR NOT coalesce((to_jsonb(c)->>'conenforced')::boolean, true)))
           OR EXISTS (SELECT FROM pg_trigger t JOIN pg_constraint c ON c.oid = t.tgconstraint
                      WHERE c.conrelid = tbl.oid AND t.tgisinternal AND t.tgenabled NOT IN ('O','A'))
           OR EXISTS (SELECT FROM pg_index WHERE indrelid = tbl.oid AND NOT (indisvalid AND indisready)) THEN
            RAISE EXCEPTION 'Unexpected or inactive constraints on %', r.name;
        END IF;
        SELECT array_agg(CASE WHEN g.grantee=0 THEN 'PUBLIC' ELSE pg_get_userbyid(g.grantee) END
                         || ':' || g.privilege_type || CASE WHEN g.is_grantable THEN '*' ELSE '' END ORDER BY g.privilege_type)
        INTO actual FROM aclexplode(coalesce(tbl.relacl, acldefault('r',tbl.relowner))) g WHERE g.grantee <> tbl.relowner;
        expected := CASE WHEN r.name LIKE 'cast%' THEN ARRAY[api || ':INSERT', api || ':SELECT'] ELSE ARRAY[api || ':SELECT'] END;
        IF actual IS DISTINCT FROM expected THEN RAISE EXCEPTION 'Unexpected table grants on %', r.name; END IF;
    END LOOP;

    -- Identity, definer, volatility, fixed search path, owner, EXECUTE grants
    -- and the exact historical body, taken directly from its literal above.
    FOR r IN SELECT * FROM (VALUES
        ('cast_vote_seal_guard','','trigger','plpgsql','v',true,''),
        ('cast_vote_seal_voting_started','jsonb','boolean','sql','i',false,''),
        ('cast_vote_seal_voting_not_started','uuid,uuid','boolean','sql','s',false,''),
        ('cast_vote_seal_lock','uuid,uuid','void','sql','v',false,''),
        ('cast_vote_seal_voting_status_lock','','trigger','plpgsql','v',true,''),
        ('cast_vote_seal_require','uuid,uuid','boolean','plpgsql','v',true,publisher::text),
        ('cast_vote_seal_publish','uuid,uuid,text,integer,bytea','boolean','plpgsql','v',true,publisher::text),
        ('cast_vote_seal_state','uuid,uuid','TABLE(required boolean, key_id text, key_version integer, public_key_sec1 bytea, format text, fingerprint text)','sql','s',true,'PUBLIC'),
        ('operational_demo_immutable','','trigger','plpgsql','v',false,''),
        ('operational_demo_new_tenant','','trigger','plpgsql','v',true,''),
        ('operational_register_demo_scope','uuid,uuid,uuid,text','void','plpgsql','v',false,''),
        ('operational_signing_demo_fixture_id','uuid,uuid,text,text','uuid','sql','s',true,
         'sequent_os_approve,sequent_os_initialize_ca,sequent_os_initialize_device,sequent_os_initialize_orchestrator,sequent_os_setup_authority,sequent_os_setup_device')
    ) AS functions(name, args, result_type, lang, volatility, definer, grantees) LOOP
        SELECT * INTO fn FROM pg_proc WHERE oid = to_regprocedure(format('sequent_backend.%I(%s)',r.name,r.args));
        IF NOT FOUND OR (SELECT count(*) FROM pg_proc WHERE pronamespace = ns AND proname = r.name) <> 1
           OR fn.prokind <> 'f' OR fn.proisstrict OR fn.proleakproof OR fn.pronargdefaults <> 0
           OR pg_get_function_result(fn.oid) <> r.result_type OR fn.provolatile::text <> r.volatility
           OR fn.prosecdef <> r.definer OR (SELECT lanname FROM pg_language WHERE oid=fn.prolang) <> r.lang
           OR fn.proconfig IS DISTINCT FROM ARRAY[CASE WHEN r.name LIKE 'cast%' THEN 'search_path=pg_catalog, sequent_backend, pg_temp' ELSE 'search_path=pg_catalog' END]
           OR fn.proowner <> (CASE WHEN r.name LIKE 'cast%' OR r.name='operational_signing_demo_fixture_id'
                                 THEN api::regrole::oid ELSE demo_owner END)
           OR fn.prosrc IS DISTINCT FROM split_part(split_part(split_part(
                 CASE WHEN r.name LIKE 'cast%' THEN cast_sql ELSE demo_sql END,
                 'CREATE FUNCTION sequent_backend.' || r.name || '(', 2), 'AS $$', 2), '$$;', 1) THEN
            RAISE EXCEPTION 'Incomplete or changed 1600 function: %', r.name;
        END IF;
        SELECT ARRAY(SELECT CASE WHEN g.grantee=0 THEN 'PUBLIC' ELSE pg_get_userbyid(g.grantee) END
                       || CASE WHEN g.is_grantable THEN '*' ELSE '' END
                     FROM aclexplode(coalesce(fn.proacl, acldefault('f',fn.proowner))) g
                     WHERE g.grantee <> fn.proowner ORDER BY 1) INTO actual;
        IF NOT has_function_privilege(fn.proowner, fn.oid, 'EXECUTE')
           OR actual IS DISTINCT FROM string_to_array(r.grantees,',') THEN
            RAISE EXCEPTION 'Unexpected EXECUTE grants on %', r.name;
        END IF;
    END LOOP;

    IF (SELECT count(*) FROM pg_trigger WHERE NOT tgisinternal AND tgrelid IN (
            'sequent_backend.cast_vote_sealing_required'::regclass, 'sequent_backend.cast_vote_seal_key'::regclass,
            'sequent_backend.operational_signing_demo_install'::regclass, 'sequent_backend.operational_signing_demo_scope'::regclass)) <> 6 THEN
        RAISE EXCEPTION 'Unexpected trigger set on protected 1600 tables';
    END IF;
    -- pg_trigger.tgtype flags: ROW=1, BEFORE=2, INSERT=4, DELETE=8,
    -- UPDATE=16, TRUNCATE=32. Verify the actual target (not just its name).
    FOR r IN SELECT * FROM (VALUES
        ('cast_vote_sealing_required','cast_vote_sealing_required_guard',27,NULL,'cast_vote_seal_guard()'),
        ('cast_vote_sealing_required','cast_vote_sealing_required_guard_truncate',34,NULL,'cast_vote_seal_guard()'),
        ('cast_vote_seal_key','cast_vote_seal_key_guard',27,NULL,'cast_vote_seal_guard()'),
        ('cast_vote_seal_key','cast_vote_seal_key_guard_truncate',34,NULL,'cast_vote_seal_guard()'),
        ('election','cast_vote_seal_voting_status_lock',23,'status','cast_vote_seal_voting_status_lock()'),
        ('election_event','cast_vote_seal_voting_status_lock',23,'status','cast_vote_seal_voting_status_lock()'),
        ('operational_signing_demo_install','operational_demo_install_immutable',58,NULL,'operational_demo_immutable()'),
        ('operational_signing_demo_scope','operational_demo_scope_immutable',58,NULL,'operational_demo_immutable()'),
        ('tenant','operational_demo_new_tenant',5,NULL,'operational_demo_new_tenant()')
    ) AS triggers(rel, name, flags, col, func) LOOP
        IF NOT EXISTS (SELECT FROM pg_trigger t WHERE tgrelid = to_regclass('sequent_backend.' || r.rel)
           AND tgname = r.name AND NOT tgisinternal AND tgenabled IN ('O','A') AND tgtype = r.flags AND tgnargs = 0
           AND tgfoid = to_regprocedure('sequent_backend.' || r.func)
           AND tgattr::text = coalesce((SELECT attnum::text FROM pg_attribute WHERE attrelid = t.tgrelid AND attname = r.col),'')
           AND pg_get_expr(tgqual,tgrelid) IS NOT DISTINCT FROM
               (CASE WHEN r.rel='tenant' THEN '(current_database() = ''operational_demo_14990''::name)' END)) THEN
            RAISE EXCEPTION 'Incomplete or changed 1600 trigger: %.%', r.rel,r.name;
        END IF;
    END LOOP;
END;
$reconcile$;
