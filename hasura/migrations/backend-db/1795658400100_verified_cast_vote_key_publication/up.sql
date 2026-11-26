-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only

-- The opted-in receiver's authority is separate from the legacy manual
-- publisher. The poller can only read eligible ids and stage a public point.
-- It cannot authorize an event, replace a key, pin a board statement or open voting.
DO $$
DECLARE
    role_name text;
    role_row pg_catalog.pg_roles%ROWTYPE;
BEGIN
    FOREACH role_name IN ARRAY ARRAY[
        'sequent_os_verified_cast_key_importer',
        'sequent_os_verified_cast_key_publisher'
    ] LOOP
        BEGIN
            EXECUTE format(
                'CREATE ROLE %I NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS',
                role_name);
        EXCEPTION WHEN duplicate_object OR unique_violation THEN
            NULL;
        END;
        SELECT * INTO STRICT role_row FROM pg_catalog.pg_roles
        WHERE rolname = role_name;
        IF role_row.rolcanlogin OR role_row.rolsuper OR role_row.rolcreatedb
           OR role_row.rolcreaterole OR role_row.rolreplication OR role_row.rolbypassrls THEN
            RAISE EXCEPTION 'unsafe operational role: %', role_name;
        END IF;
        IF EXISTS (
            SELECT FROM pg_catalog.pg_auth_members AS m
            JOIN pg_catalog.pg_roles AS member ON member.oid = m.member
            JOIN pg_catalog.pg_roles AS granted ON granted.oid = m.roleid
            WHERE (member.rolname = role_name AND granted.rolname LIKE 'sequent\_os\_%')
               OR (granted.rolname = role_name AND member.rolname LIKE 'sequent\_os\_%')
        ) THEN
            RAISE EXCEPTION 'operational group roles must not be members of each other';
        END IF;
    END LOOP;
END;
$$;

GRANT USAGE ON SCHEMA sequent_backend TO
    sequent_os_verified_cast_key_importer, sequent_os_verified_cast_key_publisher;
GRANT CREATE ON SCHEMA sequent_backend
    TO sequent_os_ledger_data_owner, sequent_os_ledger_api_owner;

-- Empty on upgrade. Only installation administration may bind one
-- independently verified recipient and a distinct authenticated publisher.
CREATE TABLE sequent_backend.verified_cast_key_site (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    tenant_id uuid NOT NULL UNIQUE REFERENCES sequent_backend.tenant(id),
    publisher_login name NOT NULL CHECK (publisher_login <> '')
);

CREATE TABLE sequent_backend.verified_cast_key_job (
    id uuid NOT NULL UNIQUE DEFAULT gen_random_uuid(),
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    revision bigint NOT NULL CHECK (revision > 0),
    manifest_sha256 text NOT NULL CHECK (manifest_sha256 ~ '^[0-9a-f]{64}$'),
    roster_generation bigint NOT NULL CHECK (roster_generation > 0),
    roster_sha256 text NOT NULL CHECK (roster_sha256 ~ '^[0-9a-f]{64}$'),
    authorized_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    public_key_sec1 bytea CHECK (
        octet_length(public_key_sec1) = 65 AND get_byte(public_key_sec1, 0) = 4
    ),
    completed boolean NOT NULL DEFAULT false CHECK (NOT completed OR public_key_sec1 IS NOT NULL),
    PRIMARY KEY (tenant_id, election_event_id),
    FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE
);

REVOKE ALL ON TABLE sequent_backend.verified_cast_key_site,
    sequent_backend.verified_cast_key_job FROM PUBLIC;
ALTER TABLE sequent_backend.verified_cast_key_site OWNER TO sequent_os_ledger_data_owner;
ALTER TABLE sequent_backend.verified_cast_key_job OWNER TO sequent_os_ledger_data_owner;
GRANT SELECT ON sequent_backend.verified_cast_key_site TO sequent_os_ledger_api_owner;
GRANT SELECT, INSERT, UPDATE (public_key_sec1, completed)
    ON sequent_backend.verified_cast_key_job TO sequent_os_ledger_api_owner;
