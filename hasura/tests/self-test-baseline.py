#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Exercise production snapshot SQL against mutations in a disposable PostgreSQL DB.

Run: python3 hasura/tests/self-test-baseline.py [disposable-postgres-container]
The test creates and removes its own uniquely named database.
"""
from pathlib import Path
import re
import subprocess
import sys
import uuid

source = (Path(__file__).resolve().parents[2] / 'packages/windmill/src/services/self_tests/snapshot.rs').read_text()
def query(name):
    return re.search(r'\("'+name+r'",\s*"([^"]+)"\)', source).group(1)
event_query=query('event')
election_query=query('elections')
voters_query=re.search(r'data.insert\("voters".*?\n\s*"([^"]+)"',source,re.S).group(1)
container=sys.argv[1] if len(sys.argv)>1 else 'agd-st-postgres'
database='selftest_baseline_'+uuid.uuid4().hex

def execute(database,sql):
    result=subprocess.run(['sudo','docker','exec','-i',container,'psql','-U','postgres','-v','ON_ERROR_STOP=1','-d',database],input=sql,text=True,capture_output=True,timeout=20)
    if result.returncode:
        raise RuntimeError(result.stderr)
    return result.stdout

execute('postgres','CREATE DATABASE '+database+';')
try:
    sql='''CREATE SCHEMA sequent_backend;
CREATE TABLE sequent_backend.election_event(id uuid,tenant_id uuid,created_at text,updated_at text,last_updated_at text,status jsonb,is_archived boolean,bulletin_board_reference text,statistics jsonb,presentation jsonb,administration_settings jsonb,name text);
CREATE TABLE sequent_backend.election (LIKE sequent_backend.election_event);
ALTER TABLE sequent_backend.election ADD COLUMN election_event_id uuid,ADD COLUMN initialization_report_generated boolean;
CREATE TABLE realm(id text,name text);
CREATE TABLE user_entity(id text,realm_id text,username text,first_name text,last_name text,email text,enabled boolean);
CREATE TABLE user_attribute(id text,user_id text,name text,value text,long_value text);
INSERT INTO realm VALUES('realm','tenant-event');
INSERT INTO user_entity VALUES('voter','realm','alice','Alice','Voter','alice@example.invalid',true);
INSERT INTO user_attribute VALUES('area','voter','area-id','approved-area',NULL);
INSERT INTO sequent_backend.election_event(id,tenant_id,status,is_archived,presentation,administration_settings,name) VALUES('33333333-3333-4333-8333-333333333333','11111111-1111-4111-8111-111111111111','{"is_published":false}',false,'{"locked_down":"not-locked-down","receipts":{"key_id":"approved"},"custom_rule":7}','{"locked_down":"not-locked-down"}','Approved title');
INSERT INTO sequent_backend.election SELECT e.*,e.id,false FROM sequent_backend.election_event e;
DO $test$
DECLARE approved jsonb; observed jsonb; tenant text:='11111111-1111-4111-8111-111111111111'; event text:='33333333-3333-4333-8333-333333333333';
BEGIN
'''+f'''EXECUTE $query${event_query}$query$ INTO approved USING tenant,event;
UPDATE sequent_backend.election_event SET status='{{"is_published":true,"voting_status":"OPEN"}}',updated_at='later',statistics='{{"count":12}}',presentation=jsonb_set(presentation,'{{locked_down}}','"locked-down"'),administration_settings='{{"locked_down":"locked-down"}}';
EXECUTE $query${event_query}$query$ INTO observed USING tenant,event;
IF approved IS DISTINCT FROM observed THEN RAISE EXCEPTION 'Approved publish/start/lockdown changed the immutable snapshot'; END IF;
UPDATE sequent_backend.election_event SET presentation=jsonb_set(presentation,'{{receipts,key_id}}','"substituted"');
EXECUTE $query${event_query}$query$ INTO observed USING tenant,event;
IF approved IS NOT DISTINCT FROM observed THEN RAISE EXCEPTION 'Receipt key substitution escaped the snapshot'; END IF;
EXECUTE $query${election_query}$query$ INTO approved USING tenant,event;
UPDATE sequent_backend.election SET initialization_report_generated=true,status='{{"voting_status":"OPEN"}}',updated_at='later';
EXECUTE $query${election_query}$query$ INTO observed USING tenant,event;
IF approved IS DISTINCT FROM observed THEN RAISE EXCEPTION 'Initialization/opening changed the immutable snapshot'; END IF;
UPDATE sequent_backend.election SET name='Unapproved ballot title';
EXECUTE $query${election_query}$query$ INTO observed USING tenant,event;
IF approved IS NOT DISTINCT FROM observed THEN RAISE EXCEPTION 'Changed election data escaped the snapshot'; END IF;
EXECUTE $query${voters_query}$query$ INTO approved USING 'tenant-event';
INSERT INTO user_attribute VALUES('locale','voter','locale','de',NULL);
EXECUTE $query${voters_query}$query$ INTO observed USING 'tenant-event';
IF approved IS DISTINCT FROM observed THEN RAISE EXCEPTION 'Login locale preference changed the approved voters register'; END IF;
UPDATE user_attribute SET value='other-area' WHERE name='area-id';
EXECUTE $query${voters_query}$query$ INTO observed USING 'tenant-event';
IF approved IS NOT DISTINCT FROM observed THEN RAISE EXCEPTION 'Changed voter authorization escaped the snapshot'; END IF;
END $test$;
'''
    execute(database,sql)
    print('PASS: publish/start/lockdown/initialization/locale stay stable; receipt, election and voter authorization substitutions fail')
finally:
    execute('postgres','DROP DATABASE '+database+';')
