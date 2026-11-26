-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- The dedicated direct-HSM demo build treats its events as HSM-required while
-- the deployment policy stays legacy-p12 with no promoted tenant. Its MIRU
-- output must still be registered, and then withheld, in the transaction that
-- creates it. Admission is extended only to the exact Post of an event that
-- the optional direct-HSM overlay scopes; without that overlay nothing
-- changes. HSM-required tenants and their inventories behave as before; no
-- tenant is promoted and no signer or proof authority is added.

GRANT CREATE ON SCHEMA sequent_backend TO sequent_os_ledger_api_owner;

-- Whether the trusted direct-HSM installation scopes this exact event and
-- Post. Answered only by the optional overlay's own function; false when the
-- overlay is absent or the answer is unknown.
CREATE FUNCTION sequent_backend.operational_miru_direct_scope(
    p_tenant uuid, p_event uuid, p_post uuid
) RETURNS boolean LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
DECLARE
    scoped boolean;
BEGIN
    IF p_tenant IS NULL OR p_event IS NULL OR p_post IS NULL
       OR pg_catalog.to_regprocedure(
              'felix3_initialize_direct.miru_event_scope(uuid,uuid,uuid)') IS NULL THEN
        RETURN false;
    END IF;
    EXECUTE 'SELECT felix3_initialize_direct.miru_event_scope($1, $2, $3)'
        INTO scoped USING p_tenant, p_event, p_post;
    RETURN coalesce(scoped, false);
END;
$$;
REVOKE ALL ON FUNCTION sequent_backend.operational_miru_direct_scope(uuid, uuid, uuid) FROM PUBLIC;
ALTER FUNCTION sequent_backend.operational_miru_direct_scope(uuid, uuid, uuid)
    OWNER TO sequent_os_ledger_api_owner;
REVOKE CREATE ON SCHEMA sequent_backend FROM sequent_os_ledger_api_owner;

-- CREATE OR REPLACE keeps the owners and EXECUTE privileges of 1794700001400.

CREATE OR REPLACE FUNCTION sequent_backend.operational_miru_register_artifact(
    p_tenant uuid, p_event uuid, p_tally_session uuid, p_election uuid, p_area uuid,
    p_revision uuid, p_document uuid, p_kind text, p_content_sha256 text, p_inventory uuid
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
DECLARE
    admitted boolean;
BEGIN
    IF p_tenant IS NULL OR p_event IS NULL OR p_tally_session IS NULL OR p_election IS NULL
       OR p_area IS NULL OR p_revision IS NULL OR p_document IS NULL
       OR p_kind IS NULL OR p_kind NOT IN ('archive', 'eml', 'xz', 'csv')
       OR (p_kind = 'archive') <> (p_document = p_revision)
       OR p_content_sha256 IS NULL OR p_content_sha256 !~ '^[0-9a-f]{64}$'
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
    -- An HSM-required tenant, or the exact Post of an event the trusted
    -- direct-HSM installation scopes. Evaluated only on complete arguments.
    admitted := sequent_backend.operational_signing_hsm_required(p_tenant)
        OR sequent_backend.operational_miru_direct_scope(p_tenant, p_event, p_election);
    IF admitted IS NOT TRUE THEN
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

CREATE OR REPLACE FUNCTION sequent_backend.operational_miru_generic_withhold(
    p_tenant uuid, p_event uuid, p_document uuid
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT CASE
        WHEN p_tenant IS NULL OR p_document IS NULL THEN NULL
        -- A registered document is withheld whatever its tenant's mode.
        WHEN EXISTS (SELECT FROM sequent_backend.operational_miru_artifact AS a
                     WHERE (a.tenant_id, a.document_id) = (p_tenant, p_document)) THEN true
        WHEN NOT sequent_backend.operational_signing_hsm_required(p_tenant) THEN NULL
        -- A document outside any event is never operational MIRU output.
        WHEN p_event IS NULL THEN false
        WHEN EXISTS (SELECT FROM sequent_backend.operational_miru_inventory AS v
                     WHERE (v.tenant_id, v.event_id) = (p_tenant, p_event)
                       AND v.state = 'verified') THEN false
        ELSE NULL
    END;
$$;

CREATE OR REPLACE FUNCTION sequent_backend.operational_miru_export_exclusions(
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
        -- A legacy tenant has no inventory: only registered artifacts, which
        -- exist only in an exactly scoped direct-HSM event, are left out.
        WHEN p_tenant IS NOT NULL AND p_event IS NOT NULL
             AND NOT sequent_backend.operational_signing_hsm_required(p_tenant)
        THEN (SELECT coalesce(array_agg(a.document_id ORDER BY a.document_id), '{}'::uuid[])
              FROM sequent_backend.operational_miru_artifact AS a
              WHERE (a.tenant_id, a.event_id) = (p_tenant, p_event))
        ELSE NULL
    END;
$$;
