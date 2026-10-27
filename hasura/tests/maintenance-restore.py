# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Verify actual restore SQL against the disposable lifecycle fixture.

Run after election-lifecycle.sql. Each check rolls back; no VM data is changed.
Uses the production trusted-write function, replacing the fixture stub only
inside the rolled-back transaction.
"""
import json
from pathlib import Path
import subprocess
import sys
from unittest.mock import patch

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / 'docs/docusaurus/scripts/deployment'))
import maintenance

PSQL = ['sudo', 'docker', 'exec', '-i', 'agd-st-postgres', 'psql', '-X', '-qAt',
        '-U', 'postgres', '-d', 'lifecycle_final_test', '-v', 'ON_ERROR_STOP=1']
rows = json.loads(subprocess.check_output(PSQL + ['-c', maintenance.LIFECYCLE_SQL], text=True))
assert len(rows) >= 2
for row in rows:
    row['generation'] += 10
migration = (REPO / 'hasura/migrations/backend-db/1791000000800_guard_trusted_status_and_lockdown/up.sql').read_text()
trusted = migration[migration.index('CREATE OR REPLACE FUNCTION "sequent_backend"."trusted_write_allowed"()'):].split('$$;', 1)[0] + '$$;\n'


def execute(data, node, script, label, timeout):
    sql = script.split("<<'AGD_SQL'\n", 1)[1].rsplit('AGD_SQL\n', 1)[0]
    sql = sql.replace('BEGIN;', 'BEGIN;\n' + trusted, 1)
    verification = '''DO $$ BEGIN
      IF EXISTS(SELECT 1 FROM sequent_backend.election_lifecycle WHERE generation < 12)
        THEN RAISE EXCEPTION 'Generation rollback'; END IF;
      IF EXISTS(SELECT 1 FROM sequent_backend.lifecycle_service_lease)
        THEN RAISE EXCEPTION 'Lease survived restore'; END IF;
      IF (SELECT count(*) FROM sequent_backend.lifecycle_notice WHERE kind='SystemBackupRestored') < 2
        THEN RAISE EXCEPTION 'Missing restore notices'; END IF;
      IF NOT EXISTS(SELECT 1 FROM sequent_backend.election_lifecycle WHERE safety='unrecoverable' AND phase='post_processing')
        THEN RAISE EXCEPTION 'Unrecoverable state or phase cleared'; END IF;
      IF EXISTS(SELECT 1 FROM sequent_backend.election_lifecycle WHERE safety='operational')
        THEN RAISE EXCEPTION 'Operational after restore'; END IF;
    END $$;
    ROLLBACK;
    '''
    sql = sql.replace('COMMIT;\n', verification)
    result = subprocess.run(PSQL, input=sql, text=True, capture_output=True, timeout=5)
    assert result.returncode == 0, result.stderr
    negative = subprocess.run(PSQL, input=sql.replace("SET LOCAL sequent.trusted_write='on';", ''), text=True, capture_output=True, timeout=5)
    assert negative.returncode != 0 and 'require the server workflow' in negative.stderr
    return result.stdout.encode()


with patch('maintenance.install.ssh', side_effect=execute):
    maintenance.invalidate_restored_state({}, rows)
print('PASS: actual PostgreSQL restore advances generation, clears admissions, preserves phases/unrecoverable state, queues notices, rejects untrusted mutation; fixture rolled back')
