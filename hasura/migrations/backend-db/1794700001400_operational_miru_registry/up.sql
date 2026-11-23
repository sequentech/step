-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Operational MIRU output registry of HSM-required tenants. Not tracked in
-- Hasura. Every generated archive, EML, XZ and CSV object is registered with
-- its exact content digest in the transaction that creates it. A registered
-- document is withheld from generic downloads and event exports; the signed
-- release leaves through its own exact-file route. Legacy tenants never
-- call these functions, and every one of them refuses a legacy tenant.
GRANT CREATE ON SCHEMA sequent_backend
    TO sequent_os_ledger_data_owner, sequent_os_ledger_api_owner;

CREATE TYPE sequent_backend.operational_miru_inventory_state AS ENUM ('open', 'verified');

-- Deployment admin attestation, like the signing policy: the documents an
-- event already had when its tenant became HSM-required have been classified.
-- Until an event's inventory is verified nothing of it is released or
-- exported. Rows are written directly by the ledger data owner.
CREATE TABLE sequent_backend.operational_miru_inventory (
    tenant_id uuid NOT NULL,
    event_id uuid NOT NULL,
    inventory_id uuid NOT NULL UNIQUE DEFAULT gen_random_uuid(),
    state sequent_backend.operational_miru_inventory_state NOT NULL DEFAULT 'open',
    opened_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    verified_at timestamptz,
    PRIMARY KEY (tenant_id, event_id),
    FOREIGN KEY (tenant_id, event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE,
    CHECK ((state = 'verified') = (verified_at IS NOT NULL)),
    CHECK (verified_at >= opened_at)
);

-- One row per operational document: its kind and content never change.
CREATE TABLE sequent_backend.operational_miru_artifact (
    tenant_id uuid NOT NULL,
    event_id uuid NOT NULL,
    document_id uuid NOT NULL,
    kind text NOT NULL CHECK (kind IN ('archive', 'eml', 'xz', 'csv')),
    content_sha256 text NOT NULL CHECK (content_sha256 ~ '^[0-9a-f]{64}$'),
    inventory_id uuid REFERENCES sequent_backend.operational_miru_inventory (inventory_id),
    registered_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (tenant_id, event_id, document_id),
    FOREIGN KEY (tenant_id, event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX operational_miru_artifact_by_document
    ON sequent_backend.operational_miru_artifact (tenant_id, document_id);

-- The members of one package revision of a Post and country. The revision
-- is identified by its archive document; EML and XZ may be shared between
-- the provisional and the signed revision of the same package.
CREATE TABLE sequent_backend.operational_miru_revision_artifact (
    tenant_id uuid NOT NULL,
    event_id uuid NOT NULL,
    tally_session_id uuid NOT NULL,
    election_id uuid NOT NULL,
    area_id uuid NOT NULL,
    revision_id uuid NOT NULL,
    kind text NOT NULL CHECK (kind IN ('archive', 'eml', 'xz', 'csv')),
    document_id uuid NOT NULL,
    registered_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (tenant_id, event_id, tally_session_id, election_id, area_id, revision_id, kind),
    FOREIGN KEY (tenant_id, event_id, document_id)
        REFERENCES sequent_backend.operational_miru_artifact (tenant_id, event_id, document_id)
        ON DELETE CASCADE,
    CHECK ((kind = 'archive') = (document_id = revision_id))
);
-- A revision identifier belongs to one tally session, Post and country.
CREATE UNIQUE INDEX operational_miru_revision_scope
    ON sequent_backend.operational_miru_revision_artifact (tenant_id, event_id, revision_id, kind);

CREATE FUNCTION sequent_backend.operational_miru_guard()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
BEGIN
    IF TG_TABLE_NAME = 'operational_miru_inventory' THEN
        IF TG_OP = 'INSERT' THEN
            IF NEW.state = 'open'
               AND sequent_backend.operational_signing_hsm_required(NEW.tenant_id) THEN
                RETURN NEW;
            END IF;
        ELSIF OLD.state = 'open' AND NEW.state = 'verified'
              AND ROW(NEW.tenant_id, NEW.event_id, NEW.inventory_id, NEW.opened_at)
                  IS NOT DISTINCT FROM
                  ROW(OLD.tenant_id, OLD.event_id, OLD.inventory_id, OLD.opened_at) THEN
            RETURN NEW;
        END IF;
        RAISE EXCEPTION 'invalid operational MIRU inventory change'
            USING ERRCODE = 'check_violation';
    END IF;
    RAISE EXCEPTION 'operational MIRU registry rows are immutable'
        USING ERRCODE = 'check_violation';
END;
$$;
CREATE TRIGGER operational_miru_inventory_guard
BEFORE INSERT OR UPDATE ON sequent_backend.operational_miru_inventory
FOR EACH ROW EXECUTE FUNCTION sequent_backend.operational_miru_guard();
CREATE TRIGGER operational_miru_artifact_guard
BEFORE UPDATE ON sequent_backend.operational_miru_artifact
FOR EACH ROW EXECUTE FUNCTION sequent_backend.operational_miru_guard();
CREATE TRIGGER operational_miru_revision_artifact_guard
BEFORE UPDATE ON sequent_backend.operational_miru_revision_artifact
FOR EACH ROW EXECUTE FUNCTION sequent_backend.operational_miru_guard();

-- Runtime: classify one generated object as a member of a package revision.
-- Idempotent for the exact same tuple; a document can never be rebound to
-- other content, another kind or another revision slot. The last argument
-- is NULL at runtime. Only the deployment admin, in a session that is a
-- member of the ledger data owner, may pass the identifier of the event's
-- open inventory to classify a document that predates the registry.
CREATE FUNCTION sequent_backend.operational_miru_register_artifact(
    p_tenant uuid, p_event uuid, p_tally_session uuid, p_election uuid, p_area uuid,
    p_revision uuid, p_document uuid, p_kind text, p_content_sha256 text, p_inventory uuid
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
BEGIN
    IF p_tenant IS NULL OR p_event IS NULL OR p_tally_session IS NULL OR p_election IS NULL
       OR p_area IS NULL OR p_revision IS NULL OR p_document IS NULL
       OR p_kind IS NULL OR p_kind NOT IN ('archive', 'eml', 'xz', 'csv')
       OR (p_kind = 'archive') <> (p_document = p_revision)
       OR p_content_sha256 IS NULL OR p_content_sha256 !~ '^[0-9a-f]{64}$'
       OR NOT sequent_backend.operational_signing_hsm_required(p_tenant)
       OR NOT EXISTS (SELECT FROM sequent_backend.tally_session AS s
                      WHERE (s.tenant_id, s.election_event_id, s.id)
                          = (p_tenant, p_event, p_tally_session))
       OR NOT EXISTS (SELECT FROM sequent_backend.election AS e
                      WHERE (e.tenant_id, e.election_event_id, e.id)
                          = (p_tenant, p_event, p_election))
       OR NOT EXISTS (SELECT FROM sequent_backend.area AS a
                      WHERE (a.tenant_id, a.election_event_id, a.id)
                          = (p_tenant, p_event, p_area)) THEN
        RETURN false;
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'operational-miru-registry:' || p_tenant::text || ':' || p_event::text, 0));
    IF p_inventory IS NOT NULL
       AND (NOT pg_has_role(session_user, 'sequent_os_ledger_data_owner', 'MEMBER')
            OR NOT EXISTS (SELECT FROM sequent_backend.operational_miru_inventory AS v
                           WHERE (v.tenant_id, v.event_id, v.inventory_id)
                               = (p_tenant, p_event, p_inventory)
                             AND v.state = 'open')) THEN
        RETURN false;
    END IF;

    -- Decide before writing: a refusal leaves no partial classification.
    IF EXISTS (SELECT FROM sequent_backend.operational_miru_artifact AS a
               WHERE (a.tenant_id, a.document_id) = (p_tenant, p_document)
                 AND (a.event_id, a.kind, a.content_sha256)
                     IS DISTINCT FROM (p_event, p_kind, p_content_sha256))
       OR EXISTS (SELECT FROM sequent_backend.operational_miru_revision_artifact AS r
                  WHERE (r.tenant_id, r.event_id, r.revision_id, r.kind)
                      = (p_tenant, p_event, p_revision, p_kind)
                    AND (r.tally_session_id, r.election_id, r.area_id, r.document_id)
                        IS DISTINCT FROM (p_tally_session, p_election, p_area, p_document)) THEN
        RETURN false;
    END IF;
    INSERT INTO sequent_backend.operational_miru_artifact
        (tenant_id, event_id, document_id, kind, content_sha256, inventory_id)
    VALUES (p_tenant, p_event, p_document, p_kind, p_content_sha256, p_inventory)
    ON CONFLICT DO NOTHING;
    INSERT INTO sequent_backend.operational_miru_revision_artifact
        (tenant_id, event_id, tally_session_id, election_id, area_id, revision_id, kind,
         document_id)
    VALUES (p_tenant, p_event, p_tally_session, p_election, p_area, p_revision, p_kind,
            p_document)
    ON CONFLICT DO NOTHING;
    RETURN true;
END;
$$;

-- Runtime: whether the generic document routes must withhold this stored
-- document. NULL while the answer is not known: a legacy tenant, or an event
-- whose inventory is not verified. A registered document is always withheld.
CREATE FUNCTION sequent_backend.operational_miru_generic_withhold(
    p_tenant uuid, p_event uuid, p_document uuid
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT CASE
        WHEN p_tenant IS NULL OR p_document IS NULL
             OR NOT sequent_backend.operational_signing_hsm_required(p_tenant) THEN NULL
        WHEN EXISTS (SELECT FROM sequent_backend.operational_miru_artifact AS a
                     WHERE (a.tenant_id, a.document_id) = (p_tenant, p_document)) THEN true
        -- A document outside any event is never operational MIRU output.
        WHEN p_event IS NULL THEN false
        WHEN EXISTS (SELECT FROM sequent_backend.operational_miru_inventory AS v
                     WHERE (v.tenant_id, v.event_id) = (p_tenant, p_event)
                       AND v.state = 'verified') THEN false
        ELSE NULL
    END;
$$;

-- Runtime: the documents an event export must leave out, in the statement
-- snapshot of the export itself. NULL while the inventory is not verified.
CREATE FUNCTION sequent_backend.operational_miru_export_exclusions(
    p_tenant uuid, p_event uuid
) RETURNS uuid[] LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT CASE
        WHEN p_tenant IS NOT NULL AND p_event IS NOT NULL
             AND sequent_backend.operational_signing_hsm_required(p_tenant)
             AND EXISTS (SELECT FROM sequent_backend.operational_miru_inventory AS v
                         WHERE (v.tenant_id, v.event_id) = (p_tenant, p_event)
                           AND v.state = 'verified')
        THEN (SELECT coalesce(array_agg(a.document_id ORDER BY a.document_id), '{}'::uuid[])
              FROM sequent_backend.operational_miru_artifact AS a
              WHERE (a.tenant_id, a.event_id) = (p_tenant, p_event))
        ELSE NULL
    END;
$$;

REVOKE ALL ON TABLE sequent_backend.operational_miru_inventory,
    sequent_backend.operational_miru_artifact,
    sequent_backend.operational_miru_revision_artifact FROM PUBLIC;
REVOKE ALL ON TYPE sequent_backend.operational_miru_inventory_state FROM PUBLIC;
GRANT USAGE ON TYPE sequent_backend.operational_miru_inventory_state
    TO sequent_os_ledger_data_owner, sequent_os_ledger_api_owner;
REVOKE ALL ON FUNCTION
    sequent_backend.operational_miru_guard(),
    sequent_backend.operational_miru_register_artifact(uuid, uuid, uuid, uuid, uuid, uuid, uuid, text, text, uuid),
    sequent_backend.operational_miru_generic_withhold(uuid, uuid, uuid),
    sequent_backend.operational_miru_export_exclusions(uuid, uuid)
    FROM PUBLIC;

ALTER TABLE sequent_backend.operational_miru_inventory
    OWNER TO sequent_os_ledger_data_owner;
ALTER TABLE sequent_backend.operational_miru_artifact
    OWNER TO sequent_os_ledger_data_owner;
ALTER TABLE sequent_backend.operational_miru_revision_artifact
    OWNER TO sequent_os_ledger_data_owner;
GRANT SELECT ON TABLE sequent_backend.operational_miru_inventory
    TO sequent_os_ledger_api_owner;
GRANT SELECT, INSERT ON TABLE sequent_backend.operational_miru_artifact,
    sequent_backend.operational_miru_revision_artifact TO sequent_os_ledger_api_owner;
-- The transmission custody check joins the two registries directly.
GRANT SELECT ON TABLE sequent_backend.operational_miru_artifact,
    sequent_backend.operational_miru_revision_artifact TO sequent_os_approve;

ALTER FUNCTION sequent_backend.operational_miru_guard()
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_miru_register_artifact(uuid, uuid, uuid, uuid, uuid, uuid, uuid, text, text, uuid)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_miru_generic_withhold(uuid, uuid, uuid)
    OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_miru_export_exclusions(uuid, uuid)
    OWNER TO sequent_os_ledger_api_owner;

GRANT EXECUTE ON FUNCTION
    sequent_backend.operational_miru_register_artifact(uuid, uuid, uuid, uuid, uuid, uuid, uuid, text, text, uuid)
    TO sequent_os_approve, sequent_os_ledger_data_owner;
GRANT EXECUTE ON FUNCTION
    sequent_backend.operational_miru_generic_withhold(uuid, uuid, uuid),
    sequent_backend.operational_miru_export_exclusions(uuid, uuid)
    TO sequent_os_approve;

REVOKE CREATE ON SCHEMA sequent_backend
    FROM sequent_os_ledger_data_owner, sequent_os_ledger_api_owner;
