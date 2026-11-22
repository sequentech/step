-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Operational signing ledger provision. Not tracked in Hasura.
--
-- NOLOGIN group roles are cluster objects: they are created here only when
-- missing, are never granted to PUBLIC or to any login, and are left in place
-- by the rollback. The installer grants each one to exactly one dedicated
-- login; a runtime login must never be a member of the two owner roles.
--
--   sequent_os_ledger_data_owner       owns the ledger tables added from here on
--   sequent_os_ledger_api_owner        owns the SECURITY DEFINER ledger API
--   sequent_os_approve                 Harvest/Windmill approval and MIRU callers
--   sequent_os_setup_authority         Harvest Security Officer setup routes
--   sequent_os_setup_device            private initial-key hardware worker
--   sequent_os_initialize_orchestrator private Initialize ceremony broker
--   sequent_os_initialize_ca           private CA assessment service
--   sequent_os_initialize_device       private Initialize device worker
--   sequent_os_device                  private generic device worker
DO $$
DECLARE
    v_name text;
    v_role pg_catalog.pg_roles%ROWTYPE;
BEGIN
    FOREACH v_name IN ARRAY ARRAY[
        'sequent_os_ledger_data_owner', 'sequent_os_ledger_api_owner',
        'sequent_os_approve', 'sequent_os_setup_authority', 'sequent_os_setup_device',
        'sequent_os_initialize_orchestrator', 'sequent_os_initialize_ca',
        'sequent_os_initialize_device', 'sequent_os_device'
    ] LOOP
        IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = v_name) THEN
            -- Roles are cluster-wide: another database migrating at the same
            -- time may create it first, which is checked below like any other.
            BEGIN
                EXECUTE pg_catalog.format(
                    'CREATE ROLE %I NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS',
                    v_name);
            EXCEPTION WHEN duplicate_object OR unique_violation THEN
                NULL;
            END;
        END IF;
        SELECT * INTO STRICT v_role FROM pg_catalog.pg_roles WHERE rolname = v_name;
        IF v_role.rolcanlogin OR v_role.rolsuper OR v_role.rolcreatedb OR v_role.rolcreaterole
              OR v_role.rolreplication OR v_role.rolbypassrls THEN
            RAISE EXCEPTION 'role % exists with login or administrative attributes', v_name;
        END IF;
    END LOOP;
    -- No group role may reach another one through membership.
    IF EXISTS (
        SELECT FROM pg_catalog.pg_auth_members AS m
        JOIN pg_catalog.pg_roles AS member ON member.oid = m.member
        JOIN pg_catalog.pg_roles AS granted ON granted.oid = m.roleid
        WHERE member.rolname LIKE 'sequent\_os\_%' AND granted.rolname LIKE 'sequent\_os\_%'
    ) THEN
        RAISE EXCEPTION 'operational signing group roles must not be members of each other';
    END IF;
    -- A migrator that is not a superuser must be able to hand objects to the
    -- owner roles and to drop them again on rollback.
    IF NOT (SELECT rolsuper FROM pg_catalog.pg_roles WHERE rolname = current_user) THEN
        EXECUTE pg_catalog.format(
            'GRANT sequent_os_ledger_data_owner, sequent_os_ledger_api_owner TO %I WITH INHERIT TRUE, SET TRUE',
            current_user);
    END IF;
END;
$$;

GRANT USAGE ON SCHEMA sequent_backend TO
    sequent_os_ledger_data_owner, sequent_os_ledger_api_owner, sequent_os_approve,
    sequent_os_setup_authority, sequent_os_setup_device, sequent_os_initialize_orchestrator,
    sequent_os_initialize_ca, sequent_os_initialize_device, sequent_os_device;
-- ALTER ... OWNER needs CREATE on the schema for the new owner unless the
-- migrator is a superuser. Revoked again at the end of each migration.
GRANT CREATE ON SCHEMA sequent_backend TO sequent_os_ledger_api_owner;

-- What the ledger API owner needs on the objects of 1794700000700-900. Those
-- objects keep their owner. Row locks need UPDATE on one column: the stored
-- generated column can never be assigned, so the grant is a lock-only one.
GRANT USAGE ON TYPE sequent_backend.operational_signing_binding TO sequent_os_ledger_api_owner;
GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE sequent_backend.operational_signing_intent
    TO sequent_os_ledger_api_owner;
GRANT SELECT, INSERT ON TABLE sequent_backend.operational_signing_ceremony
    TO sequent_os_ledger_api_owner;
