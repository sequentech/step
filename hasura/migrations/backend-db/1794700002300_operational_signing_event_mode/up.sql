-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Installed operator policy, not a bundle field or Hasura-tracked table.
-- Leave all existing events unmodified. No service receives this table's
-- write privileges. A future authorized provisioning path must take the
-- exclusive operational-signing-policy:deployment transaction lock before
-- changing a mode and preserve the lock order at every policy consumer.
-- Install inactive. Rolling out code/schema never activates a default that
-- changes how an existing request is verified or dispatched.
-- Role names are cluster-wide. Reuse only the exact non-privileged shape.
DO $$
DECLARE
    existing pg_catalog.pg_roles%ROWTYPE;
BEGIN
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'sequent_os_mode_reader') THEN
        BEGIN
            CREATE ROLE sequent_os_mode_reader NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
        EXCEPTION WHEN duplicate_object OR unique_violation THEN NULL;
        END;
    END IF;
    SELECT * INTO STRICT existing FROM pg_catalog.pg_roles WHERE rolname = 'sequent_os_mode_reader';
    IF existing.rolcanlogin OR existing.rolsuper OR existing.rolcreatedb OR existing.rolcreaterole
       OR existing.rolreplication OR existing.rolbypassrls
       OR EXISTS (SELECT FROM pg_catalog.pg_auth_members WHERE member = existing.oid) THEN
        RAISE EXCEPTION 'existing product signing role is not a bounded NOLOGIN role';
    END IF;
END;
$$;
GRANT USAGE ON SCHEMA sequent_backend TO sequent_os_mode_reader;

GRANT CREATE ON SCHEMA sequent_backend
    TO sequent_os_ledger_data_owner, sequent_os_ledger_api_owner;

CREATE TABLE sequent_backend.operational_signing_event_mode (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    mode text NOT NULL,
    PRIMARY KEY (tenant_id, election_event_id),
    FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id)
        ON DELETE CASCADE,
    CONSTRAINT operational_signing_event_mode_known
        CHECK (mode IN ('legacy-p12', 'hsm-required'))
);
REVOKE ALL ON TABLE sequent_backend.operational_signing_event_mode FROM PUBLIC;
ALTER TABLE sequent_backend.operational_signing_event_mode
    OWNER TO sequent_os_ledger_data_owner;
ALTER TABLE sequent_backend.operational_signing_event_mode ENABLE ROW LEVEL SECURITY;
CREATE POLICY operational_signing_event_mode_read
    ON sequent_backend.operational_signing_event_mode
    FOR SELECT TO sequent_os_ledger_api_owner USING (true);
GRANT SELECT (tenant_id, election_event_id, mode)
    ON TABLE sequent_backend.operational_signing_event_mode TO sequent_os_ledger_api_owner;

CREATE TABLE sequent_backend.operational_signing_product_activation (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    active boolean NOT NULL DEFAULT false
);
INSERT INTO sequent_backend.operational_signing_product_activation DEFAULT VALUES;
ALTER TABLE sequent_backend.operational_signing_product_activation
    OWNER TO sequent_os_ledger_data_owner;
REVOKE ALL ON TABLE sequent_backend.operational_signing_product_activation FROM PUBLIC;
GRANT SELECT (singleton, active) ON TABLE sequent_backend.operational_signing_product_activation
    TO sequent_os_ledger_api_owner;

-- A narrowly authorized operator may activate by changing the singleton.
-- This is a one-way interpretation change, not a data migration. Old
-- requests are neither converted nor cancelled; wait for them to settle or
-- resolve them through the existing authorized lifecycle. Past audit rows
-- are immutable. Deployment must also verify hardware and schedule
-- readiness outside this guard before attempting the controlled change.
CREATE FUNCTION sequent_backend.operational_signing_guard_product_activation()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
DECLARE
    blocked_requests bigint;
    missing_inventory bigint;
