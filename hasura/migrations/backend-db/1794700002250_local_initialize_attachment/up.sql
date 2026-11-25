-- SPDX-License-Identifier: AGPL-3.0-only
-- Transactional after 2200. Installation creates no service grant, marker or trust.
CREATE SCHEMA felix3_initialize_attachment AUTHORIZATION sequent_os_ledger_api_owner;
REVOKE ALL ON SCHEMA felix3_initialize_attachment FROM PUBLIC;
GRANT USAGE, CREATE ON SCHEMA felix3_initialize_attachment TO sequent_os_ledger_data_owner;

-- Only the independently authenticated Harvest officer adapter can admit an
-- existing registry transaction. A PID alone can be reused, an xid8 cannot.
CREATE TABLE felix3_initialize_attachment.decision (
    attempt uuid PRIMARY KEY REFERENCES felix3_initialize_setup.attempt(id),
    bound felix3_initialize_setup.binding NOT NULL CHECK (bound IS NOT NULL),
    -- These projections are written once. Even a future erroneous UPDATE of
    -- bound or pins cannot switch the installation, either actor, key or CA.
    scope jsonb NOT NULL, fixed_pins jsonb NOT NULL,
    officer uuid NOT NULL, pins jsonb NOT NULL, trust text NOT NULL,
    officer_labels text[] NOT NULL, nominee_labels text[] NOT NULL,
    writer name NOT NULL, backend integer NOT NULL, xid xid8 NOT NULL,
    until_at timestamptz NOT NULL CHECK (isfinite(until_at)),
    certificate uuid, attached_at timestamptz,
    CHECK ((certificate IS NULL) = (attached_at IS NULL)),
    CHECK (scope = (to_jsonb(bound) - ARRAY['request','payload_sha256'])),
    CHECK (fixed_pins = (pins - ARRAY['leaf_sha256','crl_sha256']))
);
ALTER TABLE felix3_initialize_attachment.decision OWNER TO sequent_os_ledger_data_owner;
REVOKE CREATE ON SCHEMA felix3_initialize_attachment FROM sequent_os_ledger_data_owner;
REVOKE ALL ON felix3_initialize_attachment.decision FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE (bound,pins,trust,officer_labels,nominee_labels,backend,xid,until_at,certificate,attached_at)
    ON felix3_initialize_attachment.decision TO sequent_os_ledger_api_owner;

-- Exclude trust changes, phantom CRLs, registrations and approvals through the
-- final commit, including writers that don't take the application event lock.
-- The migration administrator supplies only this fixed NOWAIT lock operation.
CREATE FUNCTION felix3_initialize_attachment.lock_registry() RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, pg_temp AS $$
BEGIN
    LOCK TABLE sequent_backend.certificate_authority, sequent_backend.staff_crl,
        sequent_backend.signing_approval IN SHARE MODE NOWAIT;
    LOCK TABLE sequent_backend.staff_certificate IN SHARE ROW EXCLUSIVE MODE NOWAIT;
