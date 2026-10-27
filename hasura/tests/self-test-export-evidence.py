#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Real PostgreSQL: full failed-test preservation and stale-export refusal."""
from pathlib import Path
import argparse
import select
import subprocess
import uuid

ROOT = Path(__file__).resolve().parents[2]
DB = 'selftest_export_' + uuid.uuid4().hex
MIGRATIONS = ROOT / 'hasura/migrations/backend-db'
parser = argparse.ArgumentParser()
parser.add_argument('--without-correction', action='store_true')
args = parser.parse_args()


def sql(database, command):
    result = subprocess.run(['sudo', 'docker', 'exec', '-i', 'agd-st-postgres',
        'psql', '-X', '-qAt', '-U', 'postgres', '-v', 'ON_ERROR_STOP=1', '-d', database],
        input=command, text=True, capture_output=True, timeout=20)
    if result.returncode:
        raise RuntimeError(result.stderr)
    return result.stdout


sql('postgres', 'CREATE DATABASE ' + DB)
try:
    fixture = (ROOT / 'hasura/tests/board-operation-export.sql').read_text().replace(
        r'\ir ../migrations/backend-db/1793020005000_board_operations/up.sql',
        (MIGRATIONS / '1793020005000_board_operations/up.sql').read_text())
    sql(DB, fixture)
    tables = (MIGRATIONS / '1793020003000_self_tests/up.sql').read_text().split('CREATE OR REPLACE VIEW')[0]
    sql(DB, """
CREATE TABLE sequent_backend.tenant(id uuid PRIMARY KEY,is_active boolean);
INSERT INTO sequent_backend.tenant VALUES('11111111-1111-4111-8111-111111111111',true),('22222222-2222-4222-8222-222222222222',true);
INSERT INTO sequent_backend.election_event VALUES('aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','11111111-1111-4111-8111-111111111111','{}'),('bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','22222222-2222-4222-8222-222222222222','{}');
INSERT INTO sequent_backend.election_lifecycle VALUES('11111111-1111-4111-8111-111111111111','aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','preparation',0),('22222222-2222-4222-8222-222222222222','bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','preparation',0);
ALTER TABLE sequent_backend.election_lifecycle ADD COLUMN safety text DEFAULT 'operational';
ALTER TABLE sequent_backend.election_lifecycle ADD UNIQUE(tenant_id,election_event_id);
ALTER TABLE sequent_backend.election_lifecycle ADD FOREIGN KEY(election_event_id) REFERENCES sequent_backend.election_event(id) ON DELETE CASCADE;
CREATE TABLE sequent_backend.lifecycle_service_lease(component text,expires_at timestamptz);
CREATE FUNCTION sequent_backend.audit_storage_allows_voters() RETURNS boolean LANGUAGE sql AS 'SELECT true';
CREATE TABLE sequent_backend.lifecycle_notice(id uuid PRIMARY KEY DEFAULT gen_random_uuid(),tenant_id uuid,election_event_id uuid REFERENCES sequent_backend.election_event(id) ON DELETE CASCADE,kind text,details jsonb,created_at timestamptz DEFAULT clock_timestamp(),logged_at timestamptz);
""" + tables + (MIGRATIONS / '1793020007000_attack_heuristics/up.sql').read_text()
        + (MIGRATIONS / '1793020008000_execution_security_evidence/up.sql').read_text())
    # This run has no corresponding event-log completion: the forced timeout
    # prevented that write. The durable result must survive the complete export.
    sql(DB, """
CREATE FUNCTION public.refresh_receipt() RETURNS void LANGUAGE plpgsql AS $$
DECLARE n bigint; BEGIN
 SELECT revision INTO n FROM sequent_backend.election_execution_revision WHERE election_event_id='33333333-3333-4333-8333-333333333333';
 UPDATE sequent_backend.execution_export SET source_revision=n;
 UPDATE sequent_backend.signing_request SET subject=jsonb_build_object('lifecycle_generation',3,'source_revision',n);
END $$;
SET sequent.trusted_write='on';
SET sequent.authorized_deletion='44444444-4444-4444-8444-444444444444';
INSERT INTO sequent_backend.self_test_run(id,tenant_id,election_event_id,generation,trigger,started_at,finished_at,outcome,result) VALUES
 ('cccccccc-cccc-4ccc-8ccc-cccccccccccc','11111111-1111-4111-8111-111111111111','33333333-3333-4333-8333-333333333333',2,'requested','2026-10-27T11:30:00Z','2026-10-27T11:30:20Z','recoverable-failure','{"run_id":"cccccccc-cccc-4ccc-8ccc-cccccccccccc","generation":2,"trigger":"requested","started_at":"2026-10-27T11:30:00Z","finished_at":"2026-10-27T11:30:20Z","outcome":"recoverable-failure","checks":[{"check":"audit-generation","outcome":"recoverable-failure","detail":"The configured suite deadline was exceeded"}]}'),
 ('dddddddd-dddd-4ddd-8ddd-dddddddddddd','11111111-1111-4111-8111-111111111111','aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa',0,'requested',now(),now(),'passed','{"private-other-event":true}'),
 ('eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee','22222222-2222-4222-8222-222222222222','bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb',0,'requested',now(),now(),'passed','{"private-other-tenant":true}');
INSERT INTO sequent_backend.self_test_settings VALUES('11111111-1111-4111-8111-111111111111','33333333-3333-4333-8333-333333333333','{"interval_minutes":1,"timeout_seconds":20,"additional_conditions":[]}');
INSERT INTO sequent_backend.self_test_baseline(tenant_id,election_event_id,publication_id,snapshot,signature,digest) VALUES('11111111-1111-4111-8111-111111111111','33333333-3333-4333-8333-333333333333','ffffffff-ffff-4fff-8fff-ffffffffffff','{"approved":"digest-only"}','{"signature":"public-signed-proof"}',repeat('a',64));
SELECT public.refresh_receipt();
SELECT public.assert_deletion(true);
""")
    correction = MIGRATIONS / '1793020009000_self_test_export_evidence'
    if not args.without_correction:
        sql(DB, correction.joinpath('up.sql').read_text())
    sql(DB, """
DO $$ DECLARE evidence jsonb; preserved jsonb; BEGIN
 evidence=sequent_backend.execution_security_evidence('11111111-1111-4111-8111-111111111111','33333333-3333-4333-8333-333333333333');
 SELECT to_jsonb(r) INTO preserved FROM sequent_backend.self_test_run r WHERE id='cccccccc-cccc-4ccc-8ccc-cccccccccccc';
 IF evidence->'self_tests' IS DISTINCT FROM jsonb_build_array(preserved) THEN RAISE EXCEPTION 'Full timeout result, start/stop and scope missing from signed export snapshot'; END IF;
 IF evidence->'self_test_settings' IS DISTINCT FROM (SELECT to_jsonb(s) FROM sequent_backend.self_test_settings s) OR evidence->'self_test_baseline' IS DISTINCT FROM (SELECT to_jsonb(b) FROM sequent_backend.self_test_baseline b) THEN RAISE EXCEPTION 'Approved baseline/settings missing'; END IF;
 IF evidence::text LIKE '%private-other-%' THEN RAISE EXCEPTION 'Cross-scope test data leaked'; END IF;
 IF sequent_backend.execution_security_evidence('22222222-2222-4222-8222-222222222222','33333333-3333-4333-8333-333333333333') IS NOT NULL THEN RAISE EXCEPTION 'Cross-tenant scope accepted'; END IF;
END $$;
SET sequent.trusted_write='on';
SET sequent.authorized_deletion='44444444-4444-4444-8444-444444444444';
SELECT public.assert_deletion(false); -- The migration invalidates pre-correction receipts.
SELECT public.refresh_receipt();
SELECT public.assert_deletion(true);
UPDATE sequent_backend.self_test_run SET result=result WHERE id='cccccccc-cccc-4ccc-8ccc-cccccccccccc';
SELECT public.assert_deletion(true);
UPDATE sequent_backend.self_test_run SET result=result||'{"late_detail":"retained"}' WHERE id='cccccccc-cccc-4ccc-8ccc-cccccccccccc';
SELECT public.assert_deletion(false);
SELECT public.refresh_receipt();
UPDATE sequent_backend.self_test_settings SET settings=settings||'{"timeout_seconds":19}';
SELECT public.assert_deletion(false);
SELECT public.refresh_receipt();
UPDATE sequent_backend.self_test_baseline SET digest=repeat('b',64);
SELECT public.assert_deletion(false);
SELECT public.refresh_receipt();
UPDATE sequent_backend.self_test_run SET result=result||'{"other":"changed"}' WHERE id='dddddddd-dddd-4ddd-8ddd-dddddddddddd';
SELECT public.assert_deletion(true);
INSERT INTO sequent_backend.self_test_run(id,tenant_id,election_event_id,generation,trigger,result) VALUES('99999999-9999-4999-8999-999999999999','11111111-1111-4111-8111-111111111111','33333333-3333-4333-8333-333333333333',3,'requested','{"outcome":"pending"}');
SELECT public.assert_deletion(false);
SELECT public.refresh_receipt();
UPDATE sequent_backend.self_test_run SET finished_at=clock_timestamp(),outcome='passed',result='{"outcome":"passed"}' WHERE id='99999999-9999-4999-8999-999999999999';
SELECT public.assert_deletion(false);
SELECT public.refresh_receipt();
SELECT public.assert_deletion(true);
""")
    holder = subprocess.Popen(['sudo','docker','exec','-i','agd-st-postgres','psql','-X','-qAt','-U','postgres','-v','ON_ERROR_STOP=1','-d',DB],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
    try:
        holder.stdin.write("BEGIN; SELECT id FROM sequent_backend.election_event WHERE id='33333333-3333-4333-8333-333333333333' FOR UPDATE;\n");holder.stdin.flush()
        assert select.select([holder.stdout],[],[],5)[0]
        assert holder.stdout.readline().strip()=='33333333-3333-4333-8333-333333333333'
        try:
            sql(DB,"SET lock_timeout='150ms'; UPDATE sequent_backend.self_test_run SET result=result||'{\"concurrent\":true}' WHERE id='cccccccc-cccc-4ccc-8ccc-cccccccccccc';")
            raise AssertionError('Concurrent result bypassed protected deletion parent lock')
        except RuntimeError as error:
            assert 'lock timeout' in str(error),str(error)
        holder.communicate('ROLLBACK;\n',timeout=5);assert holder.returncode==0
    finally:
        if holder.poll() is None:holder.kill();holder.communicate(timeout=5)
    sql(DB,"SET sequent.trusted_write='on'; SET sequent.authorized_deletion='44444444-4444-4444-8444-444444444444'; DELETE FROM sequent_backend.election_event WHERE id='33333333-3333-4333-8333-333333333333'; DO $$ BEGIN IF EXISTS(SELECT FROM sequent_backend.self_test_run WHERE election_event_id='33333333-3333-4333-8333-333333333333') THEN RAISE EXCEPTION 'Own tests survived authorized teardown'; END IF; IF (SELECT count(*) FROM sequent_backend.self_test_run)<>2 THEN RAISE EXCEPTION 'Other scopes changed'; END IF; END $$;")
    sql(DB, correction.joinpath('down.sql').read_text())
    print('PASS: actual PostgreSQL full timeout preservation, baseline/settings, exact scope, old/current receipt invalidation, no-op stability, concurrent deletion lock, scoped cascade and rollback')
finally:
    sql('postgres','DROP DATABASE '+DB)
