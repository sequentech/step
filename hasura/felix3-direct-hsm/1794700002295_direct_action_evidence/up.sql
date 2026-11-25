-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only
-- Optional direct DEMO action evidence. No certificate, CRL, document
-- signature, installation, device credential or product policy is created.
SELECT pg_advisory_xact_lock(hashtextextended('operational-signing-policy:deployment',0));

ALTER TABLE sequent_backend.signing_approval ADD COLUMN action_direct_proof jsonb;

CREATE FUNCTION felix3_initialize_direct.known_action(p_action text)
RETURNS boolean LANGUAGE sql IMMUTABLE STRICT SET search_path=pg_catalog,pg_temp AS $$
 SELECT p_action COLLATE "C" IN ('initialize-voting','open-voting','close-voting',
   'generate-election-returns','generate-reports','transmit-results','approve-voter',
   'approve-configuration','key-ceremony','tally-key');
$$;
REVOKE ALL ON FUNCTION felix3_initialize_direct.known_action(text) FROM PUBLIC;

-- Strict short-form DER and minimal positive INTEGER encodings. Cryptographic
-- verification with the independently bound key is still required in Rust.
CREATE FUNCTION felix3_initialize_direct.canonical_action_ecdsa(p_der bytea)
RETURNS boolean LANGUAGE plpgsql IMMUTABLE STRICT SET search_path=pg_catalog,pg_temp AS $$
DECLARE at_byte int := 2; n int; i int; bits bytea;
        curve_order bytea:=decode('ffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551','hex');
BEGIN
 IF octet_length(p_der) NOT BETWEEN 8 AND 72 THEN RETURN false; END IF;
 IF get_byte(p_der,0)<>48 OR get_byte(p_der,1)<>octet_length(p_der)-2 THEN RETURN false; END IF;
 FOR i IN 1..2 LOOP
   IF at_byte+2>=octet_length(p_der) OR get_byte(p_der,at_byte)<>2 THEN RETURN false; END IF;
   n:=get_byte(p_der,at_byte+1);
   at_byte:=at_byte+2;
   IF n NOT BETWEEN 1 AND 33 OR at_byte+n>octet_length(p_der) THEN RETURN false; END IF;
   IF get_byte(p_der,at_byte)>=128 THEN RETURN false; END IF;
   IF get_byte(p_der,at_byte)=0 THEN
      IF n=1 OR get_byte(p_der,at_byte+1)<128 THEN RETURN false; END IF;
      bits:=substring(p_der FROM at_byte+2 FOR n-1);
   ELSE bits:=substring(p_der FROM at_byte+1 FOR n); END IF;
   IF octet_length(bits)>32 OR (octet_length(bits)=32 AND bits>=curve_order) THEN RETURN false; END IF;
   at_byte:=at_byte+n;
 END LOOP;
 RETURN at_byte=octet_length(p_der);
END;
$$;
REVOKE ALL ON FUNCTION felix3_initialize_direct.canonical_action_ecdsa(bytea) FROM PUBLIC;

CREATE TABLE felix3_initialize_direct.action_installation (
 singleton boolean PRIMARY KEY DEFAULT true CHECK(singleton),
 installation uuid NOT NULL UNIQUE REFERENCES felix3_initialize_direct.installation(installation),
 enabled_at timestamptz NOT NULL DEFAULT clock_timestamp() CHECK(isfinite(enabled_at))
);
REVOKE ALL ON TABLE felix3_initialize_direct.action_installation FROM PUBLIC;

-- Sticky scope latch: policy or publication becoming unavailable must not
-- silently revert requests to another proof protocol.
CREATE FUNCTION felix3_initialize_direct.action_scope(p_tenant uuid,p_event uuid)
RETURNS TABLE(post_id uuid,publication_id uuid,installation_id uuid)
LANGUAGE sql STABLE SECURITY DEFINER STRICT SET search_path=pg_catalog,pg_temp AS $$
 SELECT i.post,i.publication,i.installation
 FROM felix3_initialize_direct.installation i JOIN felix3_initialize_direct.action_installation a USING(installation)
 WHERE (i.tenant,i.event)=(p_tenant,p_event);
