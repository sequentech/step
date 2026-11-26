-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Restores the 1794700001400 definitions verbatim. CREATE OR REPLACE keeps
-- their owners and EXECUTE privileges. Refuses while a legacy-tenant event
-- has registered artifacts: the old definitions would release them.
LOCK TABLE sequent_backend.operational_miru_artifact IN ACCESS EXCLUSIVE MODE;
DO $$
BEGIN
    IF EXISTS (SELECT FROM sequent_backend.operational_miru_artifact AS a
               WHERE NOT sequent_backend.operational_signing_hsm_required(a.tenant_id)) THEN
        RAISE EXCEPTION 'operational MIRU direct-scope artifacts exist: refuse rollback';
    END IF;
END;
$$;

CREATE OR REPLACE FUNCTION sequent_backend.operational_miru_register_artifact(
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

CREATE OR REPLACE FUNCTION sequent_backend.operational_miru_generic_withhold(
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
        ELSE NULL
    END;
$$;

DROP FUNCTION sequent_backend.operational_miru_direct_scope(uuid, uuid, uuid);