END;
$$;
REVOKE ALL ON FUNCTION felix3_initialize_attachment.lock_registry() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION felix3_initialize_attachment.lock_registry() TO sequent_os_ledger_api_owner;
-- A row lock on election requires UPDATE privilege. Keep even that privilege
-- off the API owner and return only the result for the exact labelled Post.
CREATE FUNCTION felix3_initialize_attachment.post_access(
    b felix3_initialize_setup.binding, ol text[], nl text[]
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path = pg_catalog, pg_temp AS $$
BEGIN
    PERFORM FROM sequent_backend.signing_request r JOIN sequent_backend.election p
        ON (p.tenant_id,p.election_event_id,p.id) = (r.tenant_id,r.election_event_id,r.election_id)
      WHERE (r.tenant_id,r.election_event_id,r.election_id,r.id) = (b.tenant,b.event,b.post,b.request)
        AND (r.permission_label IS NULL OR (r.permission_label = ANY(ol) AND r.permission_label = ANY(nl)))
        AND (p.permission_label IS NULL OR (p.permission_label = ANY(ol) AND p.permission_label = ANY(nl)))
      FOR SHARE OF p NOWAIT;
    RETURN FOUND;
END;
$$;
REVOKE ALL ON FUNCTION felix3_initialize_attachment.post_access(felix3_initialize_setup.binding,text[],text[]) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION felix3_initialize_attachment.post_access(felix3_initialize_setup.binding,text[],text[])
    TO sequent_os_ledger_api_owner;
SET LOCAL ROLE sequent_os_ledger_api_owner;

CREATE FUNCTION felix3_initialize_attachment.officer_service() RETURNS boolean
LANGUAGE sql STABLE SET search_path = pg_catalog, pg_temp AS $$
    SELECT felix3_initialize_setup.principal('felix3_local_attachment_officer')
        AND system_user = 'peer:felix3_local_attachment_officer'
        AND NOT has_schema_privilege(session_user,'felix3_initialize_attachment','CREATE')
        AND NOT has_table_privilege(session_user,'felix3_initialize_attachment.decision',
                                    'SELECT,INSERT,UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER')
        AND NOT has_any_column_privilege(session_user,'felix3_initialize_attachment.decision',
                                         'SELECT,INSERT,UPDATE,REFERENCES')
        AND NOT EXISTS (SELECT FROM pg_class c WHERE c.relnamespace = 'sequent_backend'::regnamespace
            AND c.relname IN ('tenant','election_event','election','ballot_publication')
            AND (has_table_privilege(session_user,c.oid,'INSERT,UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER')
                 OR has_any_column_privilege(session_user,c.oid,'INSERT,UPDATE,REFERENCES')))
        AND NOT EXISTS (SELECT FROM pg_proc p WHERE p.pronamespace IN
                ('felix3_initialize_setup'::regnamespace,'felix3_initialize_attachment'::regnamespace)
            AND has_function_privilege(session_user,p.oid,'EXECUTE') AND NOT
                (p.pronamespace = 'felix3_initialize_attachment'::regnamespace
                 AND p.proname IN ('review_csr','inspect','admit')));
$$;

-- The preinstalled registry writer can use its ordinary registration and
-- audit rights, but must have no way to change its own admission or trust.
CREATE FUNCTION felix3_initialize_attachment.registry_writer(login name) RETURNS boolean
LANGUAGE sql STABLE STRICT SET search_path = pg_catalog, pg_temp AS $$
    SELECT EXISTS (SELECT FROM pg_roles r WHERE r.rolname = login AND r.rolcanlogin
        AND NOT (r.rolsuper OR r.rolcreatedb OR r.rolcreaterole OR r.rolreplication OR r.rolbypassrls)
        AND NOT EXISTS (SELECT FROM pg_auth_members m WHERE m.member = r.oid)
        AND NOT has_database_privilege(r.oid,current_database(),'CREATE')
        AND NOT EXISTS (SELECT FROM pg_namespace n WHERE n.nspname IN
            ('sequent_backend','felix3_initialize_setup','felix3_initialize_attachment')
            AND has_schema_privilege(r.oid,n.oid,'CREATE'))
        AND NOT EXISTS (SELECT FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE c.relkind IN ('r','p','v','m','f') AND (n.nspname IN ('felix3_initialize_setup','felix3_initialize_attachment')
                OR (n.nspname = 'sequent_backend' AND c.relname ~ '^operational_'))
              AND (has_table_privilege(r.oid,c.oid,'SELECT') OR has_any_column_privilege(r.oid,c.oid,'SELECT')))
        AND NOT EXISTS (SELECT FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE c.relkind IN ('r','p','v','m','f') AND (n.nspname IN ('felix3_initialize_setup','felix3_initialize_attachment')
                OR (n.nspname = 'sequent_backend' AND (c.relname ~ '^operational_' OR c.relname IN
                    ('tenant','election_event','election','ballot_publication','signing_rule','signing_checks',
                     'signing_request','signing_approval','certificate_authority','staff_crl'))))
              AND (has_table_privilege(r.oid,c.oid,'INSERT,UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER')
                   OR has_any_column_privilege(r.oid,c.oid,'INSERT,UPDATE,REFERENCES')))
        AND NOT EXISTS (SELECT FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace
            WHERE has_schema_privilege(r.oid,n.oid,'USAGE') AND has_function_privilege(r.oid,p.oid,'EXECUTE')
              AND ((n.nspname='sequent_backend' AND p.proname ~ '^operational_' AND p.prosecdef AND p.provolatile='v')
                OR n.nspname='felix3_initialize_setup' OR (n.nspname='felix3_initialize_attachment' AND p.proname <> 'finish'))));
$$;

-- Internal only. Bind the current 2200 permit to the user's original Gold
-- release, device claim and completed 1200 enrollment. Reuse 2200's locks,
-- five-argument marker, signed request bytes, publication and hard expiry.
CREATE FUNCTION felix3_initialize_attachment.generation(
    b felix3_initialize_setup.binding, ticket uuid, who uuid, officer_labels text[], nominee_labels text[]
) RETURNS TABLE (enrollment uuid, spki text, csr text, certificate uuid, fingerprint text,
                 attached_at timestamptz, deadline timestamptz)
LANGUAGE plpgsql STRICT SET search_path = pg_catalog, pg_temp AS $$
DECLARE
    permit record;
    g sequent_backend.operational_setup_generation%ROWTYPE;
    pin record;
    original text;
BEGIN
    IF NOT EXISTS (SELECT FROM felix3_initialize_setup.attempt a
        WHERE (a.id,a.bound,a.officer) = (ticket,b,who) AND a.abandoned_at IS NULL
          AND a.released_at IS NOT NULL AND a.claimed_until IS NOT NULL) THEN RETURN; END IF;
    SELECT * INTO permit FROM felix3_initialize_setup.eligible(b,true);
    IF NOT FOUND OR permit.officer <> who OR who = b.official THEN RETURN; END IF;
    -- Both verified sessions must see the request's frozen AND live Post labels.
    IF NOT felix3_initialize_attachment.post_access(b,officer_labels,nominee_labels) THEN RETURN; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'operational-signing-official:' || b.tenant::text || ':' || b.official::text,0));
    SELECT s.* INTO g FROM sequent_backend.operational_setup_generation s
      JOIN felix3_initialize_setup.attempt a ON a.id = s.attempt
      JOIN sequent_backend.operational_signing_official_enrollment e
        ON (e.tenant_id,e.actor,e.id,e.spki,e.device_profile)
         = (s.tenant_id,s.official::text,s.enrollment_id,s.spki_sha256,s.device_profile)
      JOIN sequent_backend.operational_setup_installation i
        ON (i.tenant_id,i.device_profile) = (s.tenant_id,s.device_profile)
      WHERE (s.attempt,s.tenant_id,s.event_id,s.official,s.prepared_by,s.device_profile)
          = (ticket,b.tenant,b.event,b.official,who,'softhsm-test')
        AND (a.bound,a.officer) = (b,who) AND a.abandoned_at IS NULL
        AND a.released_at IS NOT NULL AND a.claimed_until IS NOT NULL
        AND s.step IN ('completed','attached') AND s.fence IS NOT NULL AND s.evidence_sha256 IS NOT NULL
        AND e.nonexportable_confirmed AND e.revoked_at IS NULL
        AND sequent_backend.operational_signing_hsm_required(b.tenant)
        AND NOT EXISTS (SELECT FROM sequent_backend.signing_approval v
             WHERE (v.tenant_id,v.election_event_id,v.request_id,v.user_id)
                 = (b.tenant,b.event,b.request,b.official::text))
      FOR UPDATE OF s FOR SHARE OF a NOWAIT;
    IF NOT FOUND OR permit.until_at <= clock_timestamp() THEN RETURN; END IF;
    IF g.step = 'completed' THEN
        original := sequent_backend.operational_setup_public_csr(b.tenant,b.event,b.official,who,ticket);
        SELECT * INTO pin FROM sequent_backend.operational_setup_pin_for_attachment(b.tenant,b.event,b.official,who,ticket);
        IF NOT FOUND OR (original,pin.enrollment_id,pin.expected_spki)
            IS DISTINCT FROM (g.public_csr_pem,g.enrollment_id,g.spki_sha256)
            OR sequent_backend.operational_official_setup_status(b.tenant,b.event,b.official::text)
                IS DISTINCT FROM 'certificate-not-attached' THEN RETURN; END IF;
    ELSE
        IF g.attached_by IS DISTINCT FROM who THEN RETURN; END IF;
        original := g.public_csr_pem;
    END IF;
    RETURN QUERY SELECT g.enrollment_id,g.spki_sha256,original,g.certificate_id,g.certificate_fingerprint,
                        g.attached_at,permit.until_at;
