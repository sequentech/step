-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- The NOLOGIN operational ledger API owner already owns/accesses the
-- device and intent evidence. Add only the columns needed to correlate
-- immutable applied receipts to executed configuration/publication rows.
GRANT CREATE ON SCHEMA sequent_backend TO sequent_os_ledger_api_owner;
GRANT SELECT (tenant_id, election_event_id, request_id, user_id, spki_sha256,
              holder_sha256, fingerprint_sha256, payload_signature)
    ON TABLE sequent_backend.signing_approval TO sequent_os_ledger_api_owner;
GRANT SELECT (tenant_id, election_event_id, approval_request_id, election_id,
              ballot_publication_id)
    ON TABLE sequent_backend.lifecycle_snapshot TO sequent_os_ledger_api_owner;

-- Observes an already authorized deadline, not a currently active key.
-- The approval transaction already verified the active enrolled key,
-- provider, and matching payload/message/artifact. Do not move a published
-- cutoff on a later key rotation or read any private key data here.
CREATE FUNCTION sequent_backend.operational_signing_historical_schedule_proof(
    p_tenant uuid, p_event uuid
) RETURNS TABLE (request_id uuid, hardware_receipts boolean, legacy_only boolean)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT r.id,
           kept.matches AND EXISTS (
               SELECT 1
               FROM sequent_backend.signing_approval AS a
               LEFT JOIN sequent_backend.operational_signing_intent AS i
                 ON (i.tenant_id, i.event_id, i.request_id, i.actor) =
                    (a.tenant_id, a.election_event_id, a.request_id, a.user_id)
                AND i.purpose = 'approval-payload'
               WHERE (a.tenant_id, a.election_event_id, a.request_id) =
                     (r.tenant_id, r.election_event_id, r.id)
               HAVING count(*) >= r.required
                  AND count(*) = count(DISTINCT a.user_id)
                  AND count(*) = count(DISTINCT a.spki_sha256)
                  AND count(*) = count(DISTINCT a.holder_sha256)
                  AND bool_and(coalesce(
                      i.phase = 'applied'
                      AND i.applied_at <= r.executed_at
                      AND (i.binding).action = r.action
                      AND (i.binding).payload_hash = r.payload_sha256
                      AND (i.binding).message_hash = r.payload_sha256
                      AND (i.binding).artifact_hash = r.payload_sha256
                      AND (i.binding).cert_fingerprint = a.fingerprint_sha256
                      AND (i.binding).cert_spki = a.spki_sha256
                      AND i.receipt = a.payload_signature
                      AND i.applied_signature_sha256 = encode(sha256(a.payload_signature), 'hex')
                      AND sequent_backend.operational_signing_recorded_device_profile(
                          i.tenant_id, i.event_id, i.request_id, i.actor, i.reservation_id
                      ) = 'yubihsm',
                      false
                  ))
           ),
           kept.matches AND (
               SELECT count(*) >= r.required
               FROM sequent_backend.signing_approval AS a
               WHERE (a.tenant_id, a.election_event_id, a.request_id) =
                     (r.tenant_id, r.election_event_id, r.id)
           ) AND NOT EXISTS (
               SELECT FROM sequent_backend.operational_signing_intent AS i
               WHERE (i.tenant_id, i.event_id, i.request_id) =
                     (r.tenant_id, r.election_event_id, r.id)
                 AND i.phase = 'applied'
           )
    FROM sequent_backend.signing_request AS r
    CROSS JOIN LATERAL (
        -- The configuration's canonical subject signs its publication, not
        -- the free-standing scope_key. Bind that scope to the same published
        -- snapshot and read it in the same MVCC snapshot as the proof.
        SELECT EXISTS (
            SELECT FROM sequent_backend.lifecycle_snapshot AS s
            WHERE (s.tenant_id, s.election_event_id, s.approval_request_id) =
                  (r.tenant_id, r.election_event_id, r.id)
              AND s.ballot_publication_id::text = r.subject->>'ballot_publication_id'
              -- RequestScope::scope_key for ApproveConfiguration.
              AND r.scope_key = '|||' || coalesce(s.election_id::text, 'event')
        ) AS matches
    ) AS kept
    WHERE (r.tenant_id, r.election_event_id) = (p_tenant, p_event)
      AND r.action = 'approve-configuration' AND r.status = 'executed';
$$;

REVOKE ALL ON FUNCTION sequent_backend.operational_signing_historical_schedule_proof(uuid, uuid)
    FROM PUBLIC;
ALTER FUNCTION sequent_backend.operational_signing_historical_schedule_proof(uuid, uuid)
    OWNER TO sequent_os_ledger_api_owner;
-- The documented Harvest/Windmill operational role receives only these
-- result flags. Installation still assigns the role to exact service logins.
GRANT EXECUTE ON FUNCTION sequent_backend.operational_signing_historical_schedule_proof(uuid, uuid)
    TO sequent_os_approve;

REVOKE CREATE ON SCHEMA sequent_backend FROM sequent_os_ledger_api_owner;
