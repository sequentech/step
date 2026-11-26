-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
SELECT EXISTS (
    SELECT FROM sequent_backend.operational_signing_intent p
    WHERE (p.tenant_id,p.event_id,p.request_id,p.actor) = ($1,$2,$3,$4)
      AND p.phase='applied' AND p.purpose='approval-payload'
      AND sequent_backend.operational_signing_recorded_device_profile(
        p.tenant_id,p.event_id,p.request_id,p.actor,p.reservation_id
      )=$14::text
      AND (p.binding).action=$5 AND (p.binding).payload_hash=$6
      AND (p.binding).cert_fingerprint=$7 AND (p.binding).cert_spki=$8
      AND EXISTS (
        SELECT FROM sequent_backend.operational_signing_get_official_chain(
          p.tenant_id,p.event_id,p.actor,(p.binding).key_id,(p.binding).key_version,$7,$8
        ) AS active WHERE active.certificate_id=$15::uuid AND active.device_profile=$14::text
      )
      AND (p.binding).message_hash=$6 AND p.receipt=$9
      AND p.applied_signature_sha256=encode(sha256($9::bytea),'hex')
      AND ($10::text IS NULL OR EXISTS (
        SELECT FROM sequent_backend.operational_signing_intent d
        WHERE (d.tenant_id,d.event_id,d.request_id,d.actor) = ($1,$2,$3,$4)
          AND d.phase='applied' AND d.purpose=$10
          AND sequent_backend.operational_signing_recorded_device_profile(
            d.tenant_id,d.event_id,d.request_id,d.actor,d.reservation_id
          )=$14::text
          AND (d.binding).action=$5 AND (d.binding).payload_hash=$6
          AND (d.binding).cert_fingerprint=$7 AND (d.binding).cert_spki=$8
          AND (d.binding).key_id=(p.binding).key_id
          AND (d.binding).key_version=(p.binding).key_version
          AND (
            ($10='eml' AND (d.binding).message_hash=$11 AND d.receipt=$12
                AND d.applied_signature_sha256=encode(sha256($12::bytea),'hex'))
            OR ($10='pdf-cms' AND d.applied_signature_sha256=encode(sha256($13::bytea),'hex')
                AND EXISTS (
                    SELECT FROM sequent_backend.signing_document_revision AS signed
                    WHERE (signed.tenant_id,signed.election_event_id,signed.request_id,signed.id)
                        = (d.tenant_id,d.event_id,d.request_id,d.applied_pdf_revision_id)
                      AND signed.state='signed' AND signed.document_id=d.applied_pdf_document_id
                      AND signed.sha256=d.applied_pdf_sha256 AND signed.prepared_for_user=d.actor
                      AND signed.parent_sha256=(d.binding).pdf_parent_hash
                      AND signed.digest_sha256=(d.binding).pdf_digest
                ))
          )
      ))
)