GRANT SELECT ON TABLE
    sequent_backend.operational_signing_policy,
    sequent_backend.operational_signing_required_tenant,
    sequent_backend.operational_signing_official_enrollment,
    sequent_backend.operational_signing_official_certificate,
    sequent_backend.signing_request,
    sequent_backend.signing_document_revision,
    sequent_backend.staff_certificate,
    sequent_backend.certificate_authority,
    sequent_backend.staff_crl,
    sequent_backend.tenant,
    sequent_backend.election_event,
    sequent_backend.election,
    sequent_backend.area,
    sequent_backend.tally_session
    TO sequent_os_ledger_api_owner;
GRANT UPDATE (subject_publication_id) ON TABLE sequent_backend.signing_request
    TO sequent_os_ledger_api_owner;
GRANT EXECUTE ON FUNCTION
    sequent_backend.operational_signing_validate(sequent_backend.operational_signing_binding),
    sequent_backend.operational_signing_prepare(sequent_backend.operational_signing_binding),
    sequent_backend.operational_signing_discard_prepared(sequent_backend.operational_signing_binding),
    sequent_backend.operational_signing_reserve(sequent_backend.operational_signing_binding, bigint, timestamptz),
    sequent_backend.operational_signing_claim_device(sequent_backend.operational_signing_binding, uuid),
    sequent_backend.operational_signing_settle(sequent_backend.operational_signing_binding, uuid, bytea),
    sequent_backend.operational_signing_enroll_official(uuid, text, text, integer, text, text, boolean, text),
    sequent_backend.operational_signing_bind_official_certificate(uuid, uuid, text, uuid, uuid, text, text[])
    TO sequent_os_ledger_api_owner;

-- The approval caller has no DML on the ledger. Its two entry points become
-- SECURITY DEFINER under their unchanged owner and pinned search_path; the
-- consumption-side quorum check reads exactly these applied-evidence columns.
ALTER FUNCTION sequent_backend.operational_signing_get_completed(uuid, uuid, uuid, text, uuid)
    SECURITY DEFINER;
ALTER FUNCTION sequent_backend.operational_signing_mark_applied_from_evidence(uuid, uuid, uuid, text, uuid, uuid, bytea, bytea)
    SECURITY DEFINER;
GRANT EXECUTE ON FUNCTION
    sequent_backend.operational_signing_get_completed(uuid, uuid, uuid, text, uuid),
    sequent_backend.operational_signing_mark_applied_from_evidence(uuid, uuid, uuid, text, uuid, uuid, bytea, bytea),
    sequent_backend.operational_signing_get_official_chain(uuid, uuid, text, text, integer, text, text)
    TO sequent_os_approve;
GRANT SELECT (tenant_id, event_id, request_id, actor, phase, purpose, binding, receipt,
              reservation_id, applied_signature_sha256, applied_pdf_revision_id,
              applied_pdf_document_id, applied_pdf_sha256)
    ON TABLE sequent_backend.operational_signing_intent TO sequent_os_approve;
GRANT SELECT ON TABLE sequent_backend.operational_signing_policy,
    sequent_backend.operational_signing_required_tenant
    TO sequent_os_approve, sequent_os_setup_authority;

-- Internal: true only for a known tenant that the deployment policy or the
-- tenant list requires to sign with hardware. The policy is monotonic, so a
-- concurrent promotion can only be missed in the refusing direction.
CREATE FUNCTION sequent_backend.operational_signing_hsm_required(p_tenant uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY INVOKER
SET search_path = pg_catalog, sequent_backend, pg_temp
AS $$
    SELECT p_tenant IS NOT NULL
       AND EXISTS (SELECT FROM sequent_backend.tenant AS t WHERE t.id = p_tenant)
       AND (EXISTS (SELECT FROM sequent_backend.operational_signing_policy AS p
                    WHERE p.singleton AND p.mode = 'hsm-required')
            OR EXISTS (SELECT FROM sequent_backend.operational_signing_required_tenant AS q
                       WHERE q.tenant_id = p_tenant));
$$;
REVOKE ALL ON FUNCTION sequent_backend.operational_signing_hsm_required(uuid) FROM PUBLIC;
ALTER FUNCTION sequent_backend.operational_signing_hsm_required(uuid)
    OWNER TO sequent_os_ledger_api_owner;

REVOKE CREATE ON SCHEMA sequent_backend FROM sequent_os_ledger_api_owner;
