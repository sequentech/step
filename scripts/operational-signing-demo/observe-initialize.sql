-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-- Read-only oracle for exactly one fresh two-official Initialize Voting test.
-- Use a separate fixture observer/operator session, not a service login.
\set ON_ERROR_STOP 1
BEGIN TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY;
SELECT 1 / CASE WHEN current_database()='operational_demo_14990' AND inet_client_addr() IS NULL
    AND :'expected'::int IN (0,1,2) AND :'first'::uuid <> :'second'::uuid
    AND sequent_backend.operational_signing_demo_fixture_id(
        :'tenant',:'event','initialize-voting',:'backend') = :'run'::uuid
    THEN 1 ELSE 0 END AS admitted \gset
WITH q AS (
    SELECT * FROM sequent_backend.signing_request
    WHERE (tenant_id,election_event_id,id,election_id) = (:'tenant',:'event',:'request',:'post')
      AND action='initialize-voting' AND required=2 AND payload_sha256=:'hash'
      AND encode(sha256(convert_to(canonical_payload,'UTF8')),'hex')=:'hash'
), ledger AS (
    -- The device settles separately from applying an approval. Count every
    -- receipt and unresolved device use on this request, not just applied rows.
    SELECT count(*) FILTER (WHERE i.receipt IS NOT NULL) AS device_receipts,
           count(*) FILTER (WHERE i.phase IN ('reserved','invoked','indeterminate')) AS unsettled
    FROM q JOIN sequent_backend.operational_signing_intent i
      ON (i.tenant_id,i.event_id,i.request_id)=(q.tenant_id,q.election_event_id,q.id)
), counted AS (
    SELECT count(*) AS approvals, count(DISTINCT a.user_id) AS people,
      count(*) FILTER (WHERE (a.user_id=:'first' OR (:'expected'::int=2 AND a.user_id=:'second')) AND EXISTS (
        SELECT FROM sequent_backend.operational_signing_intent i
         WHERE (i.tenant_id,i.event_id,i.request_id,i.actor)=
               (a.tenant_id,a.election_event_id,a.request_id,a.user_id)
           AND i.phase='applied' AND i.purpose='approval-payload'
           AND (i.binding).action=q.action AND (i.binding).payload_hash=q.payload_sha256
           AND (i.binding).message_hash=q.payload_sha256 AND (i.binding).cert_spki=a.spki_sha256
           AND (i.binding).cert_fingerprint=a.fingerprint_sha256 AND i.receipt=a.payload_signature
           AND i.applied_signature_sha256=encode(sha256(a.payload_signature),'hex')
           AND sequent_backend.operational_signing_recorded_device_profile(
               i.tenant_id,i.event_id,i.request_id,i.actor,i.reservation_id)=:'backend'
      )) AS backed
    FROM q JOIN sequent_backend.signing_approval a
      ON (a.tenant_id,a.election_event_id,a.request_id)=(q.tenant_id,q.election_event_id,q.id)
), effect AS (
    SELECT count(*) AS scoped, count(*) FILTER (WHERE t.id::text=q.execution_result->>'tally_session_id') AS linked
    FROM q JOIN sequent_backend.tally_session t
      ON (t.tenant_id,t.election_event_id)=(q.tenant_id,q.election_event_id)
     AND t.tally_type='INITIALIZATION_REPORT' AND t.created_at>=q.created_at AND t.election_ids=ARRAY[q.election_id]
     AND coalesce(t.area_ids,ARRAY[]::uuid[]) =
         CASE WHEN q.area_id IS NULL THEN ARRAY[]::uuid[] ELSE ARRAY[q.area_id] END
)
SELECT approvals, people, backed, scoped, linked, device_receipts, unsettled,
       ((SELECT count(*) FROM q)=1
       AND approvals=:'expected'::int AND people=approvals AND backed=approvals
       AND device_receipts=approvals AND unsettled=0
       AND scoped=CASE WHEN :'expected'::int=2 THEN 1 ELSE 0 END AND linked=scoped
       AND (SELECT status FROM q)=CASE WHEN :'expected'::int=2 THEN 'executed' ELSE 'waiting' END) AS matches
FROM counted,effect,ledger \gset
\echo approvals=:approvals distinct_officials=:people applied_device_receipts=:backed post_effects=:scoped linked_effects=:linked
\echo recorded_device_receipts=:device_receipts unsettled_device_uses=:unsettled
-- SQL error is intentional on a mismatch, including with older psql clients
-- that ignore nonzero exit codes on their backslash quit command.
SELECT 1 / CASE WHEN :'matches'::boolean THEN 1 ELSE 0 END AS verified;
COMMIT;
