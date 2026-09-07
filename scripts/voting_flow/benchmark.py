# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Compare the original and revised SQL paths with warm reusable connections.

The baseline follows e93ca05104: eight pre-insert reads, two trigger reads,
one Keycloak read, and one post-commit signing-key read. This is a SQL-path
benchmark, not an HTTP/cryptography/broker benchmark. PostgreSQL statement
statistics verify the read/write/transaction counts rather than trusting labels.
"""

from concurrent.futures import ThreadPoolExecutor
from contextlib import contextmanager
import base64
import json
import math
import os
from queue import Queue
import re
import time

import psycopg
from psycopg.types.json import Jsonb

from database import AREA_MIGRATION, CONFIGURATION_QUERY, ROOT
from fixtures import Election, INSERT_VOTE

PHASES = (("opening", 8, 128), ("lull", 2, 64), ("closing", 8, 128))
WARMUP_REQUESTS = 16
READ_SECRET = """
    SELECT value FROM sequent_backend.secret
    WHERE tenant_id = %s AND election_event_id = %s AND key = 'protocol-manager'
"""
ELIGIBLE_VOTES = """
    FROM sequent_backend.cast_vote
    WHERE tenant_id = %s AND election_event_id = %s AND election_id = %s
      AND voter_id_string = %s AND status IN ('valid', 'in-progress')