END;
$$;

CREATE FUNCTION felix3_initialize_attachment.review_csr(
    b felix3_initialize_setup.binding, ticket uuid, who uuid, ol text[], nl text[], auth_time timestamptz
) RETURNS TABLE (enrollment uuid, spki text, csr text)
LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path = pg_catalog, pg_temp AS $$
BEGIN
    IF felix3_initialize_attachment.officer_service() IS NOT TRUE THEN RETURN; END IF;
    RETURN QUERY SELECT g.enrollment,g.spki,g.csr FROM felix3_initialize_attachment.generation(b,ticket,who,ol,nl) g
      WHERE g.certificate IS NULL AND auth_time <= clock_timestamp()
        AND clock_timestamp() < least(auth_time + interval '60 seconds',g.deadline);
END;
$$;

-- The registry may normalize PEM line endings. Bind to the DER of exactly
-- one certificate, and fail closed on a malformed envelope or trailing data.
CREATE FUNCTION felix3_initialize_attachment.certificate_hash(pem text) RETURNS text
LANGUAGE sql IMMUTABLE STRICT SET search_path = pg_catalog, pg_temp AS $$
    SELECT CASE WHEN octet_length(pem) BETWEEN 1 AND 65536 THEN encode(sha256(decode(
        substring(pem FROM E'^-----BEGIN CERTIFICATE-----\\r?\\n([A-Za-z0-9+/=\\r\\n]+)-----END CERTIFICATE-----\\r?\\n?$'),
        'base64')),'hex') END;
