-- SPDX-License-Identifier: AGPL-3.0-only
-- Optional COMELEC direct test-key authority. This migration creates no scope,
-- enrollment, vote, tally, signing approval, certificate, or tenant policy.
CREATE SCHEMA felix3_initialize_direct;
REVOKE ALL ON SCHEMA felix3_initialize_direct FROM PUBLIC;
GRANT USAGE ON SCHEMA felix3_initialize_direct TO sequent_os_approve;

CREATE TABLE felix3_initialize_direct.installation (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    installation uuid NOT NULL UNIQUE,
    tenant uuid NOT NULL,
    event uuid NOT NULL,
    post uuid NOT NULL,
    publication uuid NOT NULL,
    profile text COLLATE "C" NOT NULL CHECK (profile='softhsm-test'),
    CHECK ('00000000-0000-0000-0000-000000000000' NOT IN (installation,tenant,event,post,publication)),
    CHECK (current_database()<>'operational_demo_14990'),
    FOREIGN KEY(tenant,event,post) REFERENCES sequent_backend.election(tenant_id,election_event_id,id),
    FOREIGN KEY(tenant,event,publication) REFERENCES sequent_backend.ballot_publication(tenant_id,election_event_id,id)
);
CREATE TABLE felix3_initialize_direct.per_actor_key (
    installation uuid NOT NULL REFERENCES felix3_initialize_direct.installation(installation),
    actor uuid NOT NULL CHECK (actor<>'00000000-0000-0000-0000-000000000000'),
    spki_der bytea NOT NULL CHECK (octet_length(spki_der)=91),
    spki_sha256 bytea GENERATED ALWAYS AS (sha256(spki_der)) STORED,
    revoked_at timestamptz,
    PRIMARY KEY(installation,actor),
    UNIQUE(installation,spki_sha256)
);
CREATE TABLE felix3_initialize_direct.reservation (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    installation uuid NOT NULL REFERENCES felix3_initialize_direct.installation(installation),
    tenant uuid NOT NULL,
    event uuid NOT NULL,
    post uuid NOT NULL,
    publication uuid NOT NULL,
    request uuid NOT NULL,
    actor uuid NOT NULL CHECK (actor<>'00000000-0000-0000-0000-000000000000'),
    nonce uuid NOT NULL UNIQUE DEFAULT gen_random_uuid(),
    payload_sha256 bytea NOT NULL CHECK (octet_length(payload_sha256)=32),
    created_at timestamptz NOT NULL,
    expires_at timestamptz NOT NULL,
    claimed_at timestamptz,
    used_at timestamptz,
    CHECK (isfinite(created_at) AND isfinite(expires_at) AND expires_at>created_at
           AND expires_at<=created_at+interval '45 seconds'),
    CHECK (claimed_at IS NULL OR claimed_at BETWEEN created_at AND expires_at),
    CHECK (used_at IS NULL OR (claimed_at IS NOT NULL AND used_at BETWEEN claimed_at AND expires_at)),
    FOREIGN KEY(tenant,event,request) REFERENCES sequent_backend.signing_request(tenant_id,election_event_id,id)
);
CREATE INDEX direct_initialize_attempts
    ON felix3_initialize_direct.reservation(tenant,event,request,actor,created_at DESC);
REVOKE ALL ON ALL TABLES IN SCHEMA felix3_initialize_direct FROM PUBLIC;

-- Approval mode is a persistent scope latch. Never substitute the live
-- ballot lookup for this read: stale scope must not bring legacy back.
CREATE FUNCTION sequent_backend.operational_signing_local_initialize_scope(p_tenant uuid,p_event uuid)
RETURNS TABLE(post_id uuid,publication_id uuid,installation_id uuid)
LANGUAGE sql STABLE SECURITY DEFINER STRICT SET search_path=pg_catalog,pg_temp AS $$
 SELECT i.post,i.publication,i.installation FROM felix3_initialize_direct.installation i
 WHERE (i.tenant,i.event)=(p_tenant,p_event);
