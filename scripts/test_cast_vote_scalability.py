#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Exercise the actual migration on an isolated, disposable PostgreSQL cluster.

Run from the repository root inside devenv: python3 scripts/test_cast_vote_scalability.py
No production credentials or existing databases are used.
"""
import base64
import concurrent.futures
import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading
import uuid

ROOT = Path(__file__).resolve().parents[1]
MIGRATION = ROOT / "hasura/migrations/backend-db/1788765000000_serialize_cast_vote_area_checks"


def main():
    with tempfile.TemporaryDirectory(prefix="cast-vote-test-") as directory:
        env = dict(os.environ, PGHOST=directory, PGPORT="55432", PGDATABASE="postgres")
        data = str(Path(directory) / "data")
        subprocess.run(["initdb", "-D", data, "-A", "trust", "--no-locale"], check=True, stdout=subprocess.DEVNULL)
        subprocess.run(["pg_ctl", "-D", data, "-l", str(Path(directory) / "postgres.log"), "-o", f"-k {directory} -p 55432 -c listen_addresses=''", "-w", "start"], check=True, stdout=subprocess.DEVNULL)
        try:
            def sql(statement, check=True):
                return subprocess.run(["psql", "-X", "-qAt", "-v", "ON_ERROR_STOP=1", "-c", statement], env=env, text=True, capture_output=True, check=check)

            tenant, event, election, area_a, area_b = [str(uuid.uuid4()) for _ in range(5)]
            sql("""CREATE SCHEMA sequent_backend;
                CREATE TABLE sequent_backend.election (
                    id uuid PRIMARY KEY, tenant_id uuid, election_event_id uuid, num_allowed_revotes integer);
                CREATE TABLE sequent_backend.cast_vote (
                    id uuid PRIMARY KEY DEFAULT gen_random_uuid(), tenant_id uuid, election_event_id uuid,
                    election_id uuid, voter_id_string text, area_id uuid, status text, content text);
            """)
            sql(f"INSERT INTO sequent_backend.election VALUES ('{election}', '{tenant}', '{event}', 3)")
            sql((MIGRATION / "up.sql").read_text())
            sql("CREATE TRIGGER check_revote_limit_trigger BEFORE INSERT ON sequent_backend.cast_vote FOR EACH ROW EXECUTE FUNCTION check_revote_limit()")

            def insert(voter, area, status="valid"):
                return f"INSERT INTO sequent_backend.cast_vote (tenant_id,election_event_id,election_id,voter_id_string,area_id,status) VALUES ('{tenant}','{event}','{election}','{voter}','{area}','{status}')"

            def concurrent_votes(voter, areas):
                barrier = threading.Barrier(len(areas))
                def vote(area):
                    barrier.wait(timeout=20)
                    return sql("BEGIN; " + insert(voter, area, "in-progress") + "; SELECT pg_sleep(0.03); COMMIT", check=False)
                with concurrent.futures.ThreadPoolExecutor(max_workers=len(areas)) as pool:
                    return list(pool.map(vote, areas))

            results = concurrent_votes("limited", [area_a] * 12)
            assert sum(result.returncode == 0 for result in results) == 3
            assert all("insert_failed_exceeds_allowed_revotes" in result.stderr for result in results if result.returncode)
            print("PASS: 12 concurrent submissions, exactly 3 accepted; in-progress consumes slots")

            sql("UPDATE sequent_backend.election SET num_allowed_revotes=0")
            results = concurrent_votes("areas", [area_a, area_b] * 6)
            assert sum(result.returncode == 0 for result in results) == 6
            assert all("check_votes_in_other_areas_failed" in result.stderr for result in results if result.returncode)
            assert sql("SELECT count(DISTINCT area_id) FROM sequent_backend.cast_vote WHERE voter_id_string='areas'").stdout.strip() == "1"
            print("PASS: unlimited revotes still serialize cross-area exclusivity")

            sql(insert("discarded", area_b, "discarded"))
            sql(insert("discarded", area_a))
            print("PASS: discarded ballots do not block another area")

            # Verify migration rollback and reapplication, without touching old migrations.
            sql((MIGRATION / "down.sql").read_text())
            sql((MIGRATION / "up.sql").read_text())
            print("PASS: migration rollback and reapplication")

            if os.environ.get("CAST_VOTE_RUN_RUST_TESTS") == "1":
                rust_env = dict(env, CAST_VOTE_TEST_DATABASE_URL=f"host={directory} port=55432 dbname=postgres",
                    CARGO_TARGET_DIR=str(ROOT / "packages/windmill/rust-local-target"))
                subprocess.run(["cargo", "test", "-p", "windmill", "--lib", "services::insert_cast_vote::tests::database_trigger_error_preserves_public_error_and_retry_contract", "--", "--ignored"], cwd=ROOT / "packages", env=rust_env, check=True)

            # Encoded ciphertext is not raw random bytes. Measure PostgreSQL's
            # actual storage decision before recommending EXTERNAL globally.
            sql("CREATE TABLE storage_probe (extended text, external text)")
            sql("ALTER TABLE storage_probe ALTER COLUMN external SET STORAGE EXTERNAL")
            ballot = json.dumps({"ciphertext": base64.b64encode(os.urandom(16000)).decode()})
            sql(f"INSERT INTO storage_probe VALUES ('{ballot}', '{ballot}')")
            print("Encoded-ciphertext bytes (EXTENDED|EXTERNAL): " + sql("SELECT pg_column_size(extended), pg_column_size(external) FROM storage_probe").stdout.strip())

            storage_migration = ROOT / "hasura/migrations/backend-db/1788765000001_cast_vote_external_storage"
            sql((storage_migration / "up.sql").read_text())
            assert sql("SELECT attstorage FROM pg_attribute WHERE attrelid='sequent_backend.cast_vote'::regclass AND attname='content'").stdout.strip() == "e"
            sql((storage_migration / "down.sql").read_text())
            assert sql("SELECT attstorage FROM pg_attribute WHERE attrelid='sequent_backend.cast_vote'::regclass AND attname='content'").stdout.strip() == "x"
            sql((storage_migration / "up.sql").read_text())
            print("PASS: EXTERNAL storage migration and rollback")

            # Measure the proposed covering index separately. Recent inserts need
            # heap visibility checks even when every filter column is in the index.
            sql("CREATE INDEX cast_vote_participation_election_idx ON sequent_backend.cast_vote (tenant_id,election_event_id,election_id,voter_id_string)")
            subprocess.run(["psql", "-X", "-v", "ON_ERROR_STOP=1", "-f", str(ROOT / "scripts/postgres/cast_vote_covering_index.sql")], env=env, check=True, stdout=subprocess.DEVNULL)
            assert sql("SELECT count(*) FROM pg_index WHERE indrelid='sequent_backend.cast_vote'::regclass AND indisvalid").stdout.strip() == "2"
            print("PASS: concurrent covering-index replacement retains exactly PK + participation index")
            sql("INSERT INTO sequent_backend.election SELECT gen_random_uuid(), gen_random_uuid(), gen_random_uuid(), 0 FROM generate_series(1,10000)")
            sql("ALTER TABLE sequent_backend.cast_vote DISABLE TRIGGER check_revote_limit_trigger")
            sql(f"INSERT INTO sequent_backend.cast_vote (tenant_id,election_event_id,election_id,voter_id_string,area_id,status) SELECT '{tenant}','{event}','{election}','bulk-' || n,'{area_a}','valid' FROM generate_series(1,100000) n")
            sql("ANALYZE sequent_backend.cast_vote")
            query = f"EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) SELECT count(*) FROM sequent_backend.cast_vote WHERE tenant_id='{tenant}' AND election_event_id='{event}' AND election_id='{election}' AND voter_id_string='limited' AND status IN ('valid','in-progress')"
            before = json.loads(sql(query).stdout)[0]["Plan"]
            sql("VACUUM ANALYZE sequent_backend.cast_vote")
            after = json.loads(sql(query).stdout)[0]["Plan"]
            print("Covering-index plans (before and after VACUUM):")
            print(json.dumps({"before": before, "after": after}, indent=2))
            assert after["Plans"][0]["Node Type"] == "Index Only Scan"
            assert after["Plans"][0]["Heap Fetches"] == 0
        finally:
            subprocess.run(["pg_ctl", "-D", data, "-m", "immediate", "-w", "stop"], check=True, stdout=subprocess.DEVNULL)


if __name__ == "__main__":
    main()