$$;

-- Read only public trust. One installed directly issuing staff root and one
-- positive full CRL for it. The external leaf is checked by Windmill before
-- admission. A change of PEM/CRL/registry metadata invalidates that decision.
CREATE FUNCTION felix3_initialize_attachment.trust(b felix3_initialize_setup.binding, pins jsonb)
RETURNS TABLE (root_pem text, crl_der bytea, url text, fetched_at timestamptz, deadline timestamptz, stamp text)
LANGUAGE sql VOLATILE STRICT SET search_path = pg_catalog, pg_temp AS $$
    SELECT ca.pem::text,l.der,l.url::text,l.last_ok_at,
        least(ca.not_after,l.next_update,l.last_ok_at + interval '24 hours'),
        encode(sha256(convert_to(jsonb_build_array(ca.id,ca.pem,ca.fingerprint_sha256,
            extract(epoch FROM ca.not_before),extract(epoch FROM ca.not_after),l.url,encode(sha256(l.der),'hex'),
            extract(epoch FROM l.last_ok_at),extract(epoch FROM l.this_update),extract(epoch FROM l.next_update))::text,'UTF8')),'hex')
      FROM sequent_backend.certificate_authority ca JOIN sequent_backend.staff_crl l
        ON (l.tenant_id,l.election_event_id,l.issuer_id,l.issuer_fingerprint)
         = (ca.tenant_id,ca.election_event_id,ca.id,ca.fingerprint_sha256)
      WHERE (ca.tenant_id,ca.election_event_id,ca.id::text,ca.fingerprint_sha256)
          = ((b).tenant,(b).event,pins->>'issuer',pins->>'root_sha256')
        AND felix3_initialize_attachment.certificate_hash(ca.pem) = ca.fingerprint_sha256
        AND ca.purpose = 'staff-signatures' AND ca.not_before <= clock_timestamp() AND ca.not_after > clock_timestamp()
        AND l.status = 'ok' AND l.this_update <= clock_timestamp() AND l.next_update > clock_timestamp()
        AND l.last_ok_at <= clock_timestamp() AND l.last_ok_at + interval '24 hours' > clock_timestamp()
        AND octet_length(ca.pem) BETWEEN 1 AND 65536 AND octet_length(l.der) BETWEEN 1 AND 4194304
        AND encode(sha256(l.der),'hex') = pins->>'crl_sha256'
        AND NOT EXISTS (SELECT FROM sequent_backend.staff_crl other
            WHERE (other.tenant_id,other.election_event_id) = ((b).tenant,(b).event)
              AND (other.issuer_id = ca.id OR other.issuer_fingerprint = ca.fingerprint_sha256)
              AND (other.tableoid,other.ctid) <> (l.tableoid,l.ctid));
