-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Run in a transaction. Removes every privilege this migration granted in
-- this database. The NOLOGIN group roles are cluster objects that other
-- databases and installer-managed memberships may use: they stay in place.
DROP FUNCTION sequent_backend.operational_signing_hsm_required(uuid);

REVOKE SELECT ON TABLE sequent_backend.operational_signing_policy,
    sequent_backend.operational_signing_required_tenant
    FROM sequent_os_approve, sequent_os_setup_authority;
REVOKE SELECT (tenant_id, event_id, request_id, actor, phase, purpose, binding, receipt,
               reservation_id, applied_signature_sha256, applied_pdf_revision_id,
               applied_pdf_document_id, applied_pdf_sha256)
    ON TABLE sequent_backend.operational_signing_intent FROM sequent_os_approve;
REVOKE EXECUTE ON FUNCTION
    sequent_backend.operational_signing_get_completed(uuid, uuid, uuid, text, uuid),
    sequent_backend.operational_signing_mark_applied_from_evidence(uuid, uuid, uuid, text, uuid, uuid, bytea, bytea),
    sequent_backend.operational_signing_get_official_chain(uuid, uuid, text, text, integer, text, text)
    FROM sequent_os_approve;
ALTER FUNCTION sequent_backend.operational_signing_mark_applied_from_evidence(uuid, uuid, uuid, text, uuid, uuid, bytea, bytea)
    SECURITY INVOKER;
ALTER FUNCTION sequent_backend.operational_signing_get_completed(uuid, uuid, uuid, text, uuid)
    SECURITY INVOKER;

REVOKE EXECUTE ON FUNCTION
    sequent_backend.operational_signing_validate(sequent_backend.operational_signing_binding),
    sequent_backend.operational_signing_prepare(sequent_backend.operational_signing_binding),
    sequent_backend.operational_signing_discard_prepared(sequent_backend.operational_signing_binding),
    sequent_backend.operational_signing_reserve(sequent_backend.operational_signing_binding, bigint, timestamptz),
    sequent_backend.operational_signing_claim_device(sequent_backend.operational_signing_binding, uuid),
    sequent_backend.operational_signing_settle(sequent_backend.operational_signing_binding, uuid, bytea),
    sequent_backend.operational_signing_enroll_official(uuid, text, text, integer, text, text, boolean, text),
    sequent_backend.operational_signing_bind_official_certificate(uuid, uuid, text, uuid, uuid, text, text[])
    FROM sequent_os_ledger_api_owner;
REVOKE UPDATE (subject_publication_id) ON TABLE sequent_backend.signing_request
    FROM sequent_os_ledger_api_owner;
REVOKE SELECT ON TABLE
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
    FROM sequent_os_ledger_api_owner;
REVOKE SELECT, INSERT ON TABLE sequent_backend.operational_signing_ceremony
    FROM sequent_os_ledger_api_owner;
REVOKE SELECT, INSERT, UPDATE, DELETE ON TABLE sequent_backend.operational_signing_intent
    FROM sequent_os_ledger_api_owner;
REVOKE USAGE ON TYPE sequent_backend.operational_signing_binding FROM sequent_os_ledger_api_owner;
REVOKE ALL ON SCHEMA sequent_backend FROM
    sequent_os_ledger_data_owner, sequent_os_ledger_api_owner, sequent_os_approve,
    sequent_os_setup_authority, sequent_os_setup_device, sequent_os_initialize_orchestrator,
    sequent_os_initialize_ca, sequent_os_initialize_device, sequent_os_device;
