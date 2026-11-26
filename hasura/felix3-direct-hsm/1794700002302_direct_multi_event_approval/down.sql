-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only

SELECT pg_advisory_xact_lock(hashtextextended('operational-signing-policy:deployment',0));

DO $$
BEGIN
 IF EXISTS (SELECT FROM felix3_initialize_direct.action_request_scope)
    OR EXISTS (SELECT FROM felix3_initialize_direct.action_generation)
    OR EXISTS (SELECT FROM felix3_initialize_direct.action_request_birth)
 THEN RAISE EXCEPTION 'cannot roll back multi-event direct HSM evidence'; END IF;
END;
$$;

CREATE OR REPLACE FUNCTION felix3_initialize_direct.waiting_action(
 p_install uuid,p_tenant uuid,p_event uuid,p_post uuid,p_pub uuid,p_request uuid,p_action text,p_sha bytea)
RETURNS timestamptz LANGUAGE sql VOLATILE STRICT SET search_path=pg_catalog,pg_temp AS $$
 SELECT coalesce(r.expires_at,'infinity'::timestamptz)
 FROM sequent_backend.signing_request r JOIN felix3_initialize_direct.action_installation i ON i.installation=p_install
 WHERE (r.tenant_id,r.election_event_id,r.id,r.action,r.status)=(p_tenant,p_event,p_request,p_action,'waiting')
   AND felix3_initialize_direct.known_action(p_action)
   AND r.created_at >= i.enabled_at AND r.completed_at IS NULL AND r.executed_at IS NULL
   AND (r.expires_at IS NULL OR (isfinite(r.expires_at) AND r.expires_at>clock_timestamp()))
   AND (r.election_id=p_post OR (r.election_id IS NULL AND p_action IN ('approve-configuration','key-ceremony','tally-key')))
   -- Anchored Initialize uses V1 exclusively; continuation is whole-Post and
   -- must name a real, retained subsequent publication in its signed subject.
   AND (p_action<>'initialize-voting' OR (
       r.election_id=p_post AND r.area_id IS NULL AND r.trustee_id IS NULL
       AND EXISTS (
           SELECT FROM sequent_backend.ballot_publication p
           WHERE (p.tenant_id,p.election_event_id)=(p_tenant,p_event)
             AND p.id<>p_pub AND r.subject=jsonb_build_object('publication_id',p.id)
             AND p.is_generated AND p.deleted_at IS NULL
             AND isfinite(p.published_at) AND p.published_at<=statement_timestamp()
             AND EXISTS (SELECT FROM sequent_backend.ballot_style b
                 WHERE (b.tenant_id,b.election_event_id,b.election_id,b.ballot_publication_id)=
                       (p_tenant,p_event,p_post,p.id) AND b.deleted_at IS NULL)
             AND p.published_at >= i.enabled_at
             AND p.id = (SELECT x.id FROM sequent_backend.ballot_publication x
                 WHERE (x.tenant_id,x.election_event_id)=(p_tenant,p_event)
                   AND (x.election_id=p_post OR p_post=ANY(x.election_ids))
                   AND x.deleted_at IS NULL AND x.published_at IS NOT NULL
                 ORDER BY x.published_at DESC,x.id LIMIT 1)
             AND EXISTS (
                 SELECT FROM sequent_backend.signing_request cfg
                 WHERE (cfg.tenant_id,cfg.election_event_id,cfg.action,cfg.status)=
                       (p_tenant,p_event,'approve-configuration','executed')
                   -- Match the Rust authority lookup before applying proof filters:
                   -- a newer invalid execution must not uncover an older proof.
                   AND cfg.id=(SELECT newest.id FROM sequent_backend.signing_request newest
                     WHERE (newest.tenant_id,newest.election_event_id,newest.action,newest.status)=
                           (p_tenant,p_event,'approve-configuration','executed')
                       AND newest.executed_at IS NOT NULL
                       AND newest.subject->>'ballot_publication_id'=p.id::text
                     ORDER BY newest.executed_at DESC,newest.id LIMIT 1)
                   AND (cfg.election_id IS NULL OR cfg.election_id=p_post)
                   AND cfg.area_id IS NULL AND cfg.trustee_id IS NULL
                   AND cfg.required>0 AND cfg.created_at>=i.enabled_at
                   AND cfg.completed_at>=cfg.created_at AND cfg.executed_at>=cfg.completed_at
                   AND cfg.executed_at>=p.published_at AND cfg.executed_at<=statement_timestamp()
                   AND cfg.subject->>'ballot_publication_id'=p.id::text
                   AND cfg.execution_result->>'ballot_publication_id'=p.id::text
                   AND jsonb_typeof(cfg.subject->'digest')='string'
                   AND cfg.execution_result->'digest'=cfg.subject->'digest'
                   AND cfg.subject=cfg.canonical_payload::jsonb->'subject'
                   AND cfg.canonical_payload::jsonb->>'action'='approve-configuration'
                   AND cfg.canonical_payload::jsonb->>'tenant_id'=p_tenant::text
                   AND cfg.canonical_payload::jsonb->>'election_event_id'=p_event::text
                   AND cfg.canonical_payload::jsonb->>'request_id'=cfg.id::text
                   AND encode(sha256(convert_to(cfg.canonical_payload,'UTF8')),'hex')=cfg.payload_sha256
                   AND NOT EXISTS (SELECT FROM sequent_backend.signing_approval a
                       WHERE (a.tenant_id,a.election_event_id,a.request_id)=(p_tenant,p_event,cfg.id)
                         AND (a.action_direct_proof IS NULL OR a.initialize_direct_proof IS NOT NULL))
                   AND EXISTS (
                       SELECT FROM sequent_backend.signing_approval a
                       JOIN felix3_initialize_direct.action_reservation v ON
                            (v.tenant,v.event,v.request,v.actor::text)=
                            (a.tenant_id,a.election_event_id,a.request_id,a.user_id)
                       JOIN felix3_initialize_direct.per_actor_key k ON
                            (k.installation,k.actor)=(v.installation,v.actor)
                       WHERE (a.tenant_id,a.election_event_id,a.request_id)=(p_tenant,p_event,cfg.id)
                         AND (v.installation,v.post,v.publication,v.action)=
                             (p_install,p_post,p_pub,'approve-configuration')
                         AND v.nonce::text=a.action_direct_proof->>'nonce'
                         AND v.used_at IS NOT NULL AND v.used_at<=cfg.completed_at
                         AND a.spki_sha256=encode(k.spki_sha256,'hex') AND k.revoked_at IS NULL
                         AND a.action_direct_proof->>'payload_sha256'=cfg.payload_sha256
                         AND encode(v.payload_sha256,'hex')=cfg.payload_sha256
                       HAVING count(DISTINCT a.user_id)>=cfg.required
                          AND count(DISTINCT k.spki_sha256)>=cfg.required
                   )
             )
       )
   ))
   AND r.payload_sha256=encode(p_sha,'hex') AND sha256(convert_to(r.canonical_payload,'UTF8'))=p_sha
   AND felix3_initialize_direct.action_scope_enabled(p_install,p_tenant,p_event,p_post,p_pub)
   AND NOT EXISTS (SELECT FROM sequent_backend.signing_approval a
       WHERE (a.tenant_id,a.election_event_id,a.request_id)=(p_tenant,p_event,p_request)
         AND a.action_direct_proof IS NULL);