$$;

CREATE FUNCTION felix3_initialize_attachment.inspect(
    b felix3_initialize_setup.binding, ticket uuid, who uuid, ol text[], nl text[], auth_time timestamptz, expected jsonb
) RETURNS TABLE (csr text, root_pem text, crl_der bytea, url text, fetched_at timestamptz,
                 deadline timestamptz, stamp text, certificate uuid, attached_at timestamptz)
LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path = pg_catalog, pg_temp AS $$
DECLARE
    g record;
BEGIN
    IF felix3_initialize_attachment.officer_service() IS NOT TRUE THEN RETURN; END IF;
    IF jsonb_typeof(expected) <> 'object' OR octet_length(expected::text) > 2048
       OR (expected - ARRAY['enrollment','spki_sha256','csr_sha256','leaf_sha256','issuer','root_sha256','crl_sha256'])
           <> '{}'::jsonb
       OR (expected->>'leaf_sha256' ~ '^[0-9a-f]{64}$') IS NOT TRUE THEN RETURN; END IF;
    SELECT * INTO g FROM felix3_initialize_attachment.generation(b,ticket,who,ol,nl);
    IF NOT FOUND OR (expected->>'enrollment',expected->>'spki_sha256',expected->>'csr_sha256')
        IS DISTINCT FROM (g.enrollment::text,g.spki,encode(sha256(convert_to(g.csr,'UTF8')),'hex'))
        OR auth_time > clock_timestamp() THEN RETURN; END IF;
    PERFORM felix3_initialize_attachment.lock_registry();
    RETURN QUERY SELECT g.csr,t.root_pem,t.crl_der,t.url,t.fetched_at,
        least(t.deadline,g.deadline,auth_time + interval '60 seconds'),t.stamp,g.certificate,g.attached_at
      FROM felix3_initialize_attachment.trust(b,expected) t
      WHERE least(g.deadline,auth_time + interval '60 seconds') > clock_timestamp()
        AND NOT EXISTS (SELECT FROM sequent_backend.signing_approval v
            WHERE (v.tenant_id,v.election_event_id,v.request_id,v.user_id)
              = (b.tenant,b.event,b.request,b.official::text))
        AND ((g.certificate IS NULL AND NOT EXISTS (SELECT FROM felix3_initialize_attachment.decision v
              WHERE v.attempt = ticket AND v.certificate IS NOT NULL)
            AND NOT EXISTS (SELECT FROM sequent_backend.staff_certificate c
              WHERE (c.tenant_id,c.election_event_id) = (b.tenant,b.event)
                AND (c.spki_sha256 = g.spki OR c.fingerprint_sha256 = expected->>'leaf_sha256')))
          OR EXISTS (SELECT FROM felix3_initialize_attachment.decision d
              JOIN sequent_backend.staff_certificate c ON c.id = d.certificate
              JOIN sequent_backend.operational_signing_official_certificate a
                ON (a.tenant_id,a.event_id,a.certificate_id,a.actor,a.enrollment_id,a.certificate_fingerprint)
                 = (b.tenant,b.event,c.id,b.official::text,g.enrollment,g.fingerprint)
              WHERE (d.attempt,d.bound,d.officer,d.pins,d.trust,d.certificate,d.attached_at)
                  = (ticket,b,who,expected,t.stamp,g.certificate,g.attached_at)
                AND (c.tenant_id,c.election_event_id,c.election_id,c.user_id,c.registered_by)
                  = (b.tenant,b.event,b.post,b.official::text,who::text)
                AND c.status = 'active' AND c.registration = 'security-officer' AND a.retired_at IS NULL
                AND c.fingerprint_sha256 = g.fingerprint AND g.fingerprint = expected->>'leaf_sha256'
                AND c.spki_sha256 = g.spki AND felix3_initialize_attachment.certificate_hash(c.pem) = expected->>'leaf_sha256'
                AND c.not_before <= clock_timestamp() AND c.not_after > clock_timestamp()
                AND NOT EXISTS (SELECT FROM sequent_backend.staff_certificate other
                    WHERE (other.tenant_id,other.election_event_id) = (b.tenant,b.event) AND other.id <> c.id
                      AND (other.spki_sha256 = g.spki OR other.fingerprint_sha256 = g.fingerprint
                           OR (other.holder_sha256 = c.holder_sha256 AND other.user_id <> c.user_id)))
                AND EXISTS (SELECT FROM sequent_backend.operational_initialize_current(
                    b.tenant,b.event,b.request,b.official::text,b.payload_sha256,'softhsm-test'))));
