-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only

SELECT pg_advisory_xact_lock(hashtextextended('operational-signing-policy:deployment',0));

-- The sealed historical event keeps its old V1/V2 APIs. Extension to another
-- event never reuses its Post or publication and never creates another key pool.
CREATE FUNCTION felix3_initialize_direct.action_device_root(p_tenant uuid,p_event uuid)
RETURNS TABLE(installation_id uuid)
LANGUAGE sql STABLE SECURITY DEFINER STRICT SET search_path=pg_catalog,pg_temp AS $$
 SELECT i.installation
 FROM felix3_initialize_direct.installation i
 JOIN felix3_initialize_direct.action_installation a USING(installation)
 JOIN sequent_backend.operational_signing_policy f ON f.singleton AND f.mode='legacy-p12'
 JOIN sequent_backend.election_event e ON (e.tenant_id,e.id)=(p_tenant,p_event)
 WHERE i.tenant=p_tenant AND i.event<>p_event AND i.profile='softhsm-test'
   AND NOT e.is_archived
   AND NOT EXISTS (SELECT FROM sequent_backend.operational_signing_required_tenant t WHERE t.tenant_id=p_tenant)
   AND EXISTS (SELECT FROM sequent_backend.configuration_package p
               WHERE (p.tenant_id,p.election_event_id)=(p_tenant,p_event));
$$;

CREATE TABLE felix3_initialize_direct.action_generation (
 installation uuid NOT NULL REFERENCES felix3_initialize_direct.action_installation(installation),
 tenant uuid NOT NULL,event uuid NOT NULL,post uuid NOT NULL,publication uuid NOT NULL,
 files_root text NOT NULL CHECK(length(files_root) BETWEEN 1 AND 1024),
 recorded_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 PRIMARY KEY(tenant,event,post,publication),
 FOREIGN KEY(tenant,event,post) REFERENCES sequent_backend.election(tenant_id,election_event_id,id),
 FOREIGN KEY(tenant,event,publication) REFERENCES sequent_backend.ballot_publication(tenant_id,election_event_id,id) ON DELETE CASCADE
);

-- Only the explicitly provisioned normal generation worker calls this after
-- completing stock file generation in the same top-level transaction.
-- Archive restore writes similar annotations but never invokes this helper.
CREATE FUNCTION felix3_initialize_direct.record_action_generation(
 p_tenant uuid,p_event uuid,p_pub uuid,p_root text)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path=pg_catalog,pg_temp AS $$
DECLARE chosen uuid; target uuid;
BEGIN
 IF current_setting('transaction_isolation')<>'read committed' THEN RETURN false; END IF;
 PERFORM pg_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment',0));
 PERFORM pg_advisory_xact_lock(hashtextextended('signing-event:'||p_tenant::text||':'||p_event::text,0));
 SELECT installation_id INTO chosen FROM felix3_initialize_direct.action_device_root(p_tenant,p_event);
 IF NOT FOUND THEN RETURN false; END IF;
 SELECT COALESCE(p.election_id,p.election_ids[1]) INTO target FROM sequent_backend.ballot_publication p
 WHERE (p.tenant_id,p.election_event_id,p.id)=(p_tenant,p_event,p_pub)
   AND p.xmin=pg_current_xact_id_if_assigned()::xid
   AND p.is_generated AND p.published_at IS NULL AND p.deleted_at IS NULL
   AND (p.election_id IS NOT NULL OR cardinality(p.election_ids)=1)
   AND p.annotations->>'ballot_files_v1'=p_root
   AND EXISTS (SELECT FROM sequent_backend.configuration_package c
      WHERE (c.tenant_id,c.election_event_id)=(p_tenant,p_event)
      AND c.imported_at<=statement_timestamp());
 IF NOT FOUND THEN RETURN false; END IF;
 INSERT INTO felix3_initialize_direct.action_generation(installation,tenant,event,post,publication,files_root)
 VALUES(chosen,p_tenant,p_event,target,p_pub,p_root);
 RETURN true;
END;
$$;