BEGIN
    IF TG_OP <> 'UPDATE' OR OLD.active OR NOT NEW.active OR NOT NEW.singleton
       OR current_setting('transaction_isolation') <> 'read committed' THEN
        RAISE EXCEPTION USING MESSAGE = 'Operational signing only allows one-way activation in READ COMMITTED';
    END IF;
    -- Same total ordering as the deployed election/approval/document policy
    -- readers. Queries below take a new snapshot after waiting for writers.
    PERFORM pg_advisory_xact_lock(hashtextextended('operational-signing-policy:deployment', 0));
    IF NOT EXISTS (
        SELECT FROM sequent_backend.operational_signing_policy
        WHERE singleton AND mode IN ('legacy-p12', 'hsm-required')
    ) THEN
        RAISE EXCEPTION USING MESSAGE = 'Operational signing deployment policy is unavailable';
    END IF;
    WITH promoted AS MATERIALIZED (
        SELECT e.tenant_id, e.id
        FROM sequent_backend.election_event e
        WHERE EXISTS (
            SELECT FROM sequent_backend.operational_signing_policy p
            WHERE p.singleton AND p.mode = 'legacy-p12'
        )
        AND NOT EXISTS (
            SELECT FROM sequent_backend.operational_signing_required_tenant t
            WHERE t.tenant_id = e.tenant_id
        )
        AND NOT EXISTS (
            SELECT FROM sequent_backend.operational_signing_event_mode m
            WHERE (m.tenant_id, m.election_event_id) = (e.tenant_id, e.id)
        )
    ), archived AS MATERIALIZED (
        SELECT t.tenant_id, t.election_event_id, t.id,
               (t.annotations ->> 'miru:tally-session-data')::jsonb AS packages
        FROM sequent_backend.tally_session t
        JOIN promoted e ON (e.tenant_id, e.id) = (t.tenant_id, t.election_event_id)
        WHERE t.annotations ? 'miru:tally-session-data'
    ), current_document AS MATERIALIZED (
        SELECT t.tenant_id, t.election_event_id, t.id,
               p.value->>'election_id' AS election,
               p.value->>'area_id' AS area,
               d.value#>>'{document_ids,all_servers}' AS revision,
               members.kind, members.id AS document
        FROM archived t
        CROSS JOIN LATERAL jsonb_array_elements(t.packages) p(value)
        CROSS JOIN LATERAL jsonb_array_elements(p.value->'documents') d(value)
        CROSS JOIN LATERAL (VALUES
            ('archive', d.value#>>'{document_ids,all_servers}'),
            ('xz', d.value#>>'{document_ids,xz}'),
            ('eml', d.value#>>'{document_ids,eml}'),
            ('csv', d.value#>>'{document_ids,csv}')
        ) AS members(kind, id)
        WHERE members.kind IN ('archive','xz') OR NULLIF(members.id,'') IS NOT NULL
    )
    SELECT
        (SELECT count(*) FROM sequent_backend.signing_request r
         JOIN promoted e ON (e.tenant_id, e.id) = (r.tenant_id, r.election_event_id)
         WHERE r.status IN ('waiting', 'completed')),
        (SELECT count(*) FROM promoted e
         WHERE (
           NOT EXISTS (SELECT FROM sequent_backend.operational_miru_inventory i
                       WHERE (i.tenant_id, i.event_id) = (e.tenant_id, e.id) AND i.state = 'verified')
           OR EXISTS (
               SELECT FROM current_document d
               WHERE (d.tenant_id, d.election_event_id) = (e.tenant_id, e.id)
                 AND NOT EXISTS (
                      SELECT FROM sequent_backend.operational_miru_revision_artifact a
                      WHERE (a.tenant_id,a.event_id,a.tally_session_id,a.kind) =
                            (d.tenant_id,d.election_event_id,d.id,d.kind)
                        AND a.revision_id::text = d.revision AND a.document_id::text = d.document
                        AND a.election_id::text = d.election AND a.area_id::text = d.area
                 )
           )
         ))
      INTO blocked_requests, missing_inventory;
    IF blocked_requests <> 0 OR missing_inventory <> 0 THEN
        RAISE EXCEPTION USING
            MESSAGE = 'Operational signing activation refused; preserve existing approvals and MIRU output',
            DETAIL = format('Waiting or unexecuted requests: %s; document inventories not verified: %s.',
                            blocked_requests, missing_inventory);
    END IF;
    RETURN NEW;
EXCEPTION WHEN invalid_text_representation OR invalid_parameter_value THEN
    RAISE EXCEPTION USING MESSAGE = 'Operational signing activation refused; a current archive inventory descriptor is invalid';
END;
$$;
REVOKE ALL ON FUNCTION sequent_backend.operational_signing_guard_product_activation() FROM PUBLIC;
ALTER FUNCTION sequent_backend.operational_signing_guard_product_activation()
    OWNER TO sequent_os_ledger_api_owner;
CREATE TRIGGER operational_signing_product_activation_guard
    BEFORE INSERT OR UPDATE OR DELETE ON sequent_backend.operational_signing_product_activation
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.operational_signing_guard_product_activation();
CREATE TRIGGER operational_signing_product_activation_no_truncate
    BEFORE TRUNCATE ON sequent_backend.operational_signing_product_activation
    FOR EACH STATEMENT EXECUTE FUNCTION sequent_backend.operational_signing_guard_product_activation();

-- Zero rows for an unknown scope, missing floor, or corrupt setting. Only
-- an absent event row in the overrides table yields a NULL explicit choice.
-- Transactional consumers take the established shared policy lock before
-- reading these facts; the function itself never locks an election row.
CREATE FUNCTION sequent_backend.operational_signing_event_mode_facts(
    p_tenant uuid, p_event uuid
) RETURNS TABLE (deployment_mode text, tenant_required boolean, explicit_mode text, product_active boolean)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT p.mode, EXISTS (
        SELECT FROM sequent_backend.operational_signing_required_tenant AS t
        WHERE t.tenant_id = e.tenant_id
    ), m.mode, activation.active
    FROM sequent_backend.election_event AS e
    CROSS JOIN sequent_backend.operational_signing_policy AS p
    CROSS JOIN sequent_backend.operational_signing_product_activation AS activation
    LEFT JOIN sequent_backend.operational_signing_event_mode AS m
      ON (m.tenant_id, m.election_event_id) = (e.tenant_id, e.id)
    WHERE (e.tenant_id, e.id) = (p_tenant, p_event)
      AND p.singleton AND activation.singleton
      AND p.mode IN ('legacy-p12', 'hsm-required')
      AND (m.election_event_id IS NULL OR m.mode IN ('legacy-p12', 'hsm-required'));
$$;
REVOKE ALL ON FUNCTION sequent_backend.operational_signing_event_mode_facts(uuid, uuid)
    FROM PUBLIC;
ALTER FUNCTION sequent_backend.operational_signing_event_mode_facts(uuid, uuid)
    OWNER TO sequent_os_ledger_api_owner;
-- Operational callers still need to be provisioned per real service login.
GRANT EXECUTE ON FUNCTION sequent_backend.operational_signing_event_mode_facts(uuid, uuid)
    TO sequent_os_mode_reader;

REVOKE CREATE ON SCHEMA sequent_backend
    FROM sequent_os_ledger_data_owner, sequent_os_ledger_api_owner;
