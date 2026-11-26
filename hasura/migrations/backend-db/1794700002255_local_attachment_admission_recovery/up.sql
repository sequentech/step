-- SPDX-License-Identifier: AGPL-3.0-only
-- Apply transactionally after the original 2250. Do not replace a previously applied migration.
LOCK TABLE felix3_initialize_attachment.decision IN ACCESS EXCLUSIVE MODE NOWAIT;
DO $$
DECLARE
    ledger oid := 'felix3_initialize_attachment.decision'::regclass;
    data_role oid := 'sequent_os_ledger_data_owner'::regrole;
    api_role oid := 'sequent_os_ledger_api_owner'::regrole;
BEGIN
    IF (SELECT array_agg(c.attname::text ORDER BY c.attnum) FROM pg_attribute c
            WHERE c.attrelid = ledger AND c.attnum > 0 AND NOT c.attisdropped)
            IS DISTINCT FROM ARRAY['attempt','bound','officer','pins','trust','officer_labels',
                'nominee_labels','writer','backend','xid','until_at','certificate','attached_at']
       OR NOT EXISTS (SELECT FROM pg_class c WHERE c.oid = ledger AND c.relowner = data_role
                      AND c.relkind = 'r' AND NOT c.relrowsecurity AND NOT c.relforcerowsecurity)
    THEN RAISE EXCEPTION 'local recovery requires the original 2250 decision layout'
       USING HINT = 'An in-place pre-release 2250 is not an upgrade source. Reconcile the ledger before retrying.';
    END IF;
    IF EXISTS (SELECT FROM pg_class c CROSS JOIN LATERAL aclexplode(coalesce(c.relacl,acldefault('r',c.relowner))) a
            WHERE c.oid = ledger AND a.grantee <> data_role
              AND (a.grantee <> api_role OR a.privilege_type NOT IN ('SELECT','INSERT') OR a.is_grantable))
       OR EXISTS (SELECT FROM pg_attribute c CROSS JOIN LATERAL aclexplode(c.attacl) a
            WHERE c.attrelid = ledger AND a.grantee <> data_role
              AND (a.grantee <> api_role OR a.privilege_type <> 'UPDATE' OR a.is_grantable))
       OR NOT has_table_privilege(api_role,ledger,'SELECT') OR NOT has_table_privilege(api_role,ledger,'INSERT')
       OR (SELECT array_agg(c.attname::text ORDER BY c.attname COLLATE "C")
            FROM pg_attribute c
            WHERE c.attrelid = 'felix3_initialize_attachment.decision'::regclass AND c.attnum > 0 AND NOT c.attisdropped
              AND has_column_privilege('sequent_os_ledger_api_owner'::regrole,c.attrelid,c.attnum,'UPDATE'))
            IS DISTINCT FROM ARRAY['attached_at','backend','certificate','nominee_labels','officer_labels','trust','until_at','writer','xid']
       OR (SELECT count(*) FROM pg_proc p
            WHERE p.oid IN ('felix3_initialize_attachment.inspect(felix3_initialize_setup.binding,uuid,uuid,text[],text[],timestamptz,jsonb)'::regprocedure,
                            'felix3_initialize_attachment.admit(felix3_initialize_setup.binding,uuid,uuid,text[],text[],timestamptz,jsonb,text,timestamptz,name,integer,xid8)'::regprocedure)
              AND p.proowner = api_role AND p.prosecdef AND p.proisstrict AND p.provolatile = 'v'
              AND p.proconfig = ARRAY['search_path=pg_catalog, pg_temp']
              AND NOT EXISTS (SELECT FROM aclexplode(coalesce(p.proacl,acldefault('f',p.proowner))) a
                WHERE a.grantee = 0)) <> 2
    THEN RAISE EXCEPTION 'unexpected 2250 ledger or admission privileges'; END IF;
END;
$$;

SET LOCAL ROLE sequent_os_ledger_data_owner;
-- These are immutable snapshots, not generated columns that follow a future UPDATE.
ALTER TABLE felix3_initialize_attachment.decision ADD COLUMN scope jsonb, ADD COLUMN fixed_pins jsonb;
UPDATE felix3_initialize_attachment.decision
    SET scope = to_jsonb(bound) - ARRAY['request','payload_sha256'],
        fixed_pins = pins - ARRAY['leaf_sha256','crl_sha256'];
ALTER TABLE felix3_initialize_attachment.decision
    ALTER COLUMN scope SET NOT NULL,
    ALTER COLUMN fixed_pins SET NOT NULL,
    ADD CONSTRAINT decision_fixed_scope CHECK (scope = (to_jsonb(bound) - ARRAY['request','payload_sha256'])),
    ADD CONSTRAINT decision_fixed_pins CHECK (fixed_pins = (pins - ARRAY['leaf_sha256','crl_sha256']));
REVOKE UPDATE (writer) ON felix3_initialize_attachment.decision FROM sequent_os_ledger_api_owner;
GRANT UPDATE (bound,pins) ON felix3_initialize_attachment.decision TO sequent_os_ledger_api_owner;
RESET ROLE;

-- Replace as the existing owner; unlike dropping/recreating, this preserves installed
-- officer/writer function ACLs. No runtime principal gets direct decision-table access.
SET LOCAL ROLE sequent_os_ledger_api_owner;

CREATE OR REPLACE FUNCTION felix3_initialize_attachment.inspect(
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

CREATE OR REPLACE FUNCTION felix3_initialize_attachment.admit(
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
        -- pg_locks exposes the low xid of the holder. Combine that lock with
        -- the full, live xid8 above: an unrelated live PID/XID pair is not a writer.
        OR NOT EXISTS (SELECT FROM pg_stat_activity a JOIN pg_locks l ON l.pid = a.pid
            WHERE a.pid = writer_pid AND a.datname = current_database() AND a.usename = writer_login
              AND l.locktype = 'transactionid' AND l.mode = 'ExclusiveLock' AND l.granted
              AND l.transactionid = tx::xid) THEN RETURN false; END IF;
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

RESET ROLE;
DO $$
BEGIN
    IF (SELECT array_agg(c.attname::text ORDER BY c.attname COLLATE "C")
            FROM pg_attribute c
            WHERE c.attrelid = 'felix3_initialize_attachment.decision'::regclass AND c.attnum > 0 AND NOT c.attisdropped
              AND has_column_privilege('sequent_os_ledger_api_owner'::regrole,c.attrelid,c.attnum,'UPDATE'))
            IS DISTINCT FROM ARRAY['attached_at','backend','bound','certificate','nominee_labels','officer_labels','pins','trust','until_at','xid']
    THEN RAISE EXCEPTION 'unexpected local recovery column privileges'; END IF;
END;
$$;
