#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Actual production migration/function in an isolated disposable PostgreSQL DB."""
from pathlib import Path
import subprocess,uuid
migrations=Path(__file__).resolve().parents[2]/'hasura/migrations/backend-db';database='account_threshold_'+uuid.uuid4().hex

def sql(database,command):
    result=subprocess.run(['sudo','docker','exec','-i','agd-st-postgres','psql','-U','postgres','-v','ON_ERROR_STOP=1','-d',database],input=command,text=True,capture_output=True,timeout=20)
    if result.returncode:raise RuntimeError(result.stderr)
    return result.stdout
sql('postgres','CREATE DATABASE '+database)
try:
    sql(database,'''CREATE SCHEMA sequent_backend; CREATE TABLE sequent_backend.tenant(id uuid PRIMARY KEY,is_active boolean); CREATE TABLE sequent_backend.election_event(id uuid PRIMARY KEY,tenant_id uuid);
CREATE TABLE sequent_backend.election_lifecycle(tenant_id uuid,election_event_id uuid,phase text,safety text,generation bigint);
CREATE TABLE sequent_backend.lifecycle_service_lease(component text,expires_at timestamptz);
CREATE TABLE sequent_backend.lifecycle_notice(id uuid,tenant_id uuid,election_event_id uuid,kind text,details jsonb,created_at timestamptz);
CREATE TABLE sequent_backend.self_test_run(id uuid,tenant_id uuid,election_event_id uuid,generation bigint,outcome text,started_at timestamptz,finished_at timestamptz);
CREATE TABLE sequent_backend.self_test_settings(tenant_id uuid,election_event_id uuid,settings jsonb);
CREATE TABLE sequent_backend.self_test_baseline(tenant_id uuid,election_event_id uuid);
CREATE FUNCTION sequent_backend.audit_storage_allows_voters() RETURNS boolean LANGUAGE sql AS 'SELECT true';
INSERT INTO sequent_backend.tenant VALUES('11111111-1111-4111-8111-111111111111',true),('22222222-2222-4222-8222-222222222222',true),('44444444-4444-4444-8444-444444444444',true);
INSERT INTO sequent_backend.election_event VALUES('33333333-3333-4333-8333-333333333333','11111111-1111-4111-8111-111111111111'),('55555555-5555-4555-8555-555555555555','22222222-2222-4222-8222-222222222222');'''+(migrations/'1793020007000_attack_heuristics/up.sql').read_text()+'''
INSERT INTO sequent_backend.attack_detection_settings(tenant_id,failed_logins_per_minute,requests_per_minute,updated_by) VALUES('11111111-1111-4111-8111-111111111111',1000,2000,'administrator');'''+(migrations/'1793020011000_account_failure_threshold/up.sql').read_text())
    sql(database,'''CREATE FUNCTION public.fail(tenant uuid,event uuid,realm text,account text,at timestamptz,win integer DEFAULT 900,lim integer DEFAULT 2) RETURNS boolean LANGUAGE sql AS $$ SELECT sequent_backend.record_attack_auth_activity(gen_random_uuid(),tenant,event,realm,account,at,lim,win) $$;
CREATE FUNCTION public.found(tenant uuid,key text) RETURNS bigint LANGUAGE sql AS $$ SELECT count(*) FROM sequent_backend.attack_detection d WHERE d.tenant_id=tenant AND d.category='brute-force-account' AND d.scope_key=key $$;
DO $test$
DECLARE t uuid:='11111111-1111-4111-8111-111111111111'; other uuid:='22222222-2222-4222-8222-222222222222'; quiet uuid:='44444444-4444-4444-8444-444444444444'; e uuid:='33333333-3333-4333-8333-333333333333'; k uuid:=gen_random_uuid(); at timestamptz:=date_trunc('minute',clock_timestamp())-interval '10 days'; d record; i integer;
BEGIN
 -- An existing row and a tenant without a row both use the default, not the realm's limit.
 IF (SELECT account_failure_threshold FROM sequent_backend.attack_detection_settings WHERE tenant_id=t)<>5 THEN RAISE EXCEPTION 'existing settings row without the default'; END IF;
 FOR i IN 1..4 LOOP PERFORM public.fail(other,NULL,'staff','a',at+make_interval(secs=>i)); END LOOP;
 IF public.found(other,'staff:a')<>0 THEN RAISE EXCEPTION 'realm limit still decides'; END IF;
 PERFORM public.fail(other,NULL,'staff','a',at+interval '5 seconds');
 SELECT * INTO d FROM sequent_backend.attack_detection WHERE tenant_id=other;
 IF d.threshold<>5 OR d.observed_count<>5 OR d.election_event_id IS NOT NULL THEN RAISE EXCEPTION 'default threshold not recorded: %',d; END IF;
 IF EXISTS(SELECT 1 FROM sequent_backend.attack_auth_activity WHERE tenant_id=other AND failure_limit<>2) THEN RAISE EXCEPTION 'realm limit no longer recorded'; END IF;
 IF public.found(t,'staff:a')<>0 THEN RAISE EXCEPTION 'tenant leakage'; END IF;

 -- The tenant's setting fires exactly at the threshold, once per window.
 UPDATE sequent_backend.attack_detection_settings SET account_failure_threshold=3 WHERE tenant_id=t;
 PERFORM public.fail(t,e,'realm','a',at);
 PERFORM public.fail(t,e,'realm','a',at+interval '1 second');
 IF public.found(t,'realm:a')<>0 THEN RAISE EXCEPTION 'detection before threshold'; END IF;
 IF sequent_backend.record_attack_auth_activity(k,t,e,'realm','a',at+interval '1 second',2,900) IS NOT TRUE THEN RAISE EXCEPTION 'first delivery refused'; END IF;
 IF public.found(t,'realm:a')<>1 THEN RAISE EXCEPTION 'threshold not detected'; END IF;
 SELECT * INTO d FROM sequent_backend.attack_detection WHERE tenant_id=t AND scope_key='realm:a';
 IF d.threshold<>3 OR d.observed_count<>3 OR d.election_event_id<>e OR d.window_start<>at+interval '1 second'-interval '900 seconds' OR d.window_end<>at+interval '1 minute' OR (d.details->>'window_seconds')::int<>900 OR d.details->>'scope'<>'one account' THEN RAISE EXCEPTION 'unexpected detection: %',d; END IF;
 IF sequent_backend.record_attack_auth_activity(k,t,e,'realm','a',at+interval '1 second',2,900) IS NOT FALSE THEN RAISE EXCEPTION 'duplicate delivery counted'; END IF;
 -- Every later failure within the window, in the same or a later minute, adds no record.
 FOR i IN 2..958 BY 2 LOOP PERFORM public.fail(t,e,'realm','a',at+make_interval(secs=>i)); END LOOP;
 IF public.found(t,'realm:a')<>1 THEN RAISE EXCEPTION 'repeats not bounded: %',public.found(t,'realm:a'); END IF;
 -- A full window later, only failures after the detection count again.
 PERFORM public.fail(t,e,'realm','b',at);
 PERFORM public.fail(t,e,'realm','b',at+interval '1 second');
 PERFORM public.fail(t,e,'realm','b',at+interval '2 seconds');
 PERFORM public.fail(t,e,'realm','b',at+interval '961 seconds');
 PERFORM public.fail(t,e,'realm','b',at+interval '962 seconds');
 IF public.found(t,'realm:b')<>1 THEN RAISE EXCEPTION 'reported failures counted twice'; END IF;
 PERFORM public.fail(t,e,'realm','b',at+interval '963 seconds');
 IF public.found(t,'realm:b')<>2 THEN RAISE EXCEPTION 'next window not detected'; END IF;
 -- A threshold of one is still one detection per window.
 UPDATE sequent_backend.attack_detection_settings SET account_failure_threshold=1 WHERE tenant_id=t;
 FOR i IN 0..50 LOOP PERFORM public.fail(t,e,'realm','one',at+make_interval(secs=>i)); END LOOP;
 IF public.found(t,'realm:one')<>1 THEN RAISE EXCEPTION 'threshold one not bounded'; END IF;
 -- A changed threshold applies at once to what the window holds.
 UPDATE sequent_backend.attack_detection_settings SET account_failure_threshold=4 WHERE tenant_id=t;
 FOR i IN 0..2 LOOP PERFORM public.fail(t,e,'realm','change',at+make_interval(secs=>i)); END LOOP;
 PERFORM public.fail(t,e,'realm','change',at+interval '3 seconds');
 IF public.found(t,'realm:change')<>1 THEN RAISE EXCEPTION 'first threshold not detected'; END IF;
 UPDATE sequent_backend.attack_detection_settings SET account_failure_threshold=2 WHERE tenant_id=t;
 PERFORM public.fail(t,e,'realm','change',at+interval '125 seconds');
 IF public.found(t,'realm:change')<>2 THEN RAISE EXCEPTION 'changed threshold ignored'; END IF;
 PERFORM public.fail(t,e,'realm','change',at+interval '185 seconds');
 IF public.found(t,'realm:change')<>2 THEN RAISE EXCEPTION 'changed threshold not bounded'; END IF;

 -- Failures without an account count together per realm, over at most 15 minutes.
 UPDATE sequent_backend.attack_detection_settings SET account_failure_threshold=3 WHERE tenant_id=t;
 PERFORM public.fail(t,e,'broker',NULL,at,43200);
 PERFORM public.fail(t,e,'broker',NULL,at+interval '901 seconds',43200);
 PERFORM public.fail(t,e,'broker',NULL,at+interval '902 seconds',43200);
 IF public.found(t,'broker:unidentified')<>0 THEN RAISE EXCEPTION 'unidentified window not limited'; END IF;
 PERFORM public.fail(t,NULL,'staff',NULL,at+interval '900 seconds',43200);
 PERFORM public.fail(t,NULL,'staff',NULL,at+interval '901 seconds',43200);
 PERFORM public.fail(t,e,'broker','named',at+interval '903 seconds',43200);
 IF public.found(t,'broker:unidentified')<>0 THEN RAISE EXCEPTION 'realms or named accounts counted together'; END IF;
 PERFORM public.fail(t,e,'broker',NULL,at+interval '903 seconds',43200);
 IF public.found(t,'broker:unidentified')<>1 THEN RAISE EXCEPTION 'unidentified failures not detected'; END IF;
 SELECT * INTO d FROM sequent_backend.attack_detection WHERE tenant_id=t AND scope_key='broker:unidentified';
 IF d.threshold<>3 OR d.observed_count<>3 OR d.election_event_id<>e OR (d.details->>'window_seconds')::int<>900 OR d.details->>'account_sha256' IS NOT NULL OR d.details->>'scope'<>'failures without an identified account' OR d.window_start<>at+interval '3 seconds' THEN RAISE EXCEPTION 'unexpected unidentified detection: %',d; END IF;
 FOR i IN 904..1740 BY 3 LOOP PERFORM public.fail(t,e,'broker',NULL,at+make_interval(secs=>i),43200); END LOOP;
 IF public.found(t,'broker:unidentified')<>1 THEN RAISE EXCEPTION 'unidentified repeats not bounded: %',public.found(t,'broker:unidentified'); END IF;
 FOR i IN 1860..1900 BY 3 LOOP PERFORM public.fail(t,e,'broker',NULL,at+make_interval(secs=>i),43200); END LOOP;
 IF public.found(t,'broker:unidentified')<>2 THEN RAISE EXCEPTION 'unidentified next window not detected'; END IF;
 PERFORM public.fail(t,NULL,'staff',NULL,at+interval '902 seconds',43200);
 IF public.found(t,'staff:unidentified')<>1 OR (SELECT election_event_id FROM sequent_backend.attack_detection WHERE tenant_id=t AND scope_key='staff:unidentified') IS NOT NULL THEN RAISE EXCEPTION 'staff realm not counted'; END IF;

 -- Out-of-order delivery still reaches the threshold, and only once.
 PERFORM public.fail(t,e,'realm','late',at+interval '15 seconds');
 PERFORM public.fail(t,e,'realm','late',at+interval '10 seconds');
 PERFORM public.fail(t,e,'realm','late',at+interval '5 seconds');
 PERFORM public.fail(t,e,'realm','late',at+interval '6 seconds');
 IF public.found(t,'realm:late')<>1 THEN RAISE EXCEPTION 'out of order activity lost or repeated'; END IF;

 -- The all-accounts rate rule is unchanged, and does not depend on an account.
 UPDATE sequent_backend.attack_detection_settings SET failed_logins_per_minute=2 WHERE tenant_id=t;
 PERFORM public.fail(t,e,'rate',NULL,at-interval '1 day');
 PERFORM public.fail(t,e,'rate',NULL,at-interval '1 day'+interval '1 second');
 IF EXISTS(SELECT 1 FROM sequent_backend.attack_detection WHERE tenant_id=t AND category='brute-force-aggregate') THEN RAISE EXCEPTION 'rate equals not exceeds'; END IF;
 PERFORM public.fail(t,e,'rate',NULL,at-interval '1 day'+interval '2 seconds');
 IF (SELECT threshold FROM sequent_backend.attack_detection WHERE tenant_id=t AND category='brute-force-aggregate')<>2 THEN RAISE EXCEPTION 'aggregate rule changed'; END IF;

 -- Bounds, and the evidence export of a tenant without a settings row.
 BEGIN UPDATE sequent_backend.attack_detection_settings SET account_failure_threshold=0 WHERE tenant_id=t; RAISE EXCEPTION 'zero accepted'; EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN UPDATE sequent_backend.attack_detection_settings SET account_failure_threshold=100001 WHERE tenant_id=t; RAISE EXCEPTION 'excess accepted'; EXCEPTION WHEN check_violation THEN NULL; END;
 UPDATE sequent_backend.attack_detection_settings SET account_failure_threshold=100000 WHERE tenant_id=t;
 UPDATE sequent_backend.attack_detection_settings SET account_failure_threshold=1 WHERE tenant_id=t;
 IF (sequent_backend.execution_security_evidence(other,'55555555-5555-4555-8555-555555555555')->'attack_settings'->>'account_failure_threshold')::int<>5 THEN RAISE EXCEPTION 'default absent from evidence'; END IF;
 IF (sequent_backend.execution_security_evidence(t,e)->'attack_settings'->>'account_failure_threshold')::int<>1 THEN RAISE EXCEPTION 'setting absent from evidence'; END IF;
 IF EXISTS(SELECT 1 FROM sequent_backend.attack_detection WHERE tenant_id=quiet) THEN RAISE EXCEPTION 'tenant leakage'; END IF;
END $test$;''')
    sql(database,(migrations/'1793020011000_account_failure_threshold/down.sql').read_text())
    sql(database,'''DO $test$
DECLARE t uuid:='44444444-4444-4444-8444-444444444444'; at timestamptz:=date_trunc('minute',clock_timestamp())-interval '10 days';
BEGIN
 IF EXISTS(SELECT 1 FROM information_schema.columns WHERE table_schema='sequent_backend' AND table_name='attack_detection_settings' AND column_name='account_failure_threshold') THEN RAISE EXCEPTION 'column not removed'; END IF;
 PERFORM public.fail(t,NULL,'staff',NULL,at);
 PERFORM public.fail(t,NULL,'staff',NULL,at+interval '1 second');
 IF public.found(t,'staff:unidentified')<>0 THEN RAISE EXCEPTION 'previous function not restored'; END IF;
 PERFORM public.fail(t,NULL,'staff','a',at);
 PERFORM public.fail(t,NULL,'staff','a',at+interval '1 second');
 IF public.found(t,'staff:a')<>1 THEN RAISE EXCEPTION 'realm limit not restored'; END IF;
 IF sequent_backend.execution_security_evidence('22222222-2222-4222-8222-222222222222','55555555-5555-4555-8555-555555555555')->'attack_settings' ? 'account_failure_threshold' THEN RAISE EXCEPTION 'previous evidence not restored'; END IF;
END $test$;''')
    print('PASS: real PostgreSQL tenant threshold with default, one detection per window for an account and for failures without an account per realm, changed thresholds, duplicate and out-of-order delivery, bounds, evidence export and rollback')
finally:sql('postgres','DROP DATABASE '+database)
