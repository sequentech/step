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
-- below, which only the publisher's group role may call. Harvest and
-- Windmill read the state through cast_vote_seal_state.
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

-- Internal: whether no election of the event, nor the event itself, has
-- opened any voting channel yet. A status that is missing counts as not
-- started.
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
        CROSS JOIN unnest(ARRAY['voting_status', 'kiosk_voting_status',
                                'early_voting_status', 'telephone_voting_status']) AS c(channel)
        WHERE coalesce(s.status::jsonb ->> c.channel, 'NOT_STARTED') <> 'NOT_STARTED'
    );
$$;

-- Publisher: require sealing for an event before it opens voting. From then
-- on its casts are refused until its key is published. Idempotent.
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
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'cast-vote-seal:' || p_tenant::text || ':' || p_event::text, 0));
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
    sequent_backend.cast_vote_seal_voting_not_started(uuid, uuid),
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
ALTER FUNCTION sequent_backend.cast_vote_seal_voting_not_started(uuid, uuid)
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
GRANT EXECUTE ON FUNCTION sequent_backend.cast_vote_seal_state(uuid, uuid)
    TO sequent_os_approve, sequent_os_cast_vote_key_publisher;

REVOKE CREATE ON SCHEMA sequent_backend
    FROM sequent_os_ledger_data_owner, sequent_os_ledger_api_owner;