$$;

CREATE FUNCTION felix3_initialize_direct.action_scope_enabled(
 p_install uuid,p_tenant uuid,p_event uuid,p_post uuid,p_pub uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER STRICT SET search_path=pg_catalog,pg_temp AS $$
 SELECT EXISTS (
   SELECT FROM felix3_initialize_direct.installation i
   JOIN felix3_initialize_direct.action_installation a USING(installation)
   JOIN sequent_backend.operational_signing_policy f ON f.singleton AND f.mode='legacy-p12'
   JOIN sequent_backend.ballot_publication p ON (p.tenant_id,p.election_event_id,p.id)=(i.tenant,i.event,i.publication)
   WHERE (i.installation,i.tenant,i.event,i.post,i.publication)=(p_install,p_tenant,p_event,p_post,p_pub)
     AND i.profile='softhsm-test' AND i.singleton AND a.singleton
     AND NOT EXISTS(SELECT FROM sequent_backend.operational_signing_required_tenant t WHERE t.tenant_id=p_tenant)
     -- The stock successor publication soft-deletes the previous snapshot and styles.
     AND p.is_generated AND isfinite(p.published_at) AND p.published_at<=statement_timestamp()
     AND EXISTS (SELECT FROM sequent_backend.ballot_style b
        WHERE (b.tenant_id,b.election_event_id,b.election_id,b.ballot_publication_id)=
              (i.tenant,i.event,i.post,i.publication))
 );
$$;

CREATE FUNCTION felix3_initialize_direct.guard_action_installation()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
DECLARE anchor felix3_initialize_direct.installation%ROWTYPE;
BEGIN
 IF TG_OP<>'INSERT' THEN RAISE EXCEPTION 'cannot remove or rewrite the direct action latch'; END IF;
 SELECT i.* INTO STRICT anchor FROM felix3_initialize_direct.installation i WHERE i.installation=NEW.installation;
 IF current_setting('transaction_isolation')<>'read committed' OR NOT
     pg_try_advisory_xact_lock(hashtextextended('operational-signing-policy:deployment',0)) OR NOT
     pg_try_advisory_xact_lock(hashtextextended('signing-event:'||anchor.tenant::text||':'||anchor.event::text,0))
 THEN RAISE EXCEPTION 'direct action policy or event is busy' USING ERRCODE='55P03'; END IF;
 IF NEW.enabled_at < statement_timestamp() OR NEW.enabled_at > clock_timestamp()
    OR NOT felix3_initialize_direct.scope_enabled(anchor.installation,anchor.tenant,anchor.event,anchor.post,anchor.publication)
    OR NOT felix3_initialize_direct.action_scope_enabled(anchor.installation,anchor.tenant,anchor.event,anchor.post,anchor.publication)
    OR EXISTS (
       SELECT FROM sequent_backend.signing_request r
        WHERE (r.tenant_id,r.election_event_id)=(anchor.tenant,anchor.event)
          AND (r.status='waiting' OR (r.status='completed' AND r.executed_at IS NULL))
          AND NOT (r.action='initialize-voting'
                   AND (EXISTS (SELECT FROM sequent_backend.signing_approval s
                               WHERE (s.tenant_id,s.election_event_id,s.request_id)=(r.tenant_id,r.election_event_id,r.id)
                                 AND s.initialize_direct_proof IS NOT NULL)
                        OR EXISTS (SELECT FROM felix3_initialize_direct.reservation prior_v1
                                   WHERE (prior_v1.tenant,prior_v1.event,prior_v1.request,prior_v1.installation)=
                                         (r.tenant_id,r.election_event_id,r.id,anchor.installation)
                                     AND prior_v1.expires_at>clock_timestamp()))
                   AND NOT EXISTS (SELECT FROM sequent_backend.signing_approval s
                               WHERE (s.tenant_id,s.election_event_id,s.request_id)=(r.tenant_id,r.election_event_id,r.id)
                                 AND s.initialize_direct_proof IS NULL))
    )
 THEN RAISE EXCEPTION 'direct action installation requires an intact demo anchor and no pending legacy requests'; END IF;
 RETURN NULL;
END;
$$;
REVOKE ALL ON FUNCTION felix3_initialize_direct.guard_action_installation() FROM PUBLIC;
CREATE TRIGGER direct_action_install AFTER INSERT ON felix3_initialize_direct.action_installation
 FOR EACH ROW EXECUTE FUNCTION felix3_initialize_direct.guard_action_installation();
CREATE TRIGGER direct_action_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON felix3_initialize_direct.action_installation
 FOR EACH STATEMENT EXECUTE FUNCTION felix3_initialize_direct.guard_action_installation();

CREATE TABLE felix3_initialize_direct.action_reservation (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
 installation uuid NOT NULL REFERENCES felix3_initialize_direct.action_installation(installation),
 tenant uuid NOT NULL,event uuid NOT NULL,post uuid NOT NULL,publication uuid NOT NULL,request uuid NOT NULL,
 actor uuid NOT NULL CHECK(actor<>'00000000-0000-0000-0000-000000000000'),
 action text COLLATE "C" NOT NULL CHECK(felix3_initialize_direct.known_action(action)),
 nonce uuid NOT NULL UNIQUE DEFAULT gen_random_uuid(),
 payload_sha256 bytea NOT NULL CHECK(octet_length(payload_sha256)=32),
 created_at timestamptz NOT NULL,expires_at timestamptz NOT NULL,used_at timestamptz,
 CHECK(isfinite(created_at) AND isfinite(expires_at) AND expires_at>created_at AND expires_at<=created_at+interval '45 seconds'),
 CHECK(used_at IS NULL OR used_at BETWEEN created_at AND expires_at),
 FOREIGN KEY(tenant,event,request) REFERENCES sequent_backend.signing_request(tenant_id,election_event_id,id)
);
CREATE INDEX direct_action_attempts ON felix3_initialize_direct.action_reservation(tenant,event,request,actor,expires_at);
REVOKE ALL ON TABLE felix3_initialize_direct.action_reservation FROM PUBLIC;

-- A request stays in its original proof protocol. Transaction caller checks
-- ordinary authorization, exact live action meaning, Post/country and quorum.
CREATE FUNCTION felix3_initialize_direct.waiting_action(
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

CREATE FUNCTION felix3_initialize_direct.reserve_action(
 p_install uuid,p_tenant uuid,p_event uuid,p_post uuid,p_pub uuid,p_request uuid,p_actor uuid,p_action text,p_sha bytea)
RETURNS TABLE(id uuid,nonce uuid,expires_at timestamptz)
LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path=pg_catalog,pg_temp AS $$
DECLARE end_at timestamptz; started timestamptz;
BEGIN
 IF current_setting('transaction_isolation')<>'read committed' OR octet_length(p_sha)<>32
   OR p_actor='00000000-0000-0000-0000-000000000000' THEN RETURN; END IF;
 PERFORM pg_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment',0));
 PERFORM pg_advisory_xact_lock(hashtextextended('signing-event:'||p_tenant::text||':'||p_event::text,0));
 end_at:=felix3_initialize_direct.waiting_action(p_install,p_tenant,p_event,p_post,p_pub,p_request,p_action,p_sha);
 started:=clock_timestamp();
 IF end_at IS NULL OR end_at<=started
    OR EXISTS(SELECT FROM sequent_backend.signing_approval a
       WHERE (a.tenant_id,a.election_event_id,a.request_id,a.user_id)=(p_tenant,p_event,p_request,p_actor::text))
    OR EXISTS(SELECT FROM felix3_initialize_direct.per_actor_key k
       WHERE (k.installation,k.actor)=(p_install,p_actor) AND k.revoked_at IS NOT NULL)
    OR EXISTS(SELECT FROM felix3_initialize_direct.action_reservation r
       WHERE (r.installation,r.tenant,r.event,r.request,r.actor)=(p_install,p_tenant,p_event,p_request,p_actor)
         AND r.expires_at>started)
 THEN RETURN; END IF;
 RETURN QUERY INSERT INTO felix3_initialize_direct.action_reservation
    (installation,tenant,event,post,publication,request,actor,action,payload_sha256,created_at,expires_at)
 VALUES(p_install,p_tenant,p_event,p_post,p_pub,p_request,p_actor,p_action,p_sha,started,least(end_at,started+interval '45 seconds'))
 RETURNING action_reservation.id,action_reservation.nonce,action_reservation.expires_at;
END;
$$;

CREATE FUNCTION felix3_initialize_direct.record_action_key(p_id uuid,p_spki bytea)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path=pg_catalog,pg_temp AS $$
DECLARE r felix3_initialize_direct.action_reservation%ROWTYPE;
BEGIN
 IF current_setting('transaction_isolation')<>'read committed' OR octet_length(p_spki)<>91
    OR substring(p_spki FROM 1 FOR 27)<>decode('3059301306072a8648ce3d020106082a8648ce3d03010703420004','hex')
    OR substring(p_spki FROM 28)=decode(repeat('00',64),'hex') THEN RETURN false; END IF;
 SELECT a.* INTO r FROM felix3_initialize_direct.action_reservation a WHERE a.id=p_id;
 IF NOT FOUND THEN RETURN false; END IF;
 PERFORM pg_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment',0));
 PERFORM pg_advisory_xact_lock(hashtextextended('signing-event:'||r.tenant::text||':'||r.event::text,0));
 SELECT a.* INTO r FROM felix3_initialize_direct.action_reservation a
 WHERE a.id=p_id AND a.used_at IS NULL AND a.expires_at>clock_timestamp() FOR UPDATE;
 IF NOT FOUND OR felix3_initialize_direct.waiting_action(r.installation,r.tenant,r.event,r.post,r.publication,r.request,r.action,r.payload_sha256) IS NULL
 THEN RETURN false; END IF;
 INSERT INTO felix3_initialize_direct.per_actor_key(installation,actor,spki_der)
 VALUES(r.installation,r.actor,p_spki) ON CONFLICT DO NOTHING;
 RETURN EXISTS (SELECT FROM felix3_initialize_direct.per_actor_key k
   WHERE (k.installation,k.actor,k.spki_der)=(r.installation,r.actor,p_spki) AND k.revoked_at IS NULL);
END;
$$;

CREATE FUNCTION felix3_initialize_direct.proof_action_key(
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

CREATE FUNCTION felix3_initialize_direct.consume_action(
 p_install uuid,p_tenant uuid,p_event uuid,p_post uuid,p_pub uuid,p_request uuid,p_actor uuid,p_nonce uuid,p_action text,p_sha bytea)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path=pg_catalog,pg_temp AS $$
DECLARE written timestamptz;
BEGIN
 IF current_setting('transaction_isolation')<>'read committed' THEN RETURN false; END IF;
 PERFORM pg_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment',0));
 PERFORM pg_advisory_xact_lock(hashtextextended('signing-event:'||p_tenant::text||':'||p_event::text,0));
 IF felix3_initialize_direct.waiting_action(p_install,p_tenant,p_event,p_post,p_pub,p_request,p_action,p_sha) IS NULL
   OR EXISTS (SELECT FROM sequent_backend.signing_approval a
          WHERE (a.tenant_id,a.election_event_id,a.request_id,a.user_id)=(p_tenant,p_event,p_request,p_actor::text))
 THEN RETURN false; END IF;
 written:=clock_timestamp();
 UPDATE felix3_initialize_direct.action_reservation r SET used_at=written
 WHERE (r.installation,r.tenant,r.event,r.post,r.publication,r.request,r.actor,r.nonce,r.action,r.payload_sha256)=
    (p_install,p_tenant,p_event,p_post,p_pub,p_request,p_actor,p_nonce,p_action,p_sha)
   AND r.used_at IS NULL AND r.expires_at>written AND EXISTS (
      SELECT FROM felix3_initialize_direct.per_actor_key k
      WHERE (k.installation,k.actor)=(p_install,p_actor) AND k.revoked_at IS NULL);
 RETURN FOUND;
END;
$$;

REVOKE ALL ON FUNCTION felix3_initialize_direct.waiting_action(uuid,uuid,uuid,uuid,uuid,uuid,text,bytea),
 felix3_initialize_direct.action_scope(uuid,uuid),felix3_initialize_direct.action_scope_enabled(uuid,uuid,uuid,uuid,uuid),
 felix3_initialize_direct.reserve_action(uuid,uuid,uuid,uuid,uuid,uuid,uuid,text,bytea),felix3_initialize_direct.record_action_key(uuid,bytea),
 felix3_initialize_direct.proof_action_key(uuid,uuid,uuid,uuid,uuid,uuid,uuid,uuid,text,bytea),
 felix3_initialize_direct.consume_action(uuid,uuid,uuid,uuid,uuid,uuid,uuid,uuid,text,bytea) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION felix3_initialize_direct.action_scope(uuid,uuid),
 felix3_initialize_direct.action_scope_enabled(uuid,uuid,uuid,uuid,uuid),
 felix3_initialize_direct.reserve_action(uuid,uuid,uuid,uuid,uuid,uuid,uuid,text,bytea),felix3_initialize_direct.record_action_key(uuid,bytea),
 felix3_initialize_direct.proof_action_key(uuid,uuid,uuid,uuid,uuid,uuid,uuid,uuid,text,bytea),
 felix3_initialize_direct.consume_action(uuid,uuid,uuid,uuid,uuid,uuid,uuid,uuid,text,bytea) TO sequent_os_approve;

-- Preserve the exact old two accepted representations. A V2 row cannot carry
-- legacy certificate, revocation, or document-signing proof.
ALTER TABLE sequent_backend.signing_approval
 DROP CONSTRAINT signing_approval_proof_format,
 ADD CONSTRAINT signing_approval_proof_format CHECK ((
    CASE WHEN action_direct_proof IS NULL THEN
        CASE WHEN initialize_direct_proof IS NULL THEN
            certificate_id IS NOT NULL AND certificate_pem IS NOT NULL AND chain_pem IS NOT NULL
            AND fingerprint_sha256 IS NOT NULL AND holder_sha256 IS NOT NULL AND revocation_status IS NOT NULL
        ELSE
            certificate_id IS NULL AND certificate_pem IS NULL AND chain_pem IS NULL
            AND fingerprint_sha256 IS NULL AND holder_sha256 IS NULL AND revocation_status IS NULL
            AND document_signature IS NULL AND pdf_cms IS NULL
            AND algorithm = 'ecdsa-p256-sha256' AND octet_length(payload_signature) BETWEEN 8 AND 72
            AND jsonb_typeof(initialize_direct_proof) = 'object'
            AND initialize_direct_proof - ARRAY['installation','post','publication','nonce','payload_sha256','profile'] = '{}'::jsonb
            AND initialize_direct_proof->>'profile' = 'softhsm-test'
            AND initialize_direct_proof->>'payload_sha256' ~ '^[0-9a-f]{64}$'
            AND initialize_direct_proof->>'installation' ~ '^[0-9a-f]{8}-([0-9a-f]{4}-){3}[0-9a-f]{12}$'
            AND initialize_direct_proof->>'post' ~ '^[0-9a-f]{8}-([0-9a-f]{4}-){3}[0-9a-f]{12}$'
            AND initialize_direct_proof->>'publication' ~ '^[0-9a-f]{8}-([0-9a-f]{4}-){3}[0-9a-f]{12}$'
            AND initialize_direct_proof->>'nonce' ~ '^[0-9a-f]{8}-([0-9a-f]{4}-){3}[0-9a-f]{12}$'
        END
    ELSE
        initialize_direct_proof IS NULL AND certificate_id IS NULL AND certificate_pem IS NULL
        AND chain_pem IS NULL AND fingerprint_sha256 IS NULL AND holder_sha256 IS NULL
        AND revocation_status IS NULL AND document_signature IS NULL AND pdf_cms IS NULL
        AND algorithm='ecdsa-p256-sha256' AND octet_length(payload_signature) BETWEEN 8 AND 72
        AND spki_sha256 ~ '^[0-9a-f]{64}$'
        AND jsonb_typeof(action_direct_proof)='object'
        AND action_direct_proof - ARRAY['installation','post','publication','nonce','payload_sha256','profile','action']='{}'::jsonb
        AND jsonb_typeof(action_direct_proof->'installation')='string'
        AND jsonb_typeof(action_direct_proof->'post')='string'
        AND jsonb_typeof(action_direct_proof->'publication')='string'
        AND jsonb_typeof(action_direct_proof->'nonce')='string'
        AND jsonb_typeof(action_direct_proof->'payload_sha256')='string'
        AND jsonb_typeof(action_direct_proof->'profile')='string'
        AND jsonb_typeof(action_direct_proof->'action')='string'
        AND action_direct_proof->>'profile'='softhsm-test'
        AND action_direct_proof->>'payload_sha256' ~ '^[0-9a-f]{64}$'
        AND action_direct_proof->>'installation' ~ '^[0-9a-f]{8}-([0-9a-f]{4}-){3}[0-9a-f]{12}$'
        AND action_direct_proof->>'post' ~ '^[0-9a-f]{8}-([0-9a-f]{4}-){3}[0-9a-f]{12}$'
        AND action_direct_proof->>'publication' ~ '^[0-9a-f]{8}-([0-9a-f]{4}-){3}[0-9a-f]{12}$'
        AND action_direct_proof->>'nonce' ~ '^[0-9a-f]{8}-([0-9a-f]{4}-){3}[0-9a-f]{12}$'
    END
 ) IS TRUE);
CREATE UNIQUE INDEX signing_approval_action_direct_nonce
 ON sequent_backend.signing_approval((action_direct_proof->>'nonce')) WHERE action_direct_proof IS NOT NULL;

-- The internal helper is not granted to the runtime role. This trigger
-- protects the shape and the used reservation under its trusted owner.
CREATE FUNCTION felix3_initialize_direct.action_evidence_guard()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
 IF TG_OP='UPDATE' AND OLD.action_direct_proof IS NOT NULL
    AND NEW.action_direct_proof IS DISTINCT FROM OLD.action_direct_proof
 THEN RAISE EXCEPTION 'direct action evidence cannot be reclassified' USING ERRCODE='23514'; END IF;
 IF NEW.action_direct_proof IS NULL THEN
   -- Serialize new legacy/V1 rows with marker installation. A retained row
   -- may be annotated; it cannot move to a different request or gain proof.
   IF (TG_OP='INSERT' OR
      ROW(NEW.tenant_id,NEW.election_event_id,NEW.request_id,NEW.user_id,NEW.initialize_direct_proof)
      IS DISTINCT FROM
      ROW(OLD.tenant_id,OLD.election_event_id,OLD.request_id,OLD.user_id,OLD.initialize_direct_proof))
      AND EXISTS (SELECT FROM felix3_initialize_direct.installation i
                  WHERE (i.tenant,i.event)=(NEW.tenant_id,NEW.election_event_id))
   THEN
      IF current_setting('transaction_isolation')<>'read committed'
      THEN RAISE EXCEPTION 'direct action policy requires read committed' USING ERRCODE='25001'; END IF;
      PERFORM pg_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment',0));
      IF EXISTS (
        SELECT FROM felix3_initialize_direct.action_installation a
        JOIN felix3_initialize_direct.installation i USING(installation)
        WHERE (i.tenant,i.event)=(NEW.tenant_id,NEW.election_event_id)
      ) AND NOT EXISTS (
        SELECT FROM felix3_initialize_direct.action_installation a
        JOIN felix3_initialize_direct.installation i USING(installation)
        JOIN sequent_backend.signing_request r ON (r.tenant_id,r.election_event_id,r.id)=
             (i.tenant,i.event,NEW.request_id)
        JOIN felix3_initialize_direct.reservation v ON
             (v.installation,v.request,v.tenant,v.event,v.post,v.publication)=
             (i.installation,r.id,i.tenant,i.event,i.post,i.publication)
        WHERE (i.tenant,i.event)=(NEW.tenant_id,NEW.election_event_id)
          AND r.action='initialize-voting' AND r.election_id=i.post
          AND r.area_id IS NULL AND r.trustee_id IS NULL
          AND r.subject->>'publication_id'=i.publication::text AND v.used_at IS NOT NULL
          AND v.actor::text=NEW.user_id AND v.nonce::text=NEW.initialize_direct_proof->>'nonce'
          AND v.installation::text=NEW.initialize_direct_proof->>'installation'
          AND v.post::text=NEW.initialize_direct_proof->>'post'
          AND v.publication::text=NEW.initialize_direct_proof->>'publication'
          AND r.payload_sha256=NEW.initialize_direct_proof->>'payload_sha256'
          AND encode(v.payload_sha256,'hex')=r.payload_sha256
      ) THEN RAISE EXCEPTION 'direct action evidence is required in the installed demo event' USING ERRCODE='23514'; END IF;
   END IF;
   RETURN NEW;
 END IF;
 IF NOT (felix3_initialize_direct.known_action(NEW.action_direct_proof->>'action')
        AND felix3_initialize_direct.canonical_action_ecdsa(NEW.payload_signature)
        AND EXISTS (
           SELECT FROM felix3_initialize_direct.action_reservation r
           JOIN felix3_initialize_direct.per_actor_key k ON (k.installation,k.actor)=(r.installation,r.actor)
           WHERE (r.tenant,r.event,r.request,r.actor::text)=(NEW.tenant_id,NEW.election_event_id,NEW.request_id,NEW.user_id)
             AND r.installation::text=NEW.action_direct_proof->>'installation' AND r.post::text=NEW.action_direct_proof->>'post'
             AND r.publication::text=NEW.action_direct_proof->>'publication' AND r.nonce::text=NEW.action_direct_proof->>'nonce'
             AND r.action=NEW.action_direct_proof->>'action' AND encode(r.payload_sha256,'hex')=NEW.action_direct_proof->>'payload_sha256'
             AND r.used_at IS NOT NULL AND k.revoked_at IS NULL AND encode(k.spki_sha256,'hex')=NEW.spki_sha256
        )) IS TRUE
 THEN RAISE EXCEPTION 'invalid or unconsumed direct action evidence' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END;
$$;
REVOKE ALL ON FUNCTION felix3_initialize_direct.action_evidence_guard() FROM PUBLIC;
CREATE TRIGGER direct_action_approval_evidence BEFORE INSERT OR UPDATE OF action_direct_proof,initialize_direct_proof,payload_signature,spki_sha256,
 user_id,request_id,tenant_id,election_event_id ON sequent_backend.signing_approval
 FOR EACH ROW EXECUTE FUNCTION felix3_initialize_direct.action_evidence_guard();