"""


class VoterClient:
    """Each worker reuses its connections, as application pools already do."""

    def __init__(self, database, fixture, variant, content):
        self.fixture = fixture
        self.variant = variant
        self.content = content
        self.checkouts = 0
        self.writer = psycopg.connect(
            database.dsn, autocommit=True, prepare_threshold=None
        )
        self.identity = None
        if variant == "before":
            self.identity = psycopg.connect(
                database.dsn,
                dbname="keycloak",
                autocommit=True,
                prepare_threshold=None,
            )

    def close(self):
        self.writer.close()
        if self.identity:
            self.identity.close()

    @contextmanager
    def checkout(self, connection, read_only=False):
        self.checkouts += 1
        # The original identity/audit transactions were dropped without COMMIT.
        with connection.transaction(force_rollback=read_only):
            yield connection

    def cast(self, request_id):
        voter = f"voter-{request_id}"
        fixture = self.fixture
        with self.checkout(self.writer) as connection:
            connection.execute(
                "SELECT * FROM sequent_backend.area WHERE tenant_id = %s AND id = %s",
                (fixture.tenant, fixture.area),
            ).fetchone()
            connection.execute(
                "SELECT * FROM sequent_backend.election_event WHERE tenant_id = %s AND id = %s",
                (fixture.tenant, fixture.event),
            ).fetchone()
            connection.execute(READ_SECRET, (fixture.tenant, fixture.event)).fetchone()

            if self.variant == "before":
                self.read_original_configuration(connection, voter)
                returning = " RETURNING *"
            else:
                connection.execute(CONFIGURATION_QUERY, fixture.scope).fetchone()
                returning = " RETURNING id, ballot_id, election_id, election_event_id, tenant_id, area_id, created_at, last_updated_at, cast_ballot_signature, voter_id_string, status"

            connection.execute(
                INSERT_VOTE + returning,
                fixture.vote_parameters(voter, content=self.content),
            ).fetchone()

        if self.variant == "before":
            with self.checkout(self.identity, read_only=True) as connection:
                connection.execute(
                    "SELECT username FROM user_entity WHERE id = %s",
                    (voter,),
                ).fetchone()
            with self.checkout(self.writer, read_only=True) as connection:
                connection.execute(
                    READ_SECRET, (fixture.tenant, fixture.event)
                ).fetchone()

    def read_original_configuration(self, connection, voter):
        fixture = self.fixture
        # new_from_sk reloads the same key just read by get_electoral_log.
        connection.execute(READ_SECRET, (fixture.tenant, fixture.event)).fetchone()
        connection.execute(
            """
            SELECT * FROM sequent_backend.election
            WHERE tenant_id = %s AND election_event_id = %s AND id = %s
        """,
            fixture.scope,
        ).fetchone()
        connection.execute(
            """
            SELECT * FROM sequent_backend.scheduled_event
            WHERE tenant_id = %s AND election_event_id = %s AND archived_at IS NULL
        """,
            (fixture.tenant, fixture.event),
        ).fetchall()
        connection.execute(
            """
            SELECT id, num_allowed_revotes FROM sequent_backend.election
            WHERE tenant_id = %s AND election_event_id = %s AND id = %s
        """,
            fixture.scope,
        ).fetchone()
        connection.execute(
            "SELECT * " + ELIGIBLE_VOTES, (*fixture.scope, voter)
        ).fetchall()


def seed(database, schedule_count):
    connection = database.connection
    fixture = Election()
    fixture.create(connection, limit=100)
    connection.execute(
        "INSERT INTO sequent_backend.area VALUES (%s, %s, %s, %s)",
        (
            fixture.area,
            fixture.tenant,
            fixture.event,
            Jsonb({}),
        ),
    )
    connection.execute(
        "INSERT INTO sequent_backend.election_event VALUES (%s, %s, %s, 'board')",
        (
            fixture.event,
            fixture.tenant,
            Jsonb({}),
        ),
    )
    connection.execute(
        "INSERT INTO sequent_backend.secret VALUES (%s, %s, 'protocol-manager', %s)",
        (
            fixture.tenant,
            fixture.event,
            "encrypted-signing-key" * 64,
        ),
    )
    connection.execute(
        "UPDATE sequent_backend.election SET eml = %s WHERE id = %s",
        (
            "election-configuration" * 1600,
            fixture.election,
        ),
    )
    fixture.schedule(connection, "START", "2026-10-01T10:00:00Z")
    fixture.schedule(connection, "END", "2026-10-01T12:00:00Z")
    connection.execute(
        """
        INSERT INTO sequent_backend.scheduled_event
            (tenant_id, election_event_id, task_id, event_payload, cron_config)
        SELECT %s, %s, 'unrelated-' || number, '{}'::jsonb, '{}'::jsonb
        FROM generate_series(1, %s) AS number
    """,
        (fixture.tenant, fixture.event, schedule_count),
    )
    return fixture


def prepare_votes(database, fixture, content):
    connection = database.connection
    connection.execute("TRUNCATE sequent_backend.cast_vote")
    connection.execute(
        "ALTER TABLE sequent_backend.cast_vote DISABLE TRIGGER check_revote_limit_trigger"
    )
    try:
        # 50,000 voters with two previous ballots each makes the eligibility
        # index useful. The identical dataset is restored before each variant.
        connection.execute(
            """
            INSERT INTO sequent_backend.cast_vote
                (tenant_id, election_event_id, election_id, area_id,
                 voter_id_string, status, content)
            SELECT %s, %s, %s, %s, 'voter-' || (number / 2), 'valid', %s
            FROM generate_series(0, 99999) AS number
        """,
            (*fixture.scope, fixture.area, content),
        )
    finally:
        connection.execute(
            "ALTER TABLE sequent_backend.cast_vote ENABLE TRIGGER check_revote_limit_trigger"
        )
    connection.execute("VACUUM ANALYZE sequent_backend.cast_vote")
    connection.execute("ANALYZE sequent_backend.scheduled_event")
    connection.execute("ANALYZE sequent_backend.election_voting_window")


def percentile(values, percent):
    return sorted(values)[max(0, math.ceil(len(values) * percent / 100) - 1)]


def statement_counts(database, requests):
    rows = database.connection.execute(
        "SELECT query, calls FROM pg_stat_statements"
    ).fetchall()
    counts = {"reads": 0, "writes": 0, "transactions": 0}
    measured = []
    for query, calls in rows:
        normalized = re.sub(r"--[^\n]*", "", query).strip().lower().replace('"', "")
        kind = None
        if normalized == "begin":
            kind = "transactions"
        elif normalized.startswith("insert into sequent_backend.cast_vote"):
            kind = "writes"
        elif normalized.lstrip("( \n").startswith("select") and (
            "sequent_backend." in normalized or "from user_entity" in normalized
        ):
            kind = "reads"
        if kind:
            counts[kind] += calls
            measured.append({"kind": kind, "calls": calls, "query": query})
    return {key: value / requests for key, value in counts.items()}, measured


def run_variant(database, fixture, content, variant):
    database.apply(AREA_MIGRATION, "down" if variant == "before" else "up")
    connection = database.connection
    connection.execute(
        "DROP INDEX sequent_backend.cast_vote_participation_election_idx"
    )
    include = " INCLUDE (status, area_id)" if variant == "after" else ""
    connection.execute(
        """
        CREATE INDEX cast_vote_participation_election_idx ON sequent_backend.cast_vote
            (tenant_id, election_event_id, election_id, voter_id_string)
    """
        + include
    )
    storage = "EXTENDED" if variant == "before" else "EXTERNAL"
    connection.execute(
        f"ALTER TABLE sequent_backend.cast_vote ALTER COLUMN content SET STORAGE {storage}"
    )
    prepare_votes(database, fixture, content)

    clients = [VoterClient(database, fixture, variant, content) for _ in range(8)]
    available = Queue()
    for client in clients:
        available.put(client)
    try:
        for request_id in range(WARMUP_REQUESTS):
            clients[0].cast(request_id)
        for client in clients:
            client.checkouts = 0
        connection.execute("SELECT pg_stat_statements_reset()")

        def request(request_id):
            client = available.get()
            started = time.perf_counter()
            try:
                client.cast(request_id)
                return (time.perf_counter() - started) * 1000
            finally:
                available.put(client)

        phases = []
        next_request = WARMUP_REQUESTS
        all_latencies = []
        for name, concurrency, count in PHASES:
            started = time.perf_counter()
            with ThreadPoolExecutor(max_workers=concurrency) as executor:
                latencies = list(
                    executor.map(request, range(next_request, next_request + count))
                )
            seconds = time.perf_counter() - started
            phases.append(
                {
                    "name": name,
                    "concurrency": concurrency,
                    "requests": count,
                    "requests_per_second": count / seconds,
                    "p50_ms": percentile(latencies, 50),
                    "p99_ms": percentile(latencies, 99),
                }
            )
            all_latencies.extend(latencies)
            next_request += count

        counts, statements = statement_counts(database, len(all_latencies))
        expected_reads = 12 if variant == "before" else 6
        expected_transactions = 3 if variant == "before" else 1
        assert counts == {
            "reads": expected_reads,
            "writes": 1,
            "transactions": expected_transactions,
        }, counts
        return {
            "variant": variant,
            "requests": len(all_latencies),
            "phases": phases,
            "p50_ms": percentile(all_latencies, 50),
            "p99_ms": percentile(all_latencies, 99),
            "per_request": dict(
                counts,
                checkouts=sum(client.checkouts for client in clients)
                / len(all_latencies),
                databases=2 if variant == "before" else 1,
            ),
            "postgres_statements": statements,
        }
    finally:
        for client in clients:
            client.close()


def run_benchmark(database, output):
    database.connection.execute("CREATE DATABASE keycloak")
    with psycopg.connect(database.dsn, dbname="keycloak", autocommit=True) as identity:
        identity.execute(
            "CREATE TABLE user_entity (id text PRIMARY KEY, username text)"
        )
        identity.execute(
            """
            INSERT INTO user_entity SELECT 'voter-' || n, 'voter-' || n
            FROM generate_series(0, 49999) n
        """
        )
    content = json.dumps({"ciphertext": base64.b64encode(os.urandom(16000)).decode()})
    report = {
        "scope": "SQL paths only; excludes HTTP, crypto and audit-broker delivery",
        "baseline_commit": "e93ca05104",
        "postgres_version": database.scalar("SELECT version()"),
        "ballot_bytes": len(content),
        "seeded_ballots": 100000,
        "scenarios": [],
    }
    for schedule_count in (100, 2000):
        fixture = seed(database, schedule_count)
        scenario = {"unrelated_schedules": schedule_count, "results": []}
        for variant in ("before", "after"):
            result = run_variant(database, fixture, content, variant)
            scenario["results"].append(result)
            print(
                f"{schedule_count} schedules, {variant}: {result['per_request']}; "
                f"p50={result['p50_ms']:.2f} ms, p99={result['p99_ms']:.2f} ms",
                flush=True,
            )
        report["scenarios"].append(scenario)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2) + "\n")
    print(f"Benchmark evidence written to {output}")
