#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Real PostgreSQL receipt invalidation and event-scoped security snapshot."""
from pathlib import Path
import subprocess
import uuid
import select

ROOT = Path(__file__).resolve().parents[2]
DATABASE = 'security_export_' + uuid.uuid4().hex


def sql(database, command):
    result = subprocess.run(
        ['sudo', 'docker', 'exec', '-i', 'agd-st-postgres', 'psql', '-X',
         '-U', 'postgres', '-v', 'ON_ERROR_STOP=1', '-d', database],
        input=command, text=True, capture_output=True, timeout=20)
    if result.returncode:
        raise RuntimeError(result.stderr)
    return result.stdout


sql('postgres', 'CREATE DATABASE ' + DATABASE)
try:
    fixture = (ROOT / 'hasura/tests/board-operation-export.sql').read_text()
    fixture = fixture.replace(
        r'\ir ../migrations/backend-db/1793020005000_board_operations/up.sql',
        (ROOT / 'hasura/migrations/backend-db/1793020005000_board_operations/up.sql').read_text())
    sql(DATABASE, fixture)
    sql(DATABASE, """
CREATE TABLE sequent_backend.tenant(id uuid PRIMARY KEY,is_active boolean);
INSERT INTO sequent_backend.tenant VALUES('11111111-1111-4111-8111-111111111111',true),('22222222-2222-4222-8222-222222222222',true);
INSERT INTO sequent_backend.election_event VALUES('aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','11111111-1111-4111-8111-111111111111','{}'),('bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','22222222-2222-4222-8222-222222222222','{}');
ALTER TABLE sequent_backend.election_lifecycle ADD COLUMN safety text DEFAULT 'operational';
CREATE TABLE sequent_backend.lifecycle_service_lease(component text,expires_at timestamptz);
CREATE TABLE sequent_backend.self_test_run(id uuid,tenant_id uuid,election_event_id uuid,generation bigint,outcome text,finished_at timestamptz);
CREATE TABLE sequent_backend.self_test_settings(tenant_id uuid,election_event_id uuid,settings jsonb);
CREATE FUNCTION sequent_backend.audit_storage_allows_voters() RETURNS boolean LANGUAGE sql AS 'SELECT true';
CREATE TABLE sequent_backend.lifecycle_notice(id uuid PRIMARY KEY DEFAULT gen_random_uuid(),tenant_id uuid,election_event_id uuid REFERENCES sequent_backend.election_event(id) ON DELETE CASCADE,kind text,details jsonb,created_at timestamptz DEFAULT clock_timestamp(),logged_at timestamptz);
""" + (ROOT / 'hasura/migrations/backend-db/1793020007000_attack_heuristics/up.sql').read_text())
    migration = ROOT / 'hasura/migrations/backend-db/1793020008000_execution_security_evidence/up.sql'
    sql(DATABASE, migration.read_text())
    sql(DATABASE, """
CREATE FUNCTION public.refresh_receipt() RETURNS void LANGUAGE plpgsql AS $$
DECLARE n bigint;
BEGIN
 SELECT revision INTO n FROM sequent_backend.election_execution_revision WHERE election_event_id='33333333-3333-4333-8333-333333333333';
 UPDATE sequent_backend.execution_export SET source_revision=n;
 UPDATE sequent_backend.signing_request SET subject=jsonb_build_object('lifecycle_generation',3,'source_revision',n);
END $$;
SET sequent.trusted_write='on';
SET sequent.authorized_deletion='44444444-4444-4444-8444-444444444444';
SELECT public.refresh_receipt();
SELECT public.assert_deletion(true);
INSERT INTO sequent_backend.lifecycle_notice(tenant_id,election_event_id,kind,details) VALUES('11111111-1111-4111-8111-111111111111','33333333-3333-4333-8333-333333333333','PotentialAttackDetected','{"pending":true}');
SELECT public.assert_deletion(false); -- A later security notice invalidates an already authorized receipt.
SELECT public.refresh_receipt();
UPDATE sequent_backend.lifecycle_notice SET logged_at=clock_timestamp();
SELECT public.assert_deletion(true); -- Delivery bookkeeping is not new security evidence.
INSERT INTO sequent_backend.lifecycle_notice(tenant_id,election_event_id,kind,details) VALUES('11111111-1111-4111-8111-111111111111','33333333-3333-4333-8333-333333333333','ExecutionDataExported','{}');
SELECT public.assert_deletion(true); -- The export's own receipt does not invalidate itself.
INSERT INTO sequent_backend.attack_detection_settings(tenant_id,updated_by) VALUES('11111111-1111-4111-8111-111111111111','admin');
SELECT public.assert_deletion(false);
SELECT public.refresh_receipt();
INSERT INTO sequent_backend.attack_auth_activity(id,tenant_id,election_event_id,realm,account_sha256,occurred_at,failure_limit,window_seconds) VALUES(gen_random_uuid(),'11111111-1111-4111-8111-111111111111','33333333-3333-4333-8333-333333333333','event','own-hash',clock_timestamp(),3,60);
SELECT public.assert_deletion(false);
SELECT public.refresh_receipt();
INSERT INTO sequent_backend.attack_auth_activity(id,tenant_id,election_event_id,realm,account_sha256,occurred_at,failure_limit,window_seconds) VALUES(gen_random_uuid(),'11111111-1111-4111-8111-111111111111',NULL,'staff','DO-NOT-EXPORT',clock_timestamp(),3,60),(gen_random_uuid(),'11111111-1111-4111-8111-111111111111','aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','other-event','DO-NOT-EXPORT',clock_timestamp(),3,60),(gen_random_uuid(),'22222222-2222-4222-8222-222222222222','bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','other-tenant','DO-NOT-EXPORT',clock_timestamp(),3,60);
SELECT public.assert_deletion(true); -- Another scope's raw authentication is irrelevant.
INSERT INTO sequent_backend.attack_detection(tenant_id,election_event_id,category,scope_key,window_start,window_end,observed_count,threshold,details) VALUES('11111111-1111-4111-8111-111111111111','33333333-3333-4333-8333-333333333333','brute-force-account','own',now(),now(),3,3,'{}');
SELECT public.assert_deletion(false);
SELECT public.refresh_receipt();
UPDATE sequent_backend.attack_detection SET observed_count=4 WHERE scope_key='own';
SELECT public.assert_deletion(false); -- Updated detection counts also need a new archive.
SELECT public.refresh_receipt();
UPDATE sequent_backend.attack_detection SET observed_count=observed_count WHERE scope_key='own';
SELECT public.assert_deletion(true); -- Reconciliation without a change does not invalidate.
INSERT INTO sequent_backend.attack_detection(tenant_id,category,scope_key,window_start,window_end,observed_count,threshold,details) VALUES('11111111-1111-4111-8111-111111111111','network-dos','cluster',now(),now(),5,3,'{}');
SELECT public.assert_deletion(false); -- A shared detection applies to every current tenant event.
INSERT INTO sequent_backend.attack_detection_audit SELECT id,'33333333-3333-4333-8333-333333333333',gen_random_uuid() FROM sequent_backend.attack_detection WHERE scope_key='own';
INSERT INTO sequent_backend.lifecycle_notice(tenant_id,election_event_id,kind,details) VALUES('11111111-1111-4111-8111-111111111111','33333333-3333-4333-8333-333333333333','AttackDetectionSettingsChanged','{"still_pending":true}');
DO $$ DECLARE evidence jsonb; BEGIN
 evidence=sequent_backend.execution_security_evidence('11111111-1111-4111-8111-111111111111','33333333-3333-4333-8333-333333333333');
 IF evidence::text LIKE '%DO-NOT-EXPORT%' OR evidence ? 'attack_request_bucket' THEN RAISE EXCEPTION 'Export crossed scope'; END IF;
 IF jsonb_array_length(evidence->'authentication_activity')<>1 OR jsonb_array_length(evidence->'detections')<>2 THEN RAISE EXCEPTION 'Missing own scoped execution records'; END IF;
 IF NOT EXISTS(SELECT FROM jsonb_array_elements(evidence->'notices') n WHERE n->>'kind'='AttackDetectionSettingsChanged' AND n->'logged_at'='null'::jsonb) THEN RAISE EXCEPTION 'Pending notice omitted'; END IF;
 IF EXISTS(SELECT FROM jsonb_array_elements(evidence->'notices') n WHERE n->>'kind'='ExecutionDataExported') THEN RAISE EXCEPTION 'Export completion receipt should remain separate'; END IF;
 IF sequent_backend.execution_security_evidence('22222222-2222-4222-8222-222222222222','33333333-3333-4333-8333-333333333333') IS NOT NULL THEN RAISE EXCEPTION 'Cross-tenant event selected'; END IF;
END $$;
SELECT public.refresh_receipt();
SELECT public.assert_deletion(true);
DELETE FROM sequent_backend.election_event WHERE id='33333333-3333-4333-8333-333333333333';
DO $$ BEGIN
 IF EXISTS(SELECT FROM sequent_backend.attack_auth_activity WHERE election_event_id='33333333-3333-4333-8333-333333333333') OR EXISTS(SELECT FROM sequent_backend.attack_detection WHERE election_event_id='33333333-3333-4333-8333-333333333333') THEN RAISE EXCEPTION 'Scoped execution records survived deletion'; END IF;
 IF (SELECT count(*) FROM sequent_backend.attack_auth_activity)<>3 THEN RAISE EXCEPTION 'Unrelated records deleted'; END IF;
 IF (SELECT count(*) FROM sequent_backend.attack_detection WHERE election_event_id IS NULL)<>1 THEN RAISE EXCEPTION 'Shared detection deleted'; END IF;
END $$;
""")
    # The deletion worker locks the event before checking the exported revision.
    # A concurrent tenant-wide detection must block on that same parent lock.
    holder = subprocess.Popen(
        ['sudo', 'docker', 'exec', '-i', 'agd-st-postgres', 'psql', '-X', '-qAt',
         '-U', 'postgres', '-v', 'ON_ERROR_STOP=1', '-d', DATABASE],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    try:
        holder.stdin.write("BEGIN; SELECT id FROM sequent_backend.election_event WHERE id='aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa' FOR UPDATE;\n")
        holder.stdin.flush()
        assert select.select([holder.stdout], [], [], 5)[0], 'Parent lock acquisition timed out'
        assert holder.stdout.readline().strip() == 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa'
        try:
            sql(DATABASE, "SET lock_timeout='150ms'; UPDATE sequent_backend.attack_detection SET observed_count=6 WHERE scope_key='cluster';")
            raise AssertionError('Concurrent detection bypassed the deletion worker parent lock')
        except RuntimeError as error:
            assert 'lock timeout' in str(error), str(error)
        holder.communicate('ROLLBACK;\n', timeout=5)
        assert holder.returncode == 0
    finally:
        if holder.poll() is None:
            holder.kill()
            holder.communicate(timeout=5)
    sql(DATABASE, (ROOT / 'hasura/migrations/backend-db/1793020008000_execution_security_evidence/down.sql').read_text())
    print('PASS: actual PostgreSQL receipt invalidation, pending-notice snapshot, no self-invalidation, exact tenant/event scope, late detection updates, scoped teardown and concurrent deletion serialization')
finally:
    sql('postgres', 'DROP DATABASE ' + DATABASE)
