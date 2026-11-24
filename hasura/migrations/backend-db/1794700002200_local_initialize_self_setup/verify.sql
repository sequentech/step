-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Read-only PostgreSQL contract checks. No service login, operator record,
-- challenge, enrollment, certificate or signing request is created.
DO $$
DECLARE
    b felix3_initialize_setup.binding;
    subject jsonb;
    body jsonb;
    canonical text;
    field text;
    mutated jsonb;
    n integer;
    expiry constant timestamptz := '2001-02-03T04:05:06Z';
BEGIN
    SELECT ids[1],ids[2],ids[3],ids[4],ids[5],ids[6],ids[7],'' INTO b
      FROM (SELECT array_agg(lpad(v.i::text,32,'0')::uuid ORDER BY v.i) AS ids
            FROM generate_series(1,7) AS v(i)) public_inputs;
    subject := jsonb_build_object('publication_id',b.publication);
    body := jsonb_build_object('domain','step-signing/v1','action','initialize-voting',
        'tenant_id',b.tenant,'election_event_id',b.event,'election_id',b.post,'request_id',b.request,'subject',subject,
        'expires_at','2001-02-03T04:05:06.123456789Z');
    canonical := body::text;
    b.payload_sha256 := encode(sha256(convert_to(canonical,'UTF8')),'hex');
    IF felix3_initialize_setup.payload_matches(b,subject,canonical,expiry) IS NOT TRUE
       OR felix3_initialize_setup.payload_matches(b,subject,canonical || ' ',expiry) IS TRUE
       OR felix3_initialize_setup.payload_matches(b,subject,canonical,expiry + interval '1 second') IS TRUE
       OR felix3_initialize_setup.payload_matches(b,'{}',canonical,expiry) IS TRUE THEN
        RAISE EXCEPTION 'exact normal Initialize payload contract failed';
    END IF;
    FOREACH field IN ARRAY ARRAY['domain','action','tenant_id','election_event_id','election_id','request_id','subject','expires_at'] LOOP
        mutated := body - field;
        b.payload_sha256 := encode(sha256(convert_to(mutated::text,'UTF8')),'hex');
        IF felix3_initialize_setup.payload_matches(b,subject,mutated::text,expiry) IS TRUE THEN
            RAISE EXCEPTION 'missing signed % was accepted', field;
        END IF;
    END LOOP;
    canonical := '{"domain":"step-signing/v1",' || substr(body::text,2);
    b.payload_sha256 := encode(sha256(convert_to(canonical,'UTF8')),'hex');
    IF felix3_initialize_setup.payload_matches(b,subject,canonical,expiry) IS TRUE THEN
        RAISE EXCEPTION 'ambiguous signed JSON was accepted';
    END IF;
    mutated := jsonb_set(body,'{subject,publication_id}',to_jsonb(b.post));
    b.payload_sha256 := encode(sha256(convert_to(mutated::text,'UTF8')),'hex');
    IF felix3_initialize_setup.payload_matches(b,mutated->'subject',mutated::text,expiry) IS TRUE THEN
        RAISE EXCEPTION 'different publication was accepted';
    END IF;
    IF NOT felix3_initialize_setup.fresh_gold(10,11,12,14,13)
       OR felix3_initialize_setup.fresh_gold(10,10,12,14,13)
       OR felix3_initialize_setup.fresh_gold(10,11,12,14,14)
       OR felix3_initialize_setup.fresh_gold(10,11,13,99,12)
       OR felix3_initialize_setup.fresh_gold(10,11,12,99,71) THEN
        RAISE EXCEPTION 'setup Gold time bounds failed';
    END IF;
    SELECT count(*) INTO n FROM pg_proc p
      WHERE p.pronamespace = 'felix3_initialize_setup'::regnamespace AND p.prosecdef
        AND p.proowner = 'sequent_os_ledger_api_owner'::regrole;
    IF n <> 7 OR EXISTS (
        SELECT FROM pg_proc p CROSS JOIN LATERAL aclexplode(coalesce(p.proacl,acldefault('f',p.proowner))) a
        WHERE p.pronamespace = 'felix3_initialize_setup'::regnamespace AND a.grantee = 0
    ) THEN RAISE EXCEPTION 'unexpected local setup entry points or PUBLIC access'; END IF;
END;
$$;
