#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Real PostgreSQL: bind stored ceremony policy to the same tenant/event."""
import argparse
from pathlib import Path
import select
import subprocess
import uuid

ROOT = Path(__file__).resolve().parents[2]
MIGRATION = ROOT / 'hasura/migrations/backend-db/1793020010000_manual_ceremony_policy'
DB = 'manual_ceremony_' + uuid.uuid4().hex
parser = argparse.ArgumentParser()
parser.add_argument('--without-correction', action='store_true')
args = parser.parse_args()


def sql(database, query):
    result = subprocess.run(['sudo', 'docker', 'exec', '-i', 'agd-st-postgres',
        'psql', '-X', '-qAt', '-U', 'postgres', '-v', 'ON_ERROR_STOP=1', '-d', database],
        input=query, text=True, capture_output=True, timeout=15)
    if result.returncode:
        raise RuntimeError(result.stderr)
    return result.stdout


sql('postgres', 'CREATE DATABASE ' + DB)
try:
    sql(DB, """
CREATE SCHEMA sequent_backend;
CREATE TABLE sequent_backend.election_event(id uuid PRIMARY KEY, tenant_id uuid NOT NULL, presentation jsonb);
CREATE TABLE sequent_backend.keys_ceremony(id uuid PRIMARY KEY DEFAULT gen_random_uuid(), tenant_id uuid NOT NULL, election_event_id uuid NOT NULL REFERENCES sequent_backend.election_event(id), settings jsonb, execution_status text DEFAULT 'IN_PROGRESS');
CREATE TABLE sequent_backend.tally_session(id uuid PRIMARY KEY DEFAULT gen_random_uuid(), tenant_id uuid NOT NULL, election_event_id uuid NOT NULL REFERENCES sequent_backend.election_event(id), keys_ceremony_id uuid REFERENCES sequent_backend.keys_ceremony(id), execution_status text DEFAULT 'STARTED');
CREATE TABLE public.queued_work(id uuid);
CREATE FUNCTION public.queue_work() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN INSERT INTO public.queued_work VALUES(NEW.id); RETURN NEW; END $$;
CREATE TRIGGER after_keys AFTER INSERT ON sequent_backend.keys_ceremony FOR EACH ROW EXECUTE FUNCTION public.queue_work();
CREATE TRIGGER after_tally AFTER INSERT ON sequent_backend.tally_session FOR EACH ROW EXECUTE FUNCTION public.queue_work();
INSERT INTO sequent_backend.election_event VALUES
 ('aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','11111111-1111-4111-8111-111111111111','{"ceremonies_policy":"manual-ceremonies"}'),
 ('bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','11111111-1111-4111-8111-111111111111','{"ceremonies_policy":"automated-ceremonies"}'),
 ('cccccccc-cccc-4ccc-8ccc-cccccccccccc','22222222-2222-4222-8222-222222222222','{}');
-- Preserve an existing historical mismatch; the migration must not rewrite it.
INSERT INTO sequent_backend.keys_ceremony VALUES('dddddddd-dddd-4ddd-8ddd-dddddddddddd','11111111-1111-4111-8111-111111111111','aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','{"policy":"automated-ceremonies"}');
INSERT INTO sequent_backend.tally_session(tenant_id,election_event_id,keys_ceremony_id) VALUES('11111111-1111-4111-8111-111111111111','aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','dddddddd-dddd-4ddd-8ddd-dddddddddddd');
CREATE FUNCTION public.refuse(query text, expected text) RETURNS void LANGUAGE plpgsql AS $$
DECLARE before_count bigint; after_count bigint;
BEGIN
 SELECT count(*) INTO before_count FROM public.queued_work;
 BEGIN EXECUTE query; RAISE EXCEPTION 'Expected refusal, statement succeeded';
 EXCEPTION WHEN OTHERS THEN IF SQLSTATE <> expected THEN RAISE; END IF; END;
 SELECT count(*) INTO after_count FROM public.queued_work;
 IF before_count<>after_count THEN RAISE EXCEPTION 'Refusal created work'; END IF;
END $$;
""")
    if not args.without_correction:
        sql(DB, (MIGRATION / 'up.sql').read_text())
    sql(DB, """
SELECT public.refuse($q$INSERT INTO sequent_backend.keys_ceremony(tenant_id,election_event_id,settings) VALUES('11111111-1111-4111-8111-111111111111','aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','{"policy":"automated-ceremonies"}')$q$,'23514');
SELECT public.refuse($q$INSERT INTO sequent_backend.keys_ceremony(tenant_id,election_event_id,settings) VALUES('22222222-2222-4222-8222-222222222222','cccccccc-cccc-4ccc-8ccc-cccccccccccc','{"policy":"automated-ceremonies"}')$q$,'23514');
SELECT public.refuse($q$INSERT INTO sequent_backend.keys_ceremony(tenant_id,election_event_id,settings) VALUES('22222222-2222-4222-8222-222222222222','bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','{"policy":"automated-ceremonies"}')$q$,'23503');
SELECT public.refuse($q$INSERT INTO sequent_backend.tally_session(tenant_id,election_event_id,keys_ceremony_id) VALUES('11111111-1111-4111-8111-111111111111','aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','dddddddd-dddd-4ddd-8ddd-dddddddddddd')$q$,'23514');
SELECT public.refuse($q$UPDATE sequent_backend.keys_ceremony SET execution_status='SUCCESS' WHERE id='dddddddd-dddd-4ddd-8ddd-dddddddddddd'$q$,'23514');
SELECT public.refuse($q$UPDATE sequent_backend.tally_session SET execution_status='IN_PROGRESS' WHERE keys_ceremony_id='dddddddd-dddd-4ddd-8ddd-dddddddddddd'$q$,'23514');
INSERT INTO sequent_backend.keys_ceremony VALUES
 ('eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee','11111111-1111-4111-8111-111111111111','aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','{"policy":"manual-ceremonies"}'),
 ('ffffffff-ffff-4fff-8fff-ffffffffffff','11111111-1111-4111-8111-111111111111','bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','{"policy":"automated-ceremonies"}'),
 ('99999999-9999-4999-8999-999999999999','11111111-1111-4111-8111-111111111111','bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','{"policy":"manual-ceremonies"}');
SELECT public.refuse($q$UPDATE sequent_backend.keys_ceremony SET settings='{"policy":"automated-ceremonies"}' WHERE id='eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee'$q$,'23514');
SELECT public.refuse($q$UPDATE sequent_backend.keys_ceremony SET election_event_id='aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa' WHERE id='ffffffff-ffff-4fff-8fff-ffffffffffff'$q$,'23514');
SELECT public.refuse($q$INSERT INTO sequent_backend.tally_session(tenant_id,election_event_id,keys_ceremony_id) VALUES('11111111-1111-4111-8111-111111111111','aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','ffffffff-ffff-4fff-8fff-ffffffffffff')$q$,'23503');
SELECT public.refuse($q$INSERT INTO sequent_backend.tally_session(tenant_id,election_event_id,keys_ceremony_id) VALUES('22222222-2222-4222-8222-222222222222','cccccccc-cccc-4ccc-8ccc-cccccccccccc','eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee')$q$,'23503');
SELECT public.refuse($q$UPDATE sequent_backend.election_event SET presentation='{"ceremonies_policy":"manual-ceremonies"}' WHERE id='bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb'$q$,'23514');
INSERT INTO sequent_backend.tally_session(tenant_id,election_event_id,keys_ceremony_id) VALUES
 ('11111111-1111-4111-8111-111111111111','aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee'),
 ('11111111-1111-4111-8111-111111111111','bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','ffffffff-ffff-4fff-8fff-ffffffffffff'),
 ('11111111-1111-4111-8111-111111111111','bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','99999999-9999-4999-8999-999999999999');
SELECT public.refuse($q$UPDATE sequent_backend.tally_session SET keys_ceremony_id='dddddddd-dddd-4ddd-8ddd-dddddddddddd' WHERE election_event_id='aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa'$q$,'23514');
DO $$ BEGIN
 IF (SELECT count(*) FROM sequent_backend.keys_ceremony)<>4 OR (SELECT count(*) FROM sequent_backend.tally_session)<>4 OR (SELECT count(*) FROM public.queued_work)<>8 THEN RAISE EXCEPTION 'Rejected operation left rows or queued work'; END IF;
 IF EXISTS(SELECT FROM sequent_backend.tally_session WHERE execution_status<>'STARTED') THEN RAISE EXCEPTION 'Legacy mismatched tally was activated'; END IF;
 IF (SELECT settings->>'policy' FROM sequent_backend.keys_ceremony WHERE id='dddddddd-dddd-4ddd-8ddd-dddddddddddd')<>'automated-ceremonies' THEN RAISE EXCEPTION 'Historical evidence rewritten'; END IF;
 IF EXISTS(SELECT FROM sequent_backend.keys_ceremony WHERE execution_status<>'IN_PROGRESS') THEN RAISE EXCEPTION 'Mismatched keys ceremony progressed'; END IF;
END $$;
UPDATE sequent_backend.keys_ceremony SET execution_status='SUCCESS' WHERE id IN ('eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee','ffffffff-ffff-4fff-8fff-ffffffffffff');
UPDATE sequent_backend.tally_session SET execution_status='IN_PROGRESS' WHERE keys_ceremony_id IN ('eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee','ffffffff-ffff-4fff-8fff-ffffffffffff','99999999-9999-4999-8999-999999999999');
DO $$ BEGIN
 IF (SELECT count(*) FROM sequent_backend.keys_ceremony WHERE execution_status='SUCCESS')<>2 OR (SELECT count(*) FROM sequent_backend.tally_session WHERE execution_status='IN_PROGRESS')<>3 THEN RAISE EXCEPTION 'Valid manual/automatic progress refused'; END IF;
END $$;
""")
    # Policy writes cannot race a new ceremony's policy read.
    holder = subprocess.Popen(['sudo', 'docker', 'exec', '-i', 'agd-st-postgres',
        'psql', '-X', '-qAt', '-U', 'postgres', '-v', 'ON_ERROR_STOP=1', '-d', DB],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    try:
        holder.stdin.write("BEGIN; INSERT INTO sequent_backend.keys_ceremony(tenant_id,election_event_id,settings) VALUES('11111111-1111-4111-8111-111111111111','bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','{\"policy\":\"manual-ceremonies\"}'); SELECT 'locked';\n")
        holder.stdin.flush()
        assert select.select([holder.stdout], [], [], 5)[0]
        assert holder.stdout.readline().strip() == 'locked'
        try:
            sql(DB, "SET lock_timeout='150ms'; UPDATE sequent_backend.election_event SET presentation='{}' WHERE id='bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';")
            raise AssertionError('Event policy changed concurrently')
        except RuntimeError as error:
            assert 'lock timeout' in str(error)
        holder.communicate('ROLLBACK;\n', timeout=5)
        assert holder.returncode == 0
    finally:
        if holder.poll() is None:
            holder.kill()
            holder.communicate(timeout=5)
    sql(DB, (MIGRATION / 'down.sql').read_text())
    print('PASS: actual PostgreSQL manual/default refusal, automated/manual choices, legacy mismatch, exact tenant/event scope, unchanged refused rows/work and policy-write serialization')
finally:
    sql('postgres', 'DROP DATABASE ' + DB)