CREATE FUNCTION felix3_initialize_direct.action_generated_candidate(p_tenant uuid,p_event uuid,p_publication uuid)
RETURNS TABLE(post_id uuid,files_root text)
LANGUAGE sql STABLE SECURITY DEFINER STRICT SET search_path=pg_catalog,pg_temp AS $$
 SELECT g.post,g.files_root
 FROM felix3_initialize_direct.action_generation g
 JOIN felix3_initialize_direct.action_device_root(p_tenant,p_event) i ON i.installation_id=g.installation
 JOIN sequent_backend.ballot_publication p ON (p.tenant_id,p.election_event_id,p.id)=(g.tenant,g.event,g.publication)
 WHERE (g.tenant,g.event,g.publication)=(p_tenant,p_event,p_publication)
   AND p.is_generated AND p.deleted_at IS NULL AND p.published_at IS NULL
   AND (p.election_id=g.post OR (p.election_id IS NULL AND p.election_ids=ARRAY[g.post]))
   AND p.annotations->>'ballot_files_v1'=g.files_root;
$$;
REVOKE ALL ON FUNCTION felix3_initialize_direct.action_generated_candidate(uuid,uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION felix3_initialize_direct.action_generated_candidate(uuid,uuid,uuid) TO sequent_os_approve;

-- An UPDATE also rewrites PostgreSQL xmin. An owner-only INSERT trigger
-- records the actual birth transaction so an old request cannot be relabelled
-- by an update, even in the exact approved tenant and under the right role.
CREATE TABLE felix3_initialize_direct.action_request_birth (
 tenant uuid NOT NULL,event uuid NOT NULL,request uuid NOT NULL,
 birth_tx xid8 NOT NULL,action text COLLATE "C" NOT NULL,
 payload_sha256 text COLLATE "C" NOT NULL CHECK(payload_sha256 ~ '^[0-9a-f]{64}$'),
 PRIMARY KEY(tenant,event,request),
 FOREIGN KEY(tenant,event,request) REFERENCES sequent_backend.signing_request(tenant_id,election_event_id,id)
   ON DELETE CASCADE
);
CREATE FUNCTION felix3_initialize_direct.note_action_request_birth()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
 IF NEW.status='waiting' AND felix3_initialize_direct.known_action(NEW.action) AND EXISTS (
        SELECT FROM felix3_initialize_direct.action_device_root(NEW.tenant_id,NEW.election_event_id))
 THEN INSERT INTO felix3_initialize_direct.action_request_birth(tenant,event,request,birth_tx,action,payload_sha256)
      VALUES(NEW.tenant_id,NEW.election_event_id,NEW.id,pg_current_xact_id(),NEW.action,NEW.payload_sha256);
 END IF;
 RETURN NULL;
END;
$$;
REVOKE ALL ON TABLE felix3_initialize_direct.action_request_birth FROM PUBLIC;
REVOKE ALL ON FUNCTION felix3_initialize_direct.note_action_request_birth() FROM PUBLIC;
CREATE TRIGGER direct_action_request_birth AFTER INSERT ON sequent_backend.signing_request
 FOR EACH ROW EXECUTE FUNCTION felix3_initialize_direct.note_action_request_birth();

CREATE TABLE felix3_initialize_direct.action_request_scope (
 installation uuid NOT NULL REFERENCES felix3_initialize_direct.action_installation(installation),
 tenant uuid NOT NULL,event uuid NOT NULL,post uuid NOT NULL,publication uuid NOT NULL,request uuid NOT NULL,
 action text COLLATE "C" NOT NULL CHECK(felix3_initialize_direct.known_action(action)),
 payload_sha256 bytea NOT NULL CHECK(octet_length(payload_sha256)=32),
 PRIMARY KEY(tenant,event,request),
 FOREIGN KEY(tenant,event,post) REFERENCES sequent_backend.election(tenant_id,election_event_id,id),
 FOREIGN KEY(tenant,event,publication) REFERENCES sequent_backend.ballot_publication(tenant_id,election_event_id,id),
 FOREIGN KEY(tenant,event,request) REFERENCES sequent_backend.signing_request(tenant_id,election_event_id,id)
);

CREATE FUNCTION felix3_initialize_direct.guard_action_request_scope()
RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog,pg_temp AS $$
BEGIN
 RAISE EXCEPTION 'cannot revise an HSM request scope';
END;
$$;
CREATE TRIGGER direct_action_request_scope_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
 ON felix3_initialize_direct.action_request_scope FOR EACH STATEMENT
 EXECUTE FUNCTION felix3_initialize_direct.guard_action_request_scope();

CREATE FUNCTION felix3_initialize_direct.action_request_scope(p_tenant uuid,p_event uuid,p_request uuid)
RETURNS TABLE(installation_id uuid,post_id uuid,publication_id uuid,action text,payload_sha256 bytea)
LANGUAGE sql STABLE SECURITY DEFINER STRICT SET search_path=pg_catalog,pg_temp AS $$
 SELECT s.installation,s.post,s.publication,s.action,s.payload_sha256
 FROM felix3_initialize_direct.action_request_scope s
 WHERE (s.tenant,s.event,s.request)=(p_tenant,p_event,p_request);
$$;

-- A publication is not its own authority. Resolve the newest actual execution
-- first, then evaluate proof; a newer unproved result cannot expose an older one.
CREATE FUNCTION felix3_initialize_direct.action_dynamic_configuration_verified(
 p_install uuid,p_tenant uuid,p_event uuid,p_post uuid,p_pub uuid,p_published_at timestamptz)
RETURNS boolean LANGUAGE sql STABLE STRICT SET search_path=pg_catalog,pg_temp AS $$
 SELECT EXISTS (
  SELECT FROM sequent_backend.signing_request r
  JOIN felix3_initialize_direct.action_request_scope d
    ON (d.tenant,d.event,d.request)=(r.tenant_id,r.election_event_id,r.id)
  JOIN felix3_initialize_direct.action_generation g
    ON (g.installation,g.tenant,g.event,g.post,g.publication)=(d.installation,d.tenant,d.event,d.post,d.publication)
  WHERE (d.installation,d.tenant,d.event,d.post,d.publication,d.action)=
        (p_install,p_tenant,p_event,p_post,p_pub,'approve-configuration')
    AND r.id=(SELECT newest.id FROM sequent_backend.signing_request newest
       WHERE (newest.tenant_id,newest.election_event_id,newest.action,newest.status)=
            (p_tenant,p_event,'approve-configuration','executed')
         AND newest.executed_at IS NOT NULL
         AND newest.subject->>'ballot_publication_id'=p_pub::text
       ORDER BY newest.executed_at DESC,newest.id LIMIT 1)
    AND r.required>0 AND r.completed_at>=r.created_at AND r.executed_at>=r.completed_at
    AND r.executed_at>=p_published_at AND r.executed_at<=statement_timestamp()
    AND r.execution_result->>'ballot_publication_id'=p_pub::text
    AND r.subject->>'digest' ~ '^[0-9a-f]{64}$'
    AND r.execution_result->'digest'=r.subject->'digest'
    AND r.subject->>'ballot_files_root'=g.files_root
    AND r.subject=r.canonical_payload::jsonb->'subject'
    AND encode(d.payload_sha256,'hex')=r.payload_sha256
    AND sha256(convert_to(r.canonical_payload,'UTF8'))=d.payload_sha256
    AND NOT EXISTS (SELECT FROM sequent_backend.signing_approval a
       WHERE (a.tenant_id,a.election_event_id,a.request_id)=(p_tenant,p_event,r.id)
         AND (a.action_direct_proof IS NULL OR a.initialize_direct_proof IS NOT NULL))
    AND EXISTS (
      SELECT FROM sequent_backend.signing_approval a
      JOIN felix3_initialize_direct.action_reservation v
        ON (v.tenant,v.event,v.request,v.actor::text)=(a.tenant_id,a.election_event_id,a.request_id,a.user_id)
      JOIN felix3_initialize_direct.per_actor_key k ON (k.installation,k.actor)=(v.installation,v.actor)
      WHERE (v.installation,v.tenant,v.event,v.post,v.publication,v.request,v.action,v.payload_sha256)=
            (p_install,p_tenant,p_event,p_post,p_pub,r.id,'approve-configuration',d.payload_sha256)
        AND v.nonce::text=a.action_direct_proof->>'nonce'
        AND v.used_at IS NOT NULL AND v.used_at<=r.completed_at
        AND a.spki_sha256=encode(k.spki_sha256,'hex')
        AND (k.revoked_at IS NULL OR k.revoked_at>r.executed_at)
      HAVING count(DISTINCT a.user_id)>=r.required AND count(DISTINCT k.spki_sha256)>=r.required
    )
 );
$$;

-- Active scope, re-used when looking up pending signatures. A completed
-- configuration's own old receipts remain readable without authorizing a new
-- signature on a different publication.
CREATE FUNCTION felix3_initialize_direct.action_dynamic_target(
 p_install uuid,p_tenant uuid,p_event uuid,p_post uuid,p_pub uuid,p_action text,p_subject jsonb)
RETURNS boolean LANGUAGE sql STABLE STRICT SET search_path=pg_catalog,pg_temp AS $$
 SELECT EXISTS (
   SELECT FROM felix3_initialize_direct.action_device_root(p_tenant,p_event) i
   JOIN sequent_backend.ballot_publication p ON (p.tenant_id,p.election_event_id,p.id)=(p_tenant,p_event,p_pub)
   JOIN felix3_initialize_direct.action_generation g
        ON (g.installation,g.tenant,g.event,g.post,g.publication)=(p_install,p_tenant,p_event,p_post,p_pub)
   WHERE i.installation_id=p_install
     AND p.is_generated AND p.deleted_at IS NULL
     AND (p.election_id=p_post OR (p.election_id IS NULL AND p.election_ids=ARRAY[p_post]))
     AND p.annotations->>'ballot_files_v1'=g.files_root
     AND CASE WHEN p_action='approve-configuration'
        THEN p.published_at IS NULL
          AND p_subject->>'ballot_publication_id'=p_pub::text
          AND p_subject->>'ballot_files_root'=g.files_root AND p_subject->>'digest' ~ '^[0-9a-f]{64}$'
        ELSE isfinite(p.published_at) AND p.published_at<=statement_timestamp()
          AND (p_action<>'initialize-voting' OR p_subject=jsonb_build_object('publication_id',p_pub))
          AND p.id=(SELECT x.id FROM sequent_backend.ballot_publication x
             WHERE (x.tenant_id,x.election_event_id)=(p_tenant,p_event)
               AND (x.election_id=p_post OR p_post=ANY(x.election_ids)) AND x.deleted_at IS NULL
               AND x.published_at IS NOT NULL ORDER BY x.published_at DESC,x.id LIMIT 1)
          AND EXISTS (SELECT FROM sequent_backend.ballot_style s
             WHERE (s.tenant_id,s.election_event_id,s.election_id,s.ballot_publication_id)=(p_tenant,p_event,p_post,p_pub)
               AND s.deleted_at IS NULL)
          AND felix3_initialize_direct.action_dynamic_configuration_verified(p_install,p_tenant,p_event,p_post,p_pub,p.published_at)
        END
 );
$$;

CREATE FUNCTION felix3_initialize_direct.bind_action_request(
 p_install uuid,p_tenant uuid,p_event uuid,p_post uuid,p_pub uuid,p_request uuid,p_action text,p_sha bytea)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path=pg_catalog,pg_temp AS $$
BEGIN
 IF current_setting('transaction_isolation')<>'read committed' OR octet_length(p_sha)<>32 THEN RETURN false; END IF;
 PERFORM pg_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment',0));
 PERFORM pg_advisory_xact_lock(hashtextextended('signing-event:'||p_tenant::text||':'||p_event::text,0));
 INSERT INTO felix3_initialize_direct.action_request_scope(installation,tenant,event,post,publication,request,action,payload_sha256)
 SELECT p_install,p_tenant,p_event,p_post,p_pub,p_request,p_action,p_sha
 FROM sequent_backend.signing_request r
 WHERE (r.tenant_id,r.election_event_id,r.id,r.action,r.status)=(p_tenant,p_event,p_request,p_action,'waiting')
   AND r.xmin=pg_current_xact_id_if_assigned()::xid AND r.required>0 AND r.completed_at IS NULL AND r.executed_at IS NULL
   AND EXISTS (SELECT FROM felix3_initialize_direct.action_request_birth b
      WHERE (b.tenant,b.event,b.request,b.action,b.payload_sha256)=
            (p_tenant,p_event,p_request,p_action,encode(p_sha,'hex'))
        AND b.birth_tx=pg_current_xact_id_if_assigned())
   AND felix3_initialize_direct.known_action(p_action)
   AND (CASE WHEN p_action='approve-configuration' THEN r.election_id IS NULL AND EXISTS (
       SELECT FROM sequent_backend.ballot_publication p
       WHERE (p.tenant_id,p.election_event_id,p.id)=(p_tenant,p_event,p_pub)
         AND ((p.election_id=p_post AND r.scope_key='|||'||p_post::text)
           OR (p.election_id IS NULL AND p.election_ids=ARRAY[p_post] AND r.scope_key='|||event'
              AND NOT EXISTS (SELECT FROM sequent_backend.election e
                 WHERE (e.tenant_id,e.election_event_id)=(p_tenant,p_event) AND e.id<>p_post))))
       ELSE r.election_id=p_post OR (r.election_id IS NULL
            AND p_action IN ('key-ceremony','tally-key') AND r.area_id IS NULL
            AND NOT EXISTS (SELECT FROM sequent_backend.election e
               WHERE (e.tenant_id,e.election_event_id)=(p_tenant,p_event) AND e.id<>p_post))
       END)
   AND (p_action NOT IN ('initialize-voting','approve-configuration') OR (r.area_id IS NULL AND r.trustee_id IS NULL))
   AND r.subject=r.canonical_payload::jsonb->'subject'
   AND r.canonical_payload::jsonb->>'action'=p_action
   AND r.canonical_payload::jsonb->>'request_id'=p_request::text
   AND r.canonical_payload::jsonb->>'tenant_id'=p_tenant::text
   AND r.canonical_payload::jsonb->>'election_event_id'=p_event::text
   AND r.payload_sha256=encode(p_sha,'hex') AND sha256(convert_to(r.canonical_payload,'UTF8'))=p_sha
   AND NOT EXISTS (SELECT FROM sequent_backend.signing_approval a
       WHERE (a.tenant_id,a.election_event_id,a.request_id)=(p_tenant,p_event,p_request))
   AND felix3_initialize_direct.action_dynamic_target(p_install,p_tenant,p_event,p_post,p_pub,p_action,r.subject)
 ON CONFLICT DO NOTHING;
 RETURN FOUND;
END;
$$;

-- Final SQL evidence backstop. The old guard still verifies the actual
-- consumed nonce and actor key, including V1 on its original historical event.
CREATE FUNCTION felix3_initialize_direct.guard_dynamic_action_evidence()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
DECLARE d felix3_initialize_direct.action_request_scope%ROWTYPE;
BEGIN
 IF EXISTS (SELECT FROM felix3_initialize_direct.action_device_root(NEW.tenant_id,NEW.election_event_id)) THEN
    IF current_setting('transaction_isolation')<>'read committed'
    THEN RAISE EXCEPTION 'multi-event direct HSM policy requires read committed' USING ERRCODE='25001'; END IF;
    PERFORM pg_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment',0));
    PERFORM pg_advisory_xact_lock(hashtextextended('signing-event:'||NEW.tenant_id::text||':'||NEW.election_event_id::text,0));
 END IF;
 SELECT s.* INTO d FROM felix3_initialize_direct.action_request_scope s
 WHERE (s.tenant,s.event,s.request)=(NEW.tenant_id,NEW.election_event_id,NEW.request_id);
 IF NOT FOUND THEN RETURN NEW; END IF;
 IF NEW.action_direct_proof IS NULL OR NEW.initialize_direct_proof IS NOT NULL
    OR ROW(NEW.action_direct_proof->>'installation',NEW.action_direct_proof->>'post',
        NEW.action_direct_proof->>'publication',NEW.action_direct_proof->>'action',
        NEW.action_direct_proof->>'payload_sha256') IS DISTINCT FROM
       ROW(d.installation::text,d.post::text,d.publication::text,d.action::text,encode(d.payload_sha256,'hex'))
 THEN RAISE EXCEPTION 'evidence does not match its frozen multi-event HSM scope' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END;
$$;
CREATE TRIGGER direct_dynamic_action_evidence BEFORE INSERT OR UPDATE OF
 action_direct_proof,initialize_direct_proof,user_id,request_id,tenant_id,election_event_id
 ON sequent_backend.signing_approval FOR EACH ROW
 EXECUTE FUNCTION felix3_initialize_direct.guard_dynamic_action_evidence();

REVOKE ALL ON TABLE felix3_initialize_direct.action_generation,felix3_initialize_direct.action_request_scope FROM PUBLIC;
REVOKE ALL ON FUNCTION felix3_initialize_direct.action_device_root(uuid,uuid),
 felix3_initialize_direct.record_action_generation(uuid,uuid,uuid,text),
 felix3_initialize_direct.guard_action_request_scope(),
 felix3_initialize_direct.action_request_scope(uuid,uuid,uuid),
 felix3_initialize_direct.action_dynamic_configuration_verified(uuid,uuid,uuid,uuid,uuid,timestamptz),
 felix3_initialize_direct.action_dynamic_target(uuid,uuid,uuid,uuid,uuid,text,jsonb),
 felix3_initialize_direct.bind_action_request(uuid,uuid,uuid,uuid,uuid,uuid,text,bytea),
 felix3_initialize_direct.guard_dynamic_action_evidence() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION felix3_initialize_direct.action_device_root(uuid,uuid),
 felix3_initialize_direct.action_request_scope(uuid,uuid,uuid),
 felix3_initialize_direct.bind_action_request(uuid,uuid,uuid,uuid,uuid,uuid,text,bytea) TO sequent_os_approve;

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
         AND a.action_direct_proof IS NULL)
 UNION ALL
 SELECT coalesce(r.expires_at,'infinity'::timestamptz)
 FROM felix3_initialize_direct.action_request_scope d
 JOIN sequent_backend.signing_request r ON (r.tenant_id,r.election_event_id,r.id,r.action)=(d.tenant,d.event,d.request,d.action)
 WHERE (d.installation,d.tenant,d.event,d.post,d.publication,d.request,d.action,d.payload_sha256)=
       (p_install,p_tenant,p_event,p_post,p_pub,p_request,p_action,p_sha)
   AND r.status='waiting' AND r.completed_at IS NULL AND r.executed_at IS NULL
   AND (r.expires_at IS NULL OR (isfinite(r.expires_at) AND r.expires_at>clock_timestamp()))
   AND r.payload_sha256=encode(p_sha,'hex') AND sha256(convert_to(r.canonical_payload,'UTF8'))=p_sha
   AND r.subject=r.canonical_payload::jsonb->'subject'
   AND felix3_initialize_direct.action_dynamic_target(p_install,p_tenant,p_event,p_post,p_pub,p_action,r.subject)
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
   AND felix3_initialize_direct.action_scope_enabled(p_install,p_tenant,p_event,p_post,p_pub)
 UNION ALL
 SELECT k.spki_der
 FROM felix3_initialize_direct.action_request_scope d
 JOIN felix3_initialize_direct.action_reservation r
   ON (r.installation,r.tenant,r.event,r.post,r.publication,r.request,r.action,r.payload_sha256)=
      (d.installation,d.tenant,d.event,d.post,d.publication,d.request,d.action,d.payload_sha256)
 JOIN felix3_initialize_direct.per_actor_key k ON (k.installation,k.actor)=(r.installation,r.actor)
 JOIN sequent_backend.signing_request q ON (q.tenant_id,q.election_event_id,q.id,q.action)=(d.tenant,d.event,d.request,d.action)
 WHERE (d.installation,d.tenant,d.event,d.post,d.publication,d.request,d.action,d.payload_sha256)=
       (p_install,p_tenant,p_event,p_post,p_pub,p_request,p_action,p_sha)
   AND (r.actor,r.nonce)=(p_actor,p_nonce)
   AND (r.used_at IS NOT NULL OR r.expires_at>clock_timestamp())
   AND q.payload_sha256=encode(p_sha,'hex') AND sha256(convert_to(q.canonical_payload,'UTF8'))=p_sha
   AND q.subject=q.canonical_payload::jsonb->'subject'
   AND EXISTS (SELECT FROM felix3_initialize_direct.action_device_root(p_tenant,p_event) i
               WHERE i.installation_id=p_install)
   AND CASE WHEN q.action='approve-configuration' AND q.status='executed' THEN
       r.used_at IS NOT NULL AND (k.revoked_at IS NULL OR k.revoked_at>q.executed_at) AND EXISTS (
       SELECT FROM sequent_backend.ballot_publication p
       WHERE (p.tenant_id,p.election_event_id,p.id)=(p_tenant,p_event,p_pub)
          AND felix3_initialize_direct.action_dynamic_configuration_verified(p_install,p_tenant,p_event,p_post,p_pub,p.published_at))
       ELSE k.revoked_at IS NULL AND
            felix3_initialize_direct.action_dynamic_target(p_install,p_tenant,p_event,p_post,p_pub,p_action,q.subject)
       END;
$$;