END;
$$;

-- Called in the verifier's same transaction with its independently verified
-- material. The registry connection has obtained its identity, but has taken
-- no event/registration locks. Commit this admission before registration.
CREATE FUNCTION felix3_initialize_attachment.admit(
    b felix3_initialize_setup.binding, ticket uuid, who uuid, ol text[], nl text[], auth_time timestamptz,
    expected jsonb, checked_stamp text, checked_until timestamptz, writer_login name, writer_pid integer, tx xid8
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path = pg_catalog, pg_temp AS $$
DECLARE
    m record;
    cutoff timestamptz;
BEGIN
    IF felix3_initialize_attachment.officer_service() IS NOT TRUE OR writer_pid <= 0 OR writer_pid = pg_backend_pid()
        OR writer_login = session_user OR NOT felix3_initialize_attachment.registry_writer(writer_login)
        OR NOT isfinite(checked_until) OR pg_xact_status(tx) IS DISTINCT FROM 'in progress'
        OR NOT EXISTS (SELECT FROM pg_stat_activity a WHERE a.pid = writer_pid
            AND a.datname = current_database() AND a.usename = writer_login) THEN RETURN false; END IF;
    SELECT * INTO m FROM felix3_initialize_attachment.inspect(b,ticket,who,ol,nl,auth_time,expected);
    IF NOT FOUND OR m.certificate IS NOT NULL OR m.stamp <> checked_stamp THEN RETURN false; END IF;
    cutoff := least(checked_until,m.deadline,clock_timestamp() + interval '30 seconds');
    IF cutoff <= clock_timestamp() THEN RETURN false; END IF;
    INSERT INTO felix3_initialize_attachment.decision AS d
        (attempt,bound,scope,officer,pins,fixed_pins,trust,officer_labels,nominee_labels,writer,backend,xid,until_at)
      VALUES (ticket,b,to_jsonb(b) - ARRAY['request','payload_sha256'],who,expected,
              expected - ARRAY['leaf_sha256','crl_sha256'],checked_stamp,ol,nl,writer_login,writer_pid,tx,cutoff)
      ON CONFLICT (attempt) DO UPDATE SET bound = excluded.bound,pins = excluded.pins,trust = excluded.trust,
          officer_labels = excluded.officer_labels,nominee_labels = excluded.nominee_labels,
          backend = excluded.backend,xid = excluded.xid,until_at = excluded.until_at
        -- inspect held the current generation, Post and trust locks. Only a
        -- known terminal old writer can be replaced; lease expiry, PID reuse
        -- and unavailable xid history are not evidence of that outcome.
        WHERE (d.scope,d.officer,d.fixed_pins,d.writer) = (excluded.scope,excluded.officer,excluded.fixed_pins,excluded.writer)
          AND d.certificate IS NULL AND d.xid <> excluded.xid
          AND pg_xact_status(d.xid) IN ('aborted','committed');
    IF FOUND THEN RETURN true; END IF;
    -- ON CONFLICT also locked a non-updated row. A repeat by the very same
    -- writer is read-only: no rebind, different proof, or deadline extension.
    RETURN EXISTS (SELECT FROM felix3_initialize_attachment.decision d
        WHERE (d.attempt,d.bound,d.officer,d.pins,d.trust,d.officer_labels,d.nominee_labels)
            = (ticket,b,who,expected,checked_stamp,ol,nl)
          AND (d.writer,d.backend,d.xid) = (writer_login,writer_pid,tx)
          AND d.certificate IS NULL AND d.until_at > clock_timestamp() AND d.until_at <= cutoff);
END;
$$;

-- The existing registry writer gets only this entry. A separate authorized
-- officer decision must already name THIS database transaction and backend.
CREATE FUNCTION felix3_initialize_attachment.finish(ticket uuid, registered uuid)
RETURNS timestamptz LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path = pg_catalog, pg_temp AS $$
DECLARE
    d felix3_initialize_attachment.decision%ROWTYPE;
    b felix3_initialize_setup.binding;
    g record;
    t record;
    c record;
    settled timestamptz;
BEGIN
    IF inet_client_addr() IS NOT NULL OR (system_user LIKE 'peer:%') IS NOT TRUE
       OR NOT felix3_initialize_attachment.registry_writer(session_user)
       OR current_setting('transaction_isolation') <> 'read committed' THEN RETURN NULL; END IF;
    SELECT * INTO d FROM felix3_initialize_attachment.decision v WHERE v.attempt = ticket
      AND (v.writer,v.backend,v.xid) = (session_user,pg_backend_pid(),pg_current_xact_id())
      AND v.certificate IS NULL AND v.until_at > clock_timestamp();
    IF NOT FOUND THEN RETURN NULL; END IF;
    b := d.bound;
    SELECT * INTO g FROM felix3_initialize_attachment.generation(b,ticket,d.officer,d.officer_labels,d.nominee_labels);
    IF NOT FOUND OR g.certificate IS NOT NULL OR (g.enrollment::text,g.spki,encode(sha256(convert_to(g.csr,'UTF8')),'hex'))
        IS DISTINCT FROM (d.pins->>'enrollment',d.pins->>'spki_sha256',d.pins->>'csr_sha256') THEN RETURN NULL; END IF;
    PERFORM felix3_initialize_attachment.lock_registry();
    SELECT * INTO t FROM felix3_initialize_attachment.trust(b,d.pins);
    IF NOT FOUND OR t.stamp <> d.trust THEN RETURN NULL; END IF;
    -- Recheck and lock in the same event-before-decision order as admission.
    PERFORM FROM felix3_initialize_attachment.decision v WHERE v.attempt = ticket
      AND (v.writer,v.backend,v.xid) = (session_user,pg_backend_pid(),pg_current_xact_id())
      AND v.certificate IS NULL AND v.until_at > clock_timestamp() FOR UPDATE;
    IF NOT FOUND THEN RETURN NULL; END IF;
    SELECT s.id,s.pem,s.fingerprint_sha256,s.not_after INTO c FROM sequent_backend.staff_certificate s
      WHERE (s.tenant_id,s.election_event_id,s.election_id,s.id,s.user_id,s.registered_by)
          = (b.tenant,b.event,b.post,registered,b.official::text,d.officer::text)
        AND s.xmin = pg_current_xact_id()::xid AND s.status = 'active' AND s.registration = 'security-officer'
        AND s.spki_sha256 = g.spki AND s.fingerprint_sha256 = d.pins->>'leaf_sha256'
        AND felix3_initialize_attachment.certificate_hash(s.pem) = d.pins->>'leaf_sha256'
        AND s.not_before <= clock_timestamp() AND s.not_after > clock_timestamp()
        AND NOT EXISTS (SELECT FROM sequent_backend.signing_approval v
            WHERE (v.tenant_id,v.election_event_id,v.request_id,v.user_id)
              = (b.tenant,b.event,b.request,b.official::text))
        AND NOT EXISTS (SELECT FROM sequent_backend.staff_certificate other
            WHERE (other.tenant_id,other.election_event_id) = (b.tenant,b.event) AND other.id <> s.id
              AND (other.spki_sha256 = g.spki OR other.fingerprint_sha256 = s.fingerprint_sha256
                   OR (other.holder_sha256 = s.holder_sha256 AND other.user_id <> s.user_id)));
    IF NOT FOUND OR least(g.deadline,t.deadline,d.until_at) <= clock_timestamp() THEN RETURN NULL; END IF;
    IF sequent_backend.operational_setup_attach_certificate(b.tenant,b.event,b.official,d.officer,
        ticket,g.enrollment,c.id,c.fingerprint_sha256,ARRAY[c.pem,t.root_pem]) IS NOT TRUE THEN
        RAISE EXCEPTION 'local certificate attachment refused'; END IF;
    SELECT s.attached_at INTO STRICT settled FROM sequent_backend.operational_setup_generation s
        WHERE (s.attempt,s.step,s.enrollment_id,s.attached_by,s.certificate_id,s.certificate_fingerprint)
          = (ticket,'attached'::sequent_backend.operational_setup_step,g.enrollment,d.officer,c.id,c.fingerprint_sha256);
    IF least(g.deadline,t.deadline,d.until_at,c.not_after) <= clock_timestamp() OR NOT EXISTS (
        SELECT FROM sequent_backend.operational_initialize_current(
            b.tenant,b.event,b.request,b.official::text,b.payload_sha256,'softhsm-test')) THEN
        RAISE EXCEPTION 'local certificate attachment lost eligibility'; END IF;
    UPDATE felix3_initialize_attachment.decision SET certificate = c.id,attached_at = settled WHERE attempt = ticket;
    RETURN settled;
END;
$$;
REVOKE ALL ON FUNCTION
    felix3_initialize_attachment.officer_service(),
    felix3_initialize_attachment.registry_writer(name),
    felix3_initialize_attachment.generation(felix3_initialize_setup.binding,uuid,uuid,text[],text[]),
    felix3_initialize_attachment.trust(felix3_initialize_setup.binding,jsonb),
    felix3_initialize_attachment.certificate_hash(text),
    felix3_initialize_attachment.review_csr(felix3_initialize_setup.binding,uuid,uuid,text[],text[],timestamptz),
    felix3_initialize_attachment.inspect(felix3_initialize_setup.binding,uuid,uuid,text[],text[],timestamptz,jsonb),
    felix3_initialize_attachment.admit(felix3_initialize_setup.binding,uuid,uuid,text[],text[],timestamptz,jsonb,text,timestamptz,name,integer,xid8),
    felix3_initialize_attachment.finish(uuid,uuid) FROM PUBLIC;
RESET ROLE;
DO $$
BEGIN
    IF EXISTS (SELECT FROM unnest(ARRAY['attempt','scope','fixed_pins','officer','writer']) AS k(col)
        WHERE has_column_privilege('sequent_os_ledger_api_owner'::regrole,
            'felix3_initialize_attachment.decision'::regclass,k.col,'UPDATE'))
    OR EXISTS (SELECT FROM pg_attribute c CROSS JOIN LATERAL aclexplode(c.attacl) a
        WHERE c.attrelid = 'felix3_initialize_attachment.decision'::regclass
          AND a.grantee NOT IN ('sequent_os_ledger_data_owner'::regrole,'sequent_os_ledger_api_owner'::regrole))
    OR EXISTS (SELECT FROM pg_proc p CROSS JOIN LATERAL aclexplode(coalesce(p.proacl,acldefault('f',p.proowner))) a
        WHERE p.pronamespace = 'felix3_initialize_attachment'::regnamespace AND a.grantee <> p.proowner
          AND NOT (p.oid IN ('felix3_initialize_attachment.lock_registry()'::regprocedure,
                   'felix3_initialize_attachment.post_access(felix3_initialize_setup.binding,text[],text[])'::regprocedure)
                   AND a.grantee = 'sequent_os_ledger_api_owner'::regrole))
    OR EXISTS (SELECT FROM pg_class c CROSS JOIN LATERAL aclexplode(c.relacl) a
        WHERE c.oid = 'felix3_initialize_attachment.decision'::regclass
          AND a.grantee NOT IN (c.relowner,'sequent_os_ledger_api_owner'::regrole))
    OR EXISTS (SELECT FROM pg_namespace n CROSS JOIN LATERAL aclexplode(n.nspacl) a
        WHERE n.oid = 'felix3_initialize_attachment'::regnamespace
          AND a.grantee NOT IN (n.nspowner,'sequent_os_ledger_data_owner'::regrole))
    THEN RAISE EXCEPTION 'unexpected local certificate privileges'; END IF;
END;
$$;
