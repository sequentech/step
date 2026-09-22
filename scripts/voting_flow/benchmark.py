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
from dataclasses import asdict, dataclass
import base64
import json
import math
import os
import platform
import subprocess
from queue import Queue
import re
import time

import psycopg

from database import AREA_MIGRATION, CONFIGURATION_QUERY, ROOT, SCHEDULE_INDEX
from fixtures import VotingEvent, INSERT_VOTE, clear_workload


def scenario_label(scenario):
    """Identify SQL workload dimensions without depending on a report renderer."""
    return (
        f"{scenario['seeded_ballots']:,} votes table, "
        f"{scenario['peak_voters']} concurrent voters, "
        f"{scenario['election_count']} elections, "
        f"{scenario['area_count']:,} areas, "
        f"{scenario['election_count'] * scenario['schedules_per_election']:,} total schedules"
    )


WARMUP_REQUESTS = 64


@dataclass(frozen=True)
class Scenario:
    name: str
    seeded_ballots: int = 100_000
    peak_voters: int = 8
    election_count: int = 10
    area_count: int = 100
    schedules_per_election: int = 10

    @property
    def schedule_count(self):
        """Return total event schedules, including both endpoints for each election."""
        return self.election_count * self.schedules_per_election

    @property
    def phases(self):
        """Return opening/lull/closing tuples of phase name, concurrency and request count."""
        return (
            ("opening", self.peak_voters, 1024),
            ("lull", max(2, self.peak_voters // 4), 512),
            ("closing", self.peak_voters, 1024),
        )

    def voter_id(self, request_id):
        # Spread distinct voters across the entire seeded population instead of
        # repeatedly touching the first few index pages. This multiplier is
        # coprime to every population below, so request IDs cannot collide.
        """Map a request to a distinct seeded voter using a permutation of the tested populations."""
        return (request_id * 104729) % (self.seeded_ballots // 2)


# Isolate ballots, concurrency and area cardinality, then combine their maxima.
# Election/schedule cases obey at most 200 elections and 10 schedules each.
SCENARIOS = (
    Scenario("10k-votes", seeded_ballots=10_000),
    Scenario("100k-votes"),
    Scenario("1m-votes", seeded_ballots=1_000_000),
    Scenario("100k-votes-32-voters", peak_voters=32),
    Scenario("100k-votes-64-voters", peak_voters=64),
    Scenario("1m-votes-64-voters", seeded_ballots=1_000_000, peak_voters=64),
    Scenario("100k-votes-200-elections", election_count=200),
    Scenario(
        "100k-votes-200-elections-400-schedules",
        election_count=200,
        schedules_per_election=2,
    ),
    Scenario("100k-votes-1k-areas", area_count=1000),
    Scenario("100k-votes-10k-areas", area_count=10_000),
    Scenario(
        "1m-votes-64-voters-200-elections-10k-areas",
        seeded_ballots=1_000_000,
        peak_voters=64,
        election_count=200,
        area_count=10_000,
    ),
)
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
        """Open reusable writer and, for the baseline only, identity-database connections."""
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
        """Close every connection owned by this worker."""
        self.writer.close()
        if self.identity:
            self.identity.close()

    @contextmanager
    def checkout(self, connection, read_only=False):
        """Count a logical checkout and yield a transaction, optionally rolling it back on exit."""
        self.checkouts += 1
        # The original identity/audit transactions were dropped without COMMIT.
        with connection.transaction(force_rollback=read_only):
            yield connection

    def cast(self, request_id):
        """Submit one returning voter through the selected SQL path and propagate any failure."""
        voter = f"voter-{request_id}"
        fixture = self.fixture.for_voter(request_id)
        area = self.fixture.area_for_voter(request_id)
        with self.checkout(self.writer) as connection:
            connection.execute(
                "SELECT * FROM sequent_backend.area WHERE tenant_id = %s AND id = %s",
                (fixture.tenant, area),
            ).fetchone()
            connection.execute(
                "SELECT * FROM sequent_backend.election_event WHERE tenant_id = %s AND id = %s",
                (fixture.tenant, fixture.event),
            ).fetchone()
            connection.execute(READ_SECRET, (fixture.tenant, fixture.event)).fetchone()

            if self.variant == "before":
                self.read_original_configuration(connection, voter, fixture)
                returning = " RETURNING *"
            else:
                connection.execute(CONFIGURATION_QUERY, fixture.scope).fetchone()
                returning = " RETURNING id, ballot_id, election_id, election_event_id, tenant_id, area_id, created_at, last_updated_at, cast_ballot_signature, voter_id_string, status"

            connection.execute(
                INSERT_VOTE + returning,
                fixture.vote_parameters(voter, area=area, content=self.content),
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

    def read_original_configuration(self, connection, voter, fixture):
        # new_from_sk reloads the same key just read by get_electoral_log.
        """Execute the baseline signing, policy, schedule and prior-vote reads in their original order."""
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


def prepare_votes(database, fixture, content, scenario):
    """Restore identical voter/election/area history and analyze relations outside measured work."""
    connection = database.connection
    connection.execute("TRUNCATE sequent_backend.cast_vote")
    connection.execute(
        "ALTER TABLE sequent_backend.cast_vote DISABLE TRIGGER check_revote_limit_trigger"
    )
    try:
        # Keep two prior ballots per voter while varying total table size.
        # Restore the identical population before each variant; seed work is
        # excluded from the timed interval and PostgreSQL statement counters.
        connection.execute(
            """
            INSERT INTO sequent_backend.cast_vote
                (tenant_id, election_event_id, election_id, area_id,
                 voter_id_string, status, content)
            SELECT %s, %s,
                   (%s::uuid[])[((number / 2) %% %s)::int + 1],
                   (%s::uuid[])[((number / 2) %% %s)::int + 1],
                   'voter-' || (number / 2), 'valid', %s
            FROM generate_series(0, %s - 1) AS number
        """,
            (
                fixture.elections[0].tenant,
                fixture.elections[0].event,
                [e.election for e in fixture.elections],
                len(fixture.elections),
                fixture.areas,
                len(fixture.areas),
                content,
                scenario.seeded_ballots,
            ),
        )
    finally:
        connection.execute(
            "ALTER TABLE sequent_backend.cast_vote ENABLE TRIGGER check_revote_limit_trigger"
        )
    connection.execute("VACUUM ANALYZE sequent_backend.cast_vote")
    connection.execute("ANALYZE sequent_backend.scheduled_event")
    connection.execute("ANALYZE sequent_backend.election_voting_window")
    connection.execute("ANALYZE sequent_backend.area")
    connection.execute("ANALYZE sequent_backend.election")


def percentile(values, percent):
    """Return the nearest-rank percentile of a nonempty sequence of latency measurements."""
    return sorted(values)[max(0, math.ceil(len(values) * percent / 100) - 1)]


def statement_counts(database, requests):
    """Return per-request SELECT/INSERT/BEGIN counts and supporting pg_stat_statements rows."""
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


def run_variant(database, fixture, content, variant, scenario, schedule_index_sql):
    """Measure one schema/path variant with warmed connections and restored ballots.

    Verify populated elections/areas, distinct voters, accepted inserts and SQL
    operation counts. Return timings, coverage and statement evidence; release
    all clients even if a request or assertion fails. Setup is excluded from timing."""
    database.apply(AREA_MIGRATION, "down" if variant == "before" else "up")
    connection = database.connection
    # The original schema had no schedule-scope index. Restore the migration's
    # exact index definition only for the revised path, outside timed work.
    connection.execute(f"DROP INDEX IF EXISTS {SCHEDULE_INDEX}")
    if variant == "after":
        connection.execute(schedule_index_sql)
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
    prepare_votes(database, fixture, content, scenario)

    seeded_count = database.scalar("SELECT count(*) FROM sequent_backend.cast_vote")
    assert seeded_count == scenario.seeded_ballots, seeded_count
    populated_elections, populated_areas = connection.execute(
        "SELECT count(DISTINCT election_id), count(DISTINCT area_id) "
        "FROM sequent_backend.cast_vote"
    ).fetchone()
    assert (populated_elections, populated_areas) == (
        scenario.election_count,
        scenario.area_count,
    )

    relation_bytes = database.scalar(
        "SELECT pg_total_relation_size('sequent_backend.cast_vote')"
    )
    request_count = sum(count for _, _, count in scenario.phases)
    voter_ids = [scenario.voter_id(i) for i in range(WARMUP_REQUESTS + request_count)]
    assert len(set(voter_ids)) == len(voter_ids), "voters must be distinct"

    clients = [
        VoterClient(database, fixture, variant, content)
        for _ in range(scenario.peak_voters)
    ]
    available = Queue()
    for client in clients:
        available.put(client)
    try:
        for request_id in range(WARMUP_REQUESTS):
            clients[request_id % len(clients)].cast(voter_ids[request_id])
        for client in clients:
            client.checkouts = 0
        connection.execute("SELECT pg_stat_statements_reset()")

        def request(request_id):
            """Borrow a worker, time one distinct voter submission, and always return the worker."""
            client = available.get()
            started = time.perf_counter()
            try:
                client.cast(voter_ids[request_id])
                return (time.perf_counter() - started) * 1000
            finally:
                available.put(client)

        phases = []
        next_request = WARMUP_REQUESTS
        all_latencies = []
        for name, concurrency, count in scenario.phases:
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
                    "elapsed_seconds": seconds,
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
        # Check acceptance after collecting statement statistics so verification
        # queries cannot inflate the measured work per successful request.
        final_count = database.scalar("SELECT count(*) FROM sequent_backend.cast_vote")
        assert final_count - seeded_count == WARMUP_REQUESTS + request_count
        return {
            "variant": variant,
            "seeded_relation_bytes": relation_bytes,
            "distinct_measured_voters": request_count,
            "distinct_measured_areas": len(
                {fixture.area_for_voter(v) for v in voter_ids[WARMUP_REQUESTS:]}
            ),
            "distinct_measured_elections": len(
                {fixture.for_voter(v).election for v in voter_ids[WARMUP_REQUESTS:]}
            ),
            "accepted_requests": request_count,
            "errors": 0,
            "requests": len(all_latencies),
            "elapsed_seconds": sum(phase["elapsed_seconds"] for phase in phases),
            "requests_per_second": len(all_latencies)
            / sum(phase["elapsed_seconds"] for phase in phases),
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


def run_benchmark(database, output, scenario_names=None):
    """Run the selected scenarios sequentially and write their raw evidence to output.

    Each before/after pair starts with the same ballots and topology. A failed
    request or invariant aborts the run instead of publishing a successful report.
    With no scenario_names filter, measure the full configured matrix."""
    schedule_index_sql = database.scalar(
        "SELECT pg_get_indexdef(%s::regclass)", (SCHEDULE_INDEX,)
    )
    database.connection.execute("CREATE DATABASE keycloak")
    with psycopg.connect(database.dsn, dbname="keycloak", autocommit=True) as identity:
        identity.execute(
            "CREATE TABLE user_entity (id text PRIMARY KEY, username text)"
        )
        identity.execute(
            """
            INSERT INTO user_entity SELECT 'voter-' || n, 'voter-' || n
            FROM generate_series(0, 499999) n
        """
        )
    content = json.dumps({"ciphertext": base64.b64encode(os.urandom(16000)).decode()})
    report = {
        "scope": "SQL paths only; excludes HTTP, crypto and audit-broker delivery",
        "baseline_commit": "e93ca05104",
        "implementation_commit": subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
        ).strip(),
        "machine": platform.machine(),
        "available_cpus": os.cpu_count(),
        "postgres_version": database.scalar("SELECT version()"),
        "ballot_bytes": len(content),
        "warmup_requests": WARMUP_REQUESTS,
        "voter_distribution": "distinct returning voters spread over the seeded population; two prior ballots each",
        "postgres_settings": dict(
            database.connection.execute(
                "SELECT name, setting FROM pg_settings WHERE name IN "
                "('max_connections', 'shared_buffers', 'fsync', 'synchronous_commit')"
            ).fetchall()
        ),
        "scenarios": [],
    }
    for scenario in SCENARIOS:
        if scenario_names and scenario.name not in scenario_names:
            continue
        clear_workload(database.connection)
        fixture = VotingEvent.create(
            database.connection,
            scenario.election_count,
            scenario.area_count,
            scenario.schedules_per_election,
        )
        cardinalities = {
            table: database.scalar(f"SELECT count(*) FROM sequent_backend.{table}")
            for table in ("election", "area", "scheduled_event")
        }
        assert cardinalities == {
            "election": scenario.election_count,
            "area": scenario.area_count,
            "scheduled_event": scenario.schedule_count,
        }, cardinalities
        evidence = dict(
            asdict(scenario),
            schedule_count=scenario.schedule_count,
            verified_cardinalities=cardinalities,
            results=[],
        )
        for variant in ("before", "after"):
            print(
                f"Preparing {scenario_label(asdict(scenario))}, {variant}: {scenario.seeded_ballots:,} ballots",
                flush=True,
            )
            result = run_variant(
                database, fixture, content, variant, scenario, schedule_index_sql
            )
            evidence["results"].append(result)
            print(
                f"{scenario_label(asdict(scenario))}, {variant}: {result['per_request']}; "
                f"p50={result['p50_ms']:.2f} ms, p99={result['p99_ms']:.2f} ms",
                flush=True,
            )
        report["scenarios"].append(evidence)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2) + "\n")
    print(f"Benchmark evidence written to {output}")