$$;
REVOKE ALL ON FUNCTION sequent_backend.operational_signing_local_initialize_scope(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION sequent_backend.operational_signing_local_initialize_scope(uuid,uuid) TO sequent_os_approve;

CREATE VIEW felix3_initialize_direct.current_installation WITH (security_barrier=true) AS
 SELECT i.*
 FROM felix3_initialize_direct.installation i
 JOIN sequent_backend.operational_signing_policy policy ON policy.singleton AND policy.mode='legacy-p12'
 JOIN sequent_backend.ballot_publication p ON
      (p.tenant_id,p.election_event_id,p.id)=(i.tenant,i.event,i.publication)
 WHERE i.singleton AND i.profile='softhsm-test' AND p.is_generated AND p.deleted_at IS NULL
   AND isfinite(p.published_at) AND p.published_at<=statement_timestamp()
   AND EXISTS (SELECT FROM sequent_backend.ballot_style b
     WHERE (b.tenant_id,b.election_event_id,b.election_id,b.ballot_publication_id)
           =(i.tenant,i.event,i.post,i.publication) AND b.deleted_at IS NULL)
   AND p.id=(SELECT x.id FROM sequent_backend.ballot_publication x
     WHERE (x.tenant_id,x.election_event_id)=(i.tenant,i.event)
       AND (x.election_id=i.post OR i.post=ANY(x.election_ids)) AND x.deleted_at IS NULL
       AND x.published_at IS NOT NULL ORDER BY x.published_at DESC,x.id LIMIT 1);
REVOKE ALL ON TABLE felix3_initialize_direct.current_installation FROM PUBLIC;

CREATE FUNCTION felix3_initialize_direct.scope_enabled(
    p_install uuid,p_tenant uuid,p_event uuid,p_post uuid,p_publication uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER STRICT SET search_path=pg_catalog,pg_temp AS $$
 SELECT EXISTS (SELECT FROM felix3_initialize_direct.current_installation i
        WHERE (i.installation,i.tenant,i.event,i.post,i.publication)=
              (p_install,p_tenant,p_event,p_post,p_publication));
$$;
REVOKE ALL ON FUNCTION felix3_initialize_direct.scope_enabled(uuid,uuid,uuid,uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION felix3_initialize_direct.scope_enabled(uuid,uuid,uuid,uuid,uuid) TO sequent_os_approve;

-- Independent of runtime clients and immutable except a separately managed
-- internal DBA revocation; no Portal-to-database write or key-choice API.
CREATE FUNCTION felix3_initialize_direct.is_waiting(
    p_install uuid,p_tenant uuid,p_event uuid,p_post uuid,p_publication uuid,p_request uuid,p_sha bytea)
RETURNS timestamptz LANGUAGE sql VOLATILE STRICT SET search_path=pg_catalog,pg_temp AS $$
 SELECT r.expires_at FROM sequent_backend.signing_request r
 WHERE (r.tenant_id,r.election_event_id,r.election_id,r.id,r.action,r.status)=
       (p_tenant,p_event,p_post,p_request,'initialize-voting','waiting')
   AND r.completed_at IS NULL AND r.executed_at IS NULL AND r.area_id IS NULL
   AND r.document_id IS NULL AND r.document_sha256 IS NULL
   AND r.subject=jsonb_build_object('publication_id',p_publication)
   AND isfinite(r.expires_at) AND r.expires_at>clock_timestamp()
   AND r.payload_sha256=encode(p_sha,'hex') AND sha256(convert_to(r.canonical_payload,'UTF8'))=p_sha
   AND felix3_initialize_direct.scope_enabled(p_install,p_tenant,p_event,p_post,p_publication);
$$;
REVOKE ALL ON FUNCTION felix3_initialize_direct.is_waiting(uuid,uuid,uuid,uuid,uuid,uuid,bytea) FROM PUBLIC;

-- Operator installs only one already-published whole Post, before any old
-- approval/effect for it. This cannot be changed back by flipping a flag.
CREATE FUNCTION felix3_initialize_direct.guard_installation()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
 IF TG_OP<>'INSERT' THEN RAISE EXCEPTION 'cannot remove or rewrite the direct signing latch'; END IF;
 IF current_setting('transaction_isolation')<>'read committed' OR NOT
      pg_try_advisory_xact_lock(hashtextextended('operational-signing-policy:deployment',0)) OR NOT
      pg_try_advisory_xact_lock(hashtextextended('signing-event:'||NEW.tenant::text||':'||NEW.event::text,0)) THEN
    RAISE EXCEPTION 'direct signing policy or event is busy' USING ERRCODE='55P03';
 END IF;
 IF NOT felix3_initialize_direct.scope_enabled(NEW.installation,NEW.tenant,NEW.event,NEW.post,NEW.publication)
   OR EXISTS (SELECT FROM sequent_backend.tally_session s WHERE
         (s.tenant_id,s.election_event_id)=(NEW.tenant,NEW.event) AND s.tally_type::text='INITIALIZATION_REPORT'
         AND (coalesce(cardinality(s.election_ids),0)=0 OR NEW.post=ANY(s.election_ids)))
   OR EXISTS (SELECT FROM sequent_backend.signing_request r WHERE
         (r.tenant_id,r.election_event_id,r.election_id,r.action)=(NEW.tenant,NEW.event,NEW.post,'initialize-voting')
         AND (r.completed_at IS NOT NULL OR r.executed_at IS NOT NULL OR r.status NOT IN ('waiting','expired','cancelled')
              OR EXISTS (SELECT FROM sequent_backend.signing_approval a
                 WHERE (a.tenant_id,a.election_event_id,a.request_id)=(r.tenant_id,r.election_event_id,r.id))))
 THEN RAISE EXCEPTION 'only a clean published synthetic Post may be installed'; END IF;
 RETURN NULL;
END;
$$;
REVOKE ALL ON FUNCTION felix3_initialize_direct.guard_installation() FROM PUBLIC;
CREATE TRIGGER direct_initialize_install AFTER INSERT ON felix3_initialize_direct.installation
 FOR EACH ROW EXECUTE FUNCTION felix3_initialize_direct.guard_installation();
CREATE TRIGGER direct_initialize_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON felix3_initialize_direct.installation
 FOR EACH STATEMENT EXECUTE FUNCTION felix3_initialize_direct.guard_installation();

CREATE FUNCTION felix3_initialize_direct.reserve(
    p_install uuid,p_tenant uuid,p_event uuid,p_post uuid,p_publication uuid,p_request uuid,p_actor uuid,p_sha bytea
) RETURNS TABLE(id uuid,nonce uuid,expires_at timestamptz) LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path=pg_catalog,pg_temp AS $$
DECLARE end_at timestamptz; started timestamptz; prior felix3_initialize_direct.reservation%ROWTYPE;
BEGIN
 IF current_setting('transaction_isolation')<>'read committed' OR octet_length(p_sha)<>32 THEN RETURN; END IF;
 PERFORM pg_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment',0));
 PERFORM pg_advisory_xact_lock(hashtextextended('signing-event:'||p_tenant::text||':'||p_event::text,0));
 end_at:=felix3_initialize_direct.is_waiting(p_install,p_tenant,p_event,p_post,p_publication,p_request,p_sha);
 started:=clock_timestamp();
 IF end_at IS NULL OR end_at<=started OR EXISTS (SELECT FROM sequent_backend.signing_approval a
    WHERE (a.tenant_id,a.election_event_id,a.request_id,a.user_id)=(p_tenant,p_event,p_request,p_actor::text))
 OR EXISTS (SELECT FROM felix3_initialize_direct.per_actor_key k WHERE (k.installation,k.actor)=(p_install,p_actor)
            AND k.revoked_at IS NOT NULL) THEN RETURN; END IF;
 SELECT r.* INTO prior FROM felix3_initialize_direct.reservation r
 WHERE (r.installation,r.tenant,r.event,r.post,r.publication,r.request,r.actor,r.payload_sha256)=
       (p_install,p_tenant,p_event,p_post,p_publication,p_request,p_actor,p_sha)
   AND r.expires_at>started ORDER BY r.created_at DESC LIMIT 1;
 -- Only Harvest prepares the attempt. A lost HTTP reply never
 -- redelivers a live attempt; after the original deadline a new explicit
 -- approval can reserve with the same stable key, and never count twice.
 IF FOUND THEN RETURN; END IF;
 RETURN QUERY INSERT INTO felix3_initialize_direct.reservation
   (installation,tenant,event,post,publication,request,actor,payload_sha256,created_at,expires_at,claimed_at)
   VALUES(p_install,p_tenant,p_event,p_post,p_publication,p_request,p_actor,p_sha,started,
          least(end_at,started+interval '45 seconds'),started)
   RETURNING reservation.id,reservation.nonce,reservation.expires_at;
END;
$$;
REVOKE ALL ON FUNCTION felix3_initialize_direct.reserve(uuid,uuid,uuid,uuid,uuid,uuid,uuid,bytea) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION felix3_initialize_direct.reserve(uuid,uuid,uuid,uuid,uuid,uuid,uuid,bytea) TO sequent_os_approve;

-- Device has no database connection, credential, installer role, or
-- EXECUTE API. Only Harvest can obtain or consume the private reservation.
CREATE FUNCTION sequent_backend.record_local_initialize_direct_key(p_id uuid,p_key bytea)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path=pg_catalog,pg_temp AS $$
DECLARE a felix3_initialize_direct.reservation%ROWTYPE;
BEGIN
 IF current_setting('transaction_isolation')<>'read committed' OR octet_length(p_key)<>91 THEN RETURN false; END IF;
 SELECT r.* INTO a FROM felix3_initialize_direct.reservation r WHERE r.id=p_id;
 IF NOT FOUND THEN RETURN false; END IF;
 PERFORM pg_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment',0));
 PERFORM pg_advisory_xact_lock(hashtextextended('signing-event:'||a.tenant::text||':'||a.event::text,0));
 SELECT r.* INTO a FROM felix3_initialize_direct.reservation r
 WHERE r.id=p_id AND r.claimed_at IS NOT NULL AND r.used_at IS NULL AND r.expires_at>clock_timestamp();
 IF NOT FOUND OR felix3_initialize_direct.is_waiting(a.installation,a.tenant,a.event,a.post,a.publication,a.request,a.payload_sha256) IS NULL
 THEN RETURN false; END IF;
 INSERT INTO felix3_initialize_direct.per_actor_key(installation,actor,spki_der) VALUES (a.installation,a.actor,p_key)
 ON CONFLICT DO NOTHING;
 RETURN EXISTS (SELECT FROM felix3_initialize_direct.per_actor_key k
   WHERE (k.installation,k.actor,k.spki_der)=(a.installation,a.actor,p_key) AND k.revoked_at IS NULL);
END;
$$;
REVOKE ALL ON FUNCTION sequent_backend.record_local_initialize_direct_key(uuid,bytea) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION sequent_backend.record_local_initialize_direct_key(uuid,bytea) TO sequent_os_approve;

CREATE FUNCTION felix3_initialize_direct.proof_key(
    p_install uuid,p_tenant uuid,p_event uuid,p_post uuid,p_publication uuid,p_request uuid,p_actor uuid,
    p_nonce uuid,p_sha bytea)
RETURNS bytea LANGUAGE sql VOLATILE SECURITY DEFINER STRICT SET search_path=pg_catalog,pg_temp AS $$
 SELECT k.spki_der FROM felix3_initialize_direct.reservation r JOIN felix3_initialize_direct.per_actor_key k
  ON (k.installation,k.actor)=(r.installation,r.actor)
 WHERE (r.installation,r.tenant,r.event,r.post,r.publication,r.request,r.actor,r.nonce,r.payload_sha256)=
    (p_install,p_tenant,p_event,p_post,p_publication,p_request,p_actor,p_nonce,p_sha)
   AND r.claimed_at IS NOT NULL AND (r.used_at IS NOT NULL OR r.expires_at>clock_timestamp())
   AND k.revoked_at IS NULL AND felix3_initialize_direct.scope_enabled(p_install,p_tenant,p_event,p_post,p_publication);
$$;
REVOKE ALL ON FUNCTION felix3_initialize_direct.proof_key(uuid,uuid,uuid,uuid,uuid,uuid,uuid,uuid,bytea) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION felix3_initialize_direct.proof_key(uuid,uuid,uuid,uuid,uuid,uuid,uuid,uuid,bytea) TO sequent_os_approve;

CREATE FUNCTION felix3_initialize_direct.consume(
    p_install uuid,p_tenant uuid,p_event uuid,p_post uuid,p_publication uuid,p_request uuid,p_actor uuid,
    p_nonce uuid,p_sha bytea)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path=pg_catalog,pg_temp AS $$
DECLARE written timestamptz;
BEGIN
 IF current_setting('transaction_isolation')<>'read committed' THEN RETURN false; END IF;
 PERFORM pg_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment',0));
 PERFORM pg_advisory_xact_lock(hashtextextended('signing-event:'||p_tenant::text||':'||p_event::text,0));
 IF felix3_initialize_direct.is_waiting(p_install,p_tenant,p_event,p_post,p_publication,p_request,p_sha) IS NULL
    THEN RETURN false; END IF;
 written:=clock_timestamp();
 UPDATE felix3_initialize_direct.reservation r SET used_at=written
   WHERE (r.installation,r.tenant,r.event,r.post,r.publication,r.request,r.actor,r.nonce,r.payload_sha256)=
         (p_install,p_tenant,p_event,p_post,p_publication,p_request,p_actor,p_nonce,p_sha)
     AND r.claimed_at IS NOT NULL AND r.used_at IS NULL AND r.expires_at>written AND EXISTS (
       SELECT FROM felix3_initialize_direct.per_actor_key k WHERE
             (k.installation,k.actor)=(p_install,p_actor) AND k.revoked_at IS NULL);
 RETURN FOUND;
END;
$$;
REVOKE ALL ON FUNCTION felix3_initialize_direct.consume(uuid,uuid,uuid,uuid,uuid,uuid,uuid,uuid,bytea) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION felix3_initialize_direct.consume(uuid,uuid,uuid,uuid,uuid,uuid,uuid,uuid,bytea) TO sequent_os_approve;