$$;

CREATE OR REPLACE FUNCTION felix3_initialize_direct.proof_action_key(
 p_install uuid,p_tenant uuid,p_event uuid,p_post uuid,p_pub uuid,p_request uuid,p_actor uuid,p_nonce uuid,p_action text,p_sha bytea)
RETURNS bytea LANGUAGE sql VOLATILE SECURITY DEFINER STRICT SET search_path=pg_catalog,pg_temp AS $$
 SELECT k.spki_der FROM felix3_initialize_direct.action_reservation r
 JOIN felix3_initialize_direct.per_actor_key k ON (k.installation,k.actor)=(r.installation,r.actor)
 JOIN sequent_backend.signing_request q ON (q.tenant_id,q.election_event_id,q.id,q.action)=(r.tenant,r.event,r.request,r.action)
 WHERE (r.installation,r.tenant,r.event,r.post,r.publication,r.request,r.actor,r.nonce,r.action,r.payload_sha256)=
    (p_install,p_tenant,p_event,p_post,p_pub,p_request,p_actor,p_nonce,p_action,p_sha)
   AND (r.used_at IS NOT NULL OR r.expires_at>clock_timestamp()) AND k.revoked_at IS NULL
   AND q.payload_sha256=encode(p_sha,'hex') AND sha256(convert_to(q.canonical_payload,'UTF8'))=p_sha
   AND felix3_initialize_direct.action_scope_enabled(p_install,p_tenant,p_event,p_post,p_pub);
$$;

DROP TRIGGER direct_dynamic_action_evidence ON sequent_backend.signing_approval;
DROP FUNCTION felix3_initialize_direct.guard_dynamic_action_evidence();
DROP FUNCTION felix3_initialize_direct.bind_action_request(uuid,uuid,uuid,uuid,uuid,uuid,text,bytea);
DROP FUNCTION felix3_initialize_direct.action_dynamic_target(uuid,uuid,uuid,uuid,uuid,text,jsonb);
DROP FUNCTION felix3_initialize_direct.action_dynamic_configuration_verified(uuid,uuid,uuid,uuid,uuid,timestamptz);
DROP FUNCTION felix3_initialize_direct.action_request_scope(uuid,uuid,uuid);
DROP TRIGGER direct_action_request_scope_immutable ON felix3_initialize_direct.action_request_scope;
DROP FUNCTION felix3_initialize_direct.guard_action_request_scope();
DROP TABLE felix3_initialize_direct.action_request_scope;
DROP TRIGGER direct_action_request_birth ON sequent_backend.signing_request;
DROP FUNCTION felix3_initialize_direct.note_action_request_birth();
DROP TABLE felix3_initialize_direct.action_request_birth;
DROP FUNCTION felix3_initialize_direct.action_generated_candidate(uuid,uuid,uuid);
DROP FUNCTION felix3_initialize_direct.record_action_generation(uuid,uuid,uuid,text);
DROP TABLE felix3_initialize_direct.action_generation;
DROP FUNCTION felix3_initialize_direct.action_device_root(uuid,uuid);
