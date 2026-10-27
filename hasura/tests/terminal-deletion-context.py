#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Exercise the worker's terminal-state query with real deletion authority rows."""
import re
import subprocess
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
database = 'terminal_deletion_' + uuid.uuid4().hex
source = (ROOT / 'packages/windmill/src/services/delete_election_event.rs').read_text()
queries = re.findall(r'"(SELECT EXISTS\(SELECT 1 FROM sequent_backend\.election_lifecycle_access[^"\n]+)"', source)
assert len(queries) == 1
tenant = '11111111-1111-4111-8111-111111111111'
event = '33333333-3333-4333-8333-333333333333'
query = queries[0].replace('$1', "'" + tenant + "'").replace('$2', "'" + event + "'")
fixture = (ROOT / 'hasura/tests/board-operation-export.sql').read_text()
migration = (ROOT / 'hasura/migrations/backend-db/1793020005000_board_operations/up.sql').read_text()
fixture = fixture.replace(r'\ir ../migrations/backend-db/1793020005000_board_operations/up.sql', migration)
fixture = fixture.replace('CREATE FUNCTION public.assert_deletion', """
ALTER TABLE sequent_backend.election_lifecycle ADD COLUMN voter_allowed boolean NOT NULL DEFAULT false;
CREATE VIEW sequent_backend.election_lifecycle_access AS SELECT * FROM sequent_backend.election_lifecycle;
CREATE FUNCTION public.assert_deletion""")
old = "sequent_backend.authorized_execution_deletion('" + tenant + "','" + event + "') IS DISTINCT FROM expected"
assert fixture.count(old) == 1
fixture = fixture.replace(old, '(' + old.removesuffix(' IS DISTINCT FROM expected') + ' AND (' + query + ')) IS DISTINCT FROM expected')
fixture += """
-- Valid quorum/export is insufficient if admission or phase is inconsistent.
UPDATE sequent_backend.election_lifecycle SET voter_allowed=true;
SELECT public.assert_deletion(false);
UPDATE sequent_backend.election_lifecycle SET voter_allowed=false;
SELECT public.assert_deletion(true);
UPDATE sequent_backend.election_lifecycle SET phase='execution';
SELECT public.assert_deletion(false);
UPDATE sequent_backend.election_lifecycle SET phase='post_processing';
SELECT public.assert_deletion(true);
SET sequent.authorized_deletion='99999999-9999-4999-8999-999999999999';
SELECT public.assert_deletion(false);
SELECT 'PASS: exact production terminal-state query plus current transaction-bound export/quorum authority';
"""


def sql(db, text):
    result = subprocess.run(['sudo', 'docker', 'exec', '-i', 'agd-st-postgres',
        'psql', '-X', '-qAt', '-U', 'postgres', '-v', 'ON_ERROR_STOP=1', '-d', db],
        input=text, text=True, capture_output=True, timeout=30)
    if result.returncode:
        raise RuntimeError(result.stderr)
    return result.stdout


sql('postgres', 'CREATE DATABASE ' + database)
try:
    print(sql(database, fixture).strip())
finally:
    sql('postgres', 'DROP DATABASE ' + database + ' WITH (FORCE)')
