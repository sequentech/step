#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Actual production migration/function in an isolated disposable PostgreSQL DB."""
from pathlib import Path
import subprocess,uuid
root=Path(__file__).resolve().parents[2];database='attack_'+uuid.uuid4().hex

def sql(database,command):
    result=subprocess.run(['sudo','docker','exec','-i','agd-st-postgres','psql','-U','postgres','-v','ON_ERROR_STOP=1','-d',database],input=command,text=True,capture_output=True,timeout=20)
    if result.returncode:raise RuntimeError(result.stderr)
    return result.stdout
sql('postgres','CREATE DATABASE '+database)
try:
    sql(database,'''CREATE SCHEMA sequent_backend; CREATE TABLE sequent_backend.tenant(id uuid PRIMARY KEY,is_active boolean); CREATE TABLE sequent_backend.election_event(id uuid PRIMARY KEY,tenant_id uuid);
CREATE TABLE sequent_backend.election_lifecycle(tenant_id uuid,election_event_id uuid,phase text,safety text,generation bigint);
CREATE TABLE sequent_backend.lifecycle_service_lease(component text,expires_at timestamptz);
CREATE TABLE sequent_backend.self_test_run(id uuid,tenant_id uuid,election_event_id uuid,generation bigint,outcome text,finished_at timestamptz);
CREATE TABLE sequent_backend.self_test_settings(tenant_id uuid,election_event_id uuid,settings jsonb);
CREATE FUNCTION sequent_backend.audit_storage_allows_voters() RETURNS boolean LANGUAGE sql AS 'SELECT true';
INSERT INTO sequent_backend.tenant VALUES('11111111-1111-4111-8111-111111111111',true),('22222222-2222-4222-8222-222222222222',true);
INSERT INTO sequent_backend.election_event VALUES('33333333-3333-4333-8333-333333333333','11111111-1111-4111-8111-111111111111');'''+(root/'hasura/migrations/backend-db/1793020007000_attack_heuristics/up.sql').read_text())
    sql(database,'''DO $test$
DECLARE t uuid:='11111111-1111-4111-8111-111111111111'; other uuid:='22222222-2222-4222-8222-222222222222'; e uuid:='33333333-3333-4333-8333-333333333333'; k uuid:=gen_random_uuid(); at timestamptz:=date_trunc('minute',clock_timestamp())-interval '1 minute'; n integer;
BEGIN
 IF sequent_backend.attack_activity_allows_voters() THEN RAISE EXCEPTION 'missing activity health allowed voting'; END IF;
 INSERT INTO sequent_backend.attack_activity_health VALUES('external-nginx',true,clock_timestamp(),'test');
 IF NOT sequent_backend.attack_activity_allows_voters() THEN RAISE EXCEPTION 'fresh available activity refused'; END IF;
 UPDATE sequent_backend.attack_activity_health SET checked_at=clock_timestamp()-interval '31 seconds';
 IF sequent_backend.attack_activity_allows_voters() THEN RAISE EXCEPTION 'stale activity allowed voting'; END IF;
 UPDATE sequent_backend.attack_activity_health SET checked_at=clock_timestamp(),available=false;
 IF sequent_backend.attack_activity_allows_voters() THEN RAISE EXCEPTION 'unavailable activity allowed voting'; END IF;
 INSERT INTO sequent_backend.attack_detection_settings(tenant_id,failed_logins_per_minute,requests_per_minute,updated_by) VALUES(t,2,3,'administrator');
 PERFORM sequent_backend.record_attack_auth_activity(k,t,e,'realm','a',at,2,900);
 IF EXISTS(SELECT 1 FROM sequent_backend.attack_detection) THEN RAISE EXCEPTION 'detection before threshold'; END IF;
 PERFORM sequent_backend.record_attack_auth_activity(k,t,e,'realm','a',at,2,900);
 IF (SELECT count(*) FROM sequent_backend.attack_auth_activity)<>1 THEN RAISE EXCEPTION 'duplicate counted twice'; END IF;
 PERFORM sequent_backend.record_attack_auth_activity(gen_random_uuid(),t,e,'realm','a',at+interval '1 second',2,900);
 IF (SELECT count(*) FROM sequent_backend.attack_detection WHERE category='brute-force-account')<>1 THEN RAISE EXCEPTION 'account limit not detected'; END IF;
 IF EXISTS(SELECT 1 FROM sequent_backend.attack_detection WHERE category='brute-force-aggregate') THEN RAISE EXCEPTION 'rate equals not exceeds'; END IF;
 PERFORM sequent_backend.record_attack_auth_activity(gen_random_uuid(),t,NULL,'realm',NULL,at+interval '2 seconds',2,900);
 IF (SELECT count(*) FROM sequent_backend.attack_detection WHERE category='brute-force-aggregate')<>1 THEN RAISE EXCEPTION 'aggregate absent'; END IF;
 PERFORM sequent_backend.record_attack_auth_activity(gen_random_uuid(),other,NULL,'other','a',at+interval '2 seconds',2,900);
 IF EXISTS(SELECT 1 FROM sequent_backend.attack_detection WHERE tenant_id=other) THEN RAISE EXCEPTION 'tenant leakage'; END IF;
 -- Out-of-order records must still reach the per-account signature.
 PERFORM sequent_backend.record_attack_auth_activity(gen_random_uuid(),t,e,'realm','b',at+interval '15 seconds',2,900);
 PERFORM sequent_backend.record_attack_auth_activity(gen_random_uuid(),t,e,'realm','b',at+interval '10 seconds',2,900);
 IF (SELECT count(*) FROM sequent_backend.attack_detection WHERE category='brute-force-account')<>2 THEN RAISE EXCEPTION 'out of order activity lost'; END IF;
 -- Request totals cover all interfaces and do not require a user or tenant token.
 INSERT INTO sequent_backend.attack_request_bucket VALUES('fixture-generation',at,'login.example',2,clock_timestamp()),('fixture-generation',at,'vote.example',1,clock_timestamp());
 PERFORM sequent_backend.compare_attack_request_activity();
 IF EXISTS(SELECT 1 FROM sequent_backend.attack_detection WHERE category='network-dos') THEN RAISE EXCEPTION 'request rate equal to threshold detected'; END IF;
 UPDATE sequent_backend.attack_request_bucket SET requests=3 WHERE host='login.example';
 PERFORM sequent_backend.compare_attack_request_activity();
 PERFORM sequent_backend.compare_attack_request_activity();
 IF (SELECT count(*) FROM sequent_backend.attack_detection WHERE category='network-dos')<>1 THEN RAISE EXCEPTION 'cluster request signature absent or duplicated'; END IF;
 IF (SELECT observed_count FROM sequent_backend.attack_detection WHERE category='network-dos')<>4 THEN RAISE EXCEPTION 'request total does not include all interfaces'; END IF;
 IF EXISTS(SELECT 1 FROM sequent_backend.attack_detection WHERE category='network-dos' AND tenant_id=other) THEN RAISE EXCEPTION 'another tenant threshold ignored'; END IF;
 BEGIN UPDATE sequent_backend.attack_detection_settings SET failed_logins_per_minute=0 WHERE tenant_id=t; RAISE EXCEPTION 'zero rate accepted'; EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN PERFORM sequent_backend.record_attack_auth_activity(gen_random_uuid(),t,e,'realm','a',clock_timestamp()+interval '1 day',2,900); RAISE EXCEPTION 'future accepted'; EXCEPTION WHEN raise_exception THEN IF SQLERRM='future accepted' THEN RAISE; END IF; END;
END $test$;''')
    print('PASS: real PostgreSQL auth activity deduplication, exact thresholds, unknown accounts, tenant isolation, out-of-order records and invalid policy/time refusals, unauthenticated cluster request thresholds, and fail-closed source health')
    sql(database,(root/'hasura/migrations/backend-db/1793020007000_attack_heuristics/down.sql').read_text())
finally:sql('postgres','DROP DATABASE '+database)
