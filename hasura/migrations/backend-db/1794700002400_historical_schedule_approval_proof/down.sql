-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP FUNCTION sequent_backend.operational_signing_historical_schedule_proof(uuid, uuid);
REVOKE SELECT (tenant_id, election_event_id, approval_request_id, election_id,
               ballot_publication_id)
    ON TABLE sequent_backend.lifecycle_snapshot FROM sequent_os_ledger_api_owner;
REVOKE SELECT (tenant_id, election_event_id, request_id, user_id, spki_sha256,
               holder_sha256, fingerprint_sha256, payload_signature)
    ON TABLE sequent_backend.signing_approval FROM sequent_os_ledger_api_owner;
