-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Install in the marked normal app DB, after the operator creates the CA
-- login. The six-UUID device probe does not carry a Post or publication.
-- Expose one boolean about the reserved request without granting table reads.
BEGIN;
GRANT CREATE ON SCHEMA sequent_backend TO sequent_os_ledger_api_owner;
-- Only the definer needs these columns to read the current publication.
GRANT SELECT (id, tenant_id, election_event_id, election_id, election_ids, published_at, deleted_at)
    ON sequent_backend.ballot_publication TO sequent_os_ledger_api_owner;
CREATE OR REPLACE FUNCTION sequent_backend.felix3_initialize_ca_request_matches(
    p_tenant uuid, p_event uuid, p_post uuid, p_publication uuid, p_run uuid,
    p_request uuid, p_official uuid, p_challenge uuid, p_fence uuid
) RETURNS boolean LANGUAGE sql STABLE STRICT SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT session_user = 'felix3_local_initialize_ca'
       AND sequent_backend.operational_signing_local_initialize_installation_id(
               p_tenant, p_event, p_post, p_publication, 'softhsm-test') = p_run
       AND p_publication = (
           SELECT id FROM sequent_backend.ballot_publication
           WHERE tenant_id = p_tenant AND election_event_id = p_event
             AND published_at IS NOT NULL AND deleted_at IS NULL
             AND (election_id = p_post OR p_post = ANY(election_ids))
           ORDER BY published_at DESC, id LIMIT 1)
       AND EXISTS (
           SELECT FROM sequent_backend.signing_request r
           JOIN sequent_backend.operational_signing_intent i
             ON (i.tenant_id, i.event_id, i.request_id) = (r.tenant_id, r.election_event_id, r.id)
           WHERE (r.tenant_id, r.election_event_id, r.election_id, r.id)
               = (p_tenant, p_event, p_post, p_request)
             AND (i.actor, i.challenge_id, i.reservation_id) = (p_official::text, p_challenge, p_fence)
             AND i.phase = 'reserved' AND i.purpose = 'approval-payload' AND i.reservation_deadline > clock_timestamp()
             AND r.action = 'initialize-voting' AND r.status = 'waiting' AND r.expires_at > clock_timestamp()
             AND r.subject->>'publication_id' = p_publication::text
             AND (i.binding).payload_hash = r.payload_sha256
             AND octet_length(r.canonical_payload) BETWEEN 1 AND 524288
             AND encode(sha256(convert_to(r.canonical_payload, 'UTF8')), 'hex') = r.payload_sha256
             AND r.canonical_payload::jsonb @> jsonb_build_object(
                 'domain', 'step-signing/v1', 'action', 'initialize-voting',
                 'tenant_id', p_tenant, 'election_event_id', p_event, 'election_id', p_post,
                 'request_id', p_request, 'subject', jsonb_build_object('publication_id', p_publication))
       );
$$;
ALTER FUNCTION sequent_backend.felix3_initialize_ca_request_matches(uuid,uuid,uuid,uuid,uuid,uuid,uuid,uuid,uuid)
    OWNER TO sequent_os_ledger_api_owner;
REVOKE CREATE ON SCHEMA sequent_backend FROM sequent_os_ledger_api_owner;
REVOKE ALL ON FUNCTION sequent_backend.felix3_initialize_ca_request_matches(uuid,uuid,uuid,uuid,uuid,uuid,uuid,uuid,uuid)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION sequent_backend.operational_signing_local_initialize_installation_id(uuid,uuid,uuid,uuid,text)
    TO sequent_os_ledger_api_owner, felix3_local_initialize_ca;
GRANT EXECUTE ON FUNCTION sequent_backend.felix3_initialize_ca_request_matches(uuid,uuid,uuid,uuid,uuid,uuid,uuid,uuid,uuid)
    TO felix3_local_initialize_ca;
COMMIT;