GRANT SELECT (tenant_id, sha256, generation) ON sequent_backend.managed_webauthn_roster
    TO sequent_os_ledger_api_owner;
GRANT SELECT (tenant_id, election_event_id, revision, manifest_sha256, xmin)
    ON sequent_backend.configuration_package TO sequent_os_ledger_api_owner;

CREATE FUNCTION sequent_backend.verified_cast_key_authorize(
    p_tenant uuid, p_event uuid, p_revision bigint, p_manifest text,
    p_generation bigint, p_roster text
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
BEGIN
    -- This seam is called in the same top-level transaction as actual new
    -- event creation and the signed-package record, never on Preview/replay.
    IF NOT EXISTS (SELECT FROM verified_cast_key_site WHERE tenant_id = p_tenant)
       OR NOT EXISTS (SELECT FROM election_event AS e
                      WHERE (e.tenant_id, e.id) = (p_tenant, p_event)
                        AND e.xmin = pg_current_xact_id()::xid)
       OR NOT EXISTS (SELECT FROM configuration_package AS p
                      WHERE (p.tenant_id, p.election_event_id, p.revision, p.manifest_sha256)
                          = (p_tenant, p_event, p_revision, p_manifest)
                        AND p.xmin = pg_current_xact_id()::xid)
       OR NOT EXISTS (SELECT FROM managed_webauthn_roster AS r
                      WHERE (r.tenant_id, r.generation, r.sha256) = (p_tenant, p_generation, p_roster))
    THEN
        RETURN false;
    END IF;
    PERFORM cast_vote_seal_lock(p_tenant, p_event);
    IF NOT cast_vote_seal_voting_not_started(p_tenant, p_event)
       OR NOT cast_vote_seal_require(p_tenant, p_event) THEN
        RETURN false;
    END IF;
    INSERT INTO verified_cast_key_job
        (tenant_id, election_event_id, revision, manifest_sha256, roster_generation, roster_sha256)
    VALUES (p_tenant, p_event, p_revision, p_manifest, p_generation, p_roster)
    ON CONFLICT DO NOTHING;
    RETURN EXISTS (SELECT FROM verified_cast_key_job AS j
                   WHERE (j.tenant_id, j.election_event_id, j.revision, j.manifest_sha256,
                          j.roster_generation, j.roster_sha256)
                       = (p_tenant, p_event, p_revision, p_manifest, p_generation, p_roster));
END;
$$;

-- A registered receiver must not silently fall back to plaintext because
-- its Step assertion or newer importer is missing. Defer until the trusted
-- importer has recorded the matching receipt in the same transaction.
CREATE FUNCTION sequent_backend.verified_cast_key_registered_import_guard()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
BEGIN
    IF EXISTS (SELECT FROM verified_cast_key_site AS s WHERE s.tenant_id = NEW.tenant_id)
       AND NOT EXISTS (
           SELECT FROM verified_cast_key_job AS j
           WHERE (j.tenant_id, j.election_event_id, j.revision, j.manifest_sha256)
               = (NEW.tenant_id, NEW.election_event_id, NEW.revision, NEW.manifest_sha256)
       )
    THEN
        RAISE EXCEPTION 'the registered receiver needs an authorized cast vote key receipt for this import'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$;
CREATE CONSTRAINT TRIGGER verified_cast_key_registered_import_guard
    AFTER INSERT ON sequent_backend.configuration_package
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION sequent_backend.verified_cast_key_registered_import_guard();

-- Match the connection identity, not SET ROLE. Refuse an account with
-- other role memberships or cluster administration; it must be dedicated.
CREATE FUNCTION sequent_backend.verified_cast_key_publisher_tenant()
RETURNS uuid LANGUAGE sql STABLE SECURITY INVOKER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT s.tenant_id FROM verified_cast_key_site AS s
    JOIN pg_roles AS identity ON identity.rolname = s.publisher_login
    WHERE identity.rolname = session_user AND identity.rolcanlogin
      AND NOT (identity.rolsuper OR identity.rolcreatedb OR identity.rolcreaterole
               OR identity.rolreplication OR identity.rolbypassrls)
      AND NOT EXISTS (
          SELECT FROM pg_auth_members AS m
          JOIN pg_roles AS parent ON parent.oid = m.roleid
          WHERE m.member = identity.oid AND parent.rolname <> 'sequent_os_verified_cast_key_publisher'
      );
$$;

-- The connection does not see the package, voter data or roster.
CREATE FUNCTION sequent_backend.verified_cast_key_next()
RETURNS TABLE (job_id text, tenant_id text, election_event_id text, manifest_sha256 text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
DECLARE
    v_tenant uuid;
BEGIN
    v_tenant := verified_cast_key_publisher_tenant();
    IF v_tenant IS NULL THEN
        RAISE EXCEPTION 'cast vote publisher has no authorized site'
            USING ERRCODE = 'insufficient_privilege';
    END IF;
    RETURN QUERY SELECT j.id::text, j.tenant_id::text, j.election_event_id::text, j.manifest_sha256
    FROM verified_cast_key_job AS j
    WHERE j.tenant_id = v_tenant AND j.public_key_sec1 IS NULL
          AND cast_vote_seal_voting_not_started(j.tenant_id, j.election_event_id)
    ORDER BY j.authorized_at, j.election_event_id
    LIMIT 16;
END;
$$;

CREATE FUNCTION sequent_backend.verified_cast_key_stage(
    p_job uuid, p_tenant uuid, p_event uuid, p_manifest text, p_public_key bytea
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
BEGIN
    IF p_tenant IS NULL OR p_tenant IS DISTINCT FROM verified_cast_key_publisher_tenant()
       OR p_public_key IS NULL OR octet_length(p_public_key) <> 65 OR get_byte(p_public_key, 0) <> 4
    THEN
        RETURN false;
    END IF;
    PERFORM cast_vote_seal_lock(p_tenant, p_event);
    IF cast_vote_seal_voting_not_started(p_tenant, p_event) THEN
        UPDATE verified_cast_key_job AS j SET public_key_sec1 = p_public_key
        WHERE (j.id, j.tenant_id, j.election_event_id, j.manifest_sha256)
            = (p_job, p_tenant, p_event, p_manifest)
          AND j.public_key_sec1 IS NULL;
    END IF;
    RETURN EXISTS (SELECT FROM verified_cast_key_job AS j
                   WHERE (j.id, j.tenant_id, j.election_event_id, j.manifest_sha256, j.public_key_sec1)
                       = (p_job, p_tenant, p_event, p_manifest, p_public_key));
END;
$$;

-- Only the trusted Step transition may consume this point and pin the
-- existing typed log statement; the publisher never receives that authority.
CREATE FUNCTION sequent_backend.verified_cast_key_for_open(p_tenant uuid, p_event uuid)
RETURNS TABLE (public_key_sec1 bytea, completed boolean)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
BEGIN
    PERFORM cast_vote_seal_lock(p_tenant, p_event);
    IF EXISTS (SELECT FROM verified_cast_key_job AS j
               WHERE (j.tenant_id, j.election_event_id) = (p_tenant, p_event) AND NOT j.completed)
       AND NOT cast_vote_seal_voting_not_started(p_tenant, p_event) THEN
        RAISE EXCEPTION 'cannot pin a verified cast vote key after voting status has changed'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN QUERY SELECT j.public_key_sec1, j.completed FROM verified_cast_key_job AS j
    WHERE (j.tenant_id, j.election_event_id) = (p_tenant, p_event);
END;
$$;

CREATE FUNCTION sequent_backend.verified_cast_key_complete(p_tenant uuid, p_event uuid, p_fingerprint text)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
DECLARE
    v_point bytea;
BEGIN
    PERFORM cast_vote_seal_lock(p_tenant, p_event);
    SELECT j.public_key_sec1 INTO v_point FROM verified_cast_key_job AS j
    WHERE (j.tenant_id, j.election_event_id) = (p_tenant, p_event)
       AND encode(sha256(j.public_key_sec1), 'hex') = p_fingerprint;
    IF v_point IS NULL OR NOT cast_vote_seal_voting_not_started(p_tenant, p_event)
       OR NOT cast_vote_seal_publish(
           p_tenant, p_event, 'cast-vote-seal/' || p_tenant::text || '/' || p_event::text, 1, v_point)
    THEN
        RETURN false;
    END IF;
    UPDATE verified_cast_key_job AS j SET completed = true
    WHERE (j.tenant_id, j.election_event_id) = (p_tenant, p_event) AND NOT j.completed;
    RETURN true;
END;
$$;

-- Even an older worker/direct write cannot leave NOT_STARTED before
-- publication. The predecessor deliberately counts every changed channel
-- as started, so a pending receipt must not be stranded by an early close.
-- Earlier events have no job; their status changes are unaffected.
CREATE FUNCTION sequent_backend.verified_cast_key_open_guard()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
DECLARE
    v_event uuid;
BEGIN
    IF NOT cast_vote_seal_voting_started(NEW.status::jsonb) THEN
        RETURN NEW;
    END IF;
    IF TG_TABLE_NAME = 'election_event' THEN
        v_event := NEW.id;
    ELSE
        v_event := NEW.election_event_id;
    END IF;
    PERFORM cast_vote_seal_lock(NEW.tenant_id, v_event);
    IF EXISTS (
        SELECT FROM verified_cast_key_job AS j
        WHERE (j.tenant_id, j.election_event_id) = (NEW.tenant_id, v_event)
          AND (
              NOT j.completed OR NOT EXISTS (
                  SELECT FROM cast_vote_seal_key AS k
                  WHERE (k.tenant_id, k.election_event_id) = (j.tenant_id, j.election_event_id)
                    AND k.key_version = 1 AND k.public_key_sec1 = j.public_key_sec1
              )
          )
    ) THEN
        RAISE EXCEPTION 'the election event needs its published and pinned cast vote key before voting status can change'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER verified_cast_key_open_guard BEFORE INSERT OR UPDATE OF status
    ON sequent_backend.election FOR EACH ROW
    EXECUTE FUNCTION sequent_backend.verified_cast_key_open_guard();
CREATE TRIGGER verified_cast_key_open_guard BEFORE INSERT OR UPDATE OF status
    ON sequent_backend.election_event FOR EACH ROW
    EXECUTE FUNCTION sequent_backend.verified_cast_key_open_guard();

REVOKE ALL ON FUNCTION
    sequent_backend.verified_cast_key_authorize(uuid, uuid, bigint, text, bigint, text),
    sequent_backend.verified_cast_key_registered_import_guard(),
    sequent_backend.verified_cast_key_publisher_tenant(),
    sequent_backend.verified_cast_key_next(),
    sequent_backend.verified_cast_key_stage(uuid, uuid, uuid, text, bytea),
    sequent_backend.verified_cast_key_for_open(uuid, uuid),
    sequent_backend.verified_cast_key_complete(uuid, uuid, text),
    sequent_backend.verified_cast_key_open_guard()
    FROM PUBLIC;
ALTER FUNCTION sequent_backend.verified_cast_key_authorize(uuid, uuid, bigint, text, bigint, text)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.verified_cast_key_registered_import_guard()
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.verified_cast_key_publisher_tenant()
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.verified_cast_key_next() OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.verified_cast_key_stage(uuid, uuid, uuid, text, bytea)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.verified_cast_key_for_open(uuid, uuid) OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.verified_cast_key_complete(uuid, uuid, text) OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.verified_cast_key_open_guard() OWNER TO sequent_os_ledger_api_owner;

GRANT EXECUTE ON FUNCTION
    sequent_backend.verified_cast_key_authorize(uuid, uuid, bigint, text, bigint, text),
    sequent_backend.verified_cast_key_for_open(uuid, uuid),
    sequent_backend.verified_cast_key_complete(uuid, uuid, text)
    TO sequent_os_verified_cast_key_importer;
GRANT EXECUTE ON FUNCTION
    sequent_backend.verified_cast_key_next(),
    sequent_backend.verified_cast_key_stage(uuid, uuid, uuid, text, bytea)
    TO sequent_os_verified_cast_key_publisher;
REVOKE CREATE ON SCHEMA sequent_backend
    FROM sequent_os_ledger_data_owner, sequent_os_ledger_api_owner;
