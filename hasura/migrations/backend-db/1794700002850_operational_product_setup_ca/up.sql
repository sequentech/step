-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-- Dedicated ordinary CA attachment capability. No authority to issue
-- signatures, prepare setup, claim devices, write trust, or create tickets.
DO $role$
BEGIN
    IF NOT EXISTS(SELECT FROM pg_catalog.pg_roles WHERE rolname='sequent_os_product_setup_ca') THEN
        CREATE ROLE sequent_os_product_setup_ca NOLOGIN;
    ELSIF EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname='sequent_os_product_setup_ca'
        AND (rolcanlogin OR rolsuper OR rolcreaterole OR rolcreatedb OR rolreplication OR rolbypassrls)) THEN
        RAISE EXCEPTION 'existing product CA role is not a limited capability';
    END IF;
END;
$role$;
GRANT USAGE ON SCHEMA sequent_backend TO sequent_os_product_setup_ca;
GRANT CREATE ON SCHEMA sequent_backend TO sequent_os_ledger_api_owner;

CREATE FUNCTION sequent_backend.operational_attach_requested_setup_certificate(
    p_tenant uuid,p_event uuid,p_ticket uuid,p_officer uuid,
    p_cert uuid,p_fingerprint text,p_chain text[]
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp
AS $$
DECLARE bound record;
BEGIN
    IF p_cert IS NULL OR p_fingerprint IS NULL
       OR p_fingerprint !~ '^[0-9a-f]{64}$' THEN RETURN false; END IF;
    SELECT * INTO bound FROM sequent_backend.operational_setup_certificate_material(
        p_tenant,p_event,p_ticket,p_officer
    );
    IF NOT FOUND THEN RETURN false; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'operational-enrollment-fingerprint:'||p_tenant::text||':'||p_fingerprint,0));
    IF EXISTS (
        SELECT FROM sequent_backend.operational_signing_official_certificate c
        WHERE c.tenant_id=p_tenant AND c.certificate_fingerprint=p_fingerprint
           AND (c.event_id,c.certificate_id)<>(p_event,p_cert)
    ) THEN RETURN false; END IF;
    IF bound.initial_attempt IS NOT NULL THEN
        RETURN sequent_backend.operational_setup_attach_certificate(
            p_tenant,p_event,bound.official,p_officer,bound.initial_attempt,bound.enrollment,
            p_cert,p_fingerprint,p_chain
        );
    END IF;
    RETURN sequent_backend.operational_setup_attach_existing_event_certificate(
        p_tenant,p_event,bound.post,bound.official,p_officer,bound.enrollment,
        p_cert,p_fingerprint,p_chain,p_ticket
    );
END;
$$;
REVOKE ALL ON FUNCTION sequent_backend.operational_attach_requested_setup_certificate(uuid,uuid,uuid,uuid,uuid,text,text[]) FROM PUBLIC;
ALTER FUNCTION sequent_backend.operational_attach_requested_setup_certificate(uuid,uuid,uuid,uuid,uuid,text,text[])
    OWNER TO sequent_os_ledger_api_owner;
GRANT EXECUTE ON FUNCTION sequent_backend.operational_setup_certificate_material(uuid,uuid,uuid,uuid),
    sequent_backend.operational_attach_requested_setup_certificate(uuid,uuid,uuid,uuid,uuid,text,text[])
    TO sequent_os_product_setup_ca;
REVOKE CREATE ON SCHEMA sequent_backend FROM sequent_os_ledger_api_owner;
