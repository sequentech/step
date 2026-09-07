# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Isolate schedule placement, query shape and index effects inside devenv.

    python3 scripts/voting_flow/schedules.py --output /tmp/schedule-indexes.json

These are single-client query/UPDATE timings, not cast-vote throughput tests.
"""

import argparse
import json
from pathlib import Path
import statistics
import subprocess
import time
from uuid import uuid4

from database import (
    CONFIGURATION_QUERY,
    ROOT,
    SCHEDULE_INDEX,
    WINDOW_MIGRATION,
    local_database,
)
from fixtures import Election

SAMPLES = 21
EXTRA_SCHEDULES = 100_000
BROAD_QUERY = """
    SELECT * FROM sequent_backend.scheduled_event
    WHERE tenant_id = %s AND election_event_id = %s AND archived_at IS NULL
"""
# Compare the same election fields and dates as the production projection query.
# This measures selection cost on valid configuration; it does not replace the
# projection's configuration-write validation and duplicate-endpoint rejection.
DIRECT_QUERY = """
    SELECT election.presentation, election.status, election.voting_channels,
           period.start_date, period.end_date
    FROM sequent_backend.election election
    LEFT JOIN LATERAL (
        SELECT
            max(cron_config ->> 'scheduled_date') FILTER (WHERE task_id = %s) AS start_date,
            max(cron_config ->> 'scheduled_date') FILTER (WHERE task_id = %s) AS end_date
        FROM sequent_backend.scheduled_event
        WHERE tenant_id = election.tenant_id
          AND election_event_id = election.election_event_id
          AND task_id IN (%s, %s)
          AND event_payload = jsonb_build_object('election_id', election.id::text)
          AND archived_at IS NULL
    ) period ON true
    WHERE election.tenant_id = %s AND election.election_event_id = %s AND election.id = %s
"""
# Alternate the value so every sample exercises real projection maintenance,
# rather than the trigger's fast path for unchanged configuration.
RESCHEDULE = """
    UPDATE sequent_backend.scheduled_event
    SET cron_config = jsonb_build_object('scheduled_date',
        CASE cron_config ->> 'scheduled_date'
            WHEN '2026-10-01T12:00:00Z' THEN '2026-10-01T12:00:01Z'
            ELSE '2026-10-01T12:00:00Z'
        END)
    WHERE id = %s
    RETURNING id
"""


def measure(connection, query, parameters):
    for _ in range(3):
        connection.execute(query, parameters).fetchall()
    timings = []
    for _ in range(SAMPLES):
        started = time.perf_counter()
        rows = connection.execute(query, parameters).fetchall()
        timings.append((time.perf_counter() - started) * 1000)
    plan = connection.execute(
        "EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) " + query, parameters
    ).fetchone()[0][0]
    return {
        "query": query,
        "returned_rows": len(rows),
        "samples": SAMPLES,
        "p50_ms": statistics.median(timings),
        "client_ms": timings,
        "explain": plan,
    }


def seed(database, placement):
    connection = database.connection
    connection.execute("TRUNCATE sequent_backend.scheduled_event")
    fixture = Election()
    fixture.create(connection)
    fixture.schedule(connection, "START", "2026-10-01T10:00:00Z")
    closing_id = fixture.schedule(connection, "END", "2026-10-01T12:00:00Z")
    noise_tenant = uuid4() if placement == "other tenant" else fixture.tenant
    noise_event = fixture.event if placement == "same event" else uuid4()
    connection.execute(
        """
        INSERT INTO sequent_backend.scheduled_event
            (tenant_id, election_event_id, task_id, cron_config, event_payload)
        SELECT %s, %s, 'unrelated-' || number, '{}'::jsonb, '{}'::jsonb
        FROM generate_series(1, %s) number
        """,
        (noise_tenant, noise_event, EXTRA_SCHEDULES),
    )
    connection.execute("VACUUM ANALYZE sequent_backend.scheduled_event")
    connection.execute("ANALYZE sequent_backend.election_voting_window")
    return fixture, closing_id


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output", type=Path, default=Path("/tmp/schedule-indexes.json")
    )
    args = parser.parse_args()
    with local_database() as database:
        # Keep planning mode constant across placements. A cached generic plan
        # from the previous distribution can hide a selective index lookup.
        database.connection.prepare_threshold = None
        database.apply(WINDOW_MIGRATION)
        index_sql = database.scalar(
            "SELECT pg_get_indexdef(%s::regclass)", (SCHEDULE_INDEX,)
        )
        report = {
            "implementation_commit": subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
            ).strip(),
            "scope": "21-sample single-client query/UPDATE diagnostic; not cast latency or votes per second",
            "postgres_version": database.scalar("SELECT version()"),
            "extra_schedules": EXTRA_SCHEDULES,
            "index_definition": index_sql,
            "scenarios": [],
        }
        for placement in ("same event", "other event", "other tenant"):
            fixture, closing_id = seed(database, placement)
            endpoints = (fixture.task_name("START"), fixture.task_name("END"))
            direct_parameters = (*endpoints, *endpoints, *fixture.scope)
            for indexed in (False, True):
                if indexed:
                    database.connection.execute(index_sql)
                else:
                    database.connection.execute(f"DROP INDEX {SCHEDULE_INDEX}")
                connection = database.connection
                assert (
                    connection.execute(DIRECT_QUERY, direct_parameters).fetchone()
                    == connection.execute(CONFIGURATION_QUERY, fixture.scope).fetchone()
                ), "Direct and projected policy must agree on the fixture"
                queries = {
                    "broad_event": (BROAD_QUERY, (fixture.tenant, fixture.event)),
                    "two_endpoints": (DIRECT_QUERY, direct_parameters),
                    "projection": (CONFIGURATION_QUERY, fixture.scope),
                    "reschedule": (RESCHEDULE, (closing_id,)),
                }
                measurements = {
                    name: measure(connection, query, parameters)
                    for name, (query, parameters) in queries.items()
                }
                expected_rows = EXTRA_SCHEDULES + 2 if placement == "same event" else 2
                assert measurements["broad_event"]["returned_rows"] == expected_rows
                # Confirm the real UPDATE trigger kept source and projection in sync.
                assert (
                    connection.execute(DIRECT_QUERY, direct_parameters).fetchone()
                    == connection.execute(CONFIGURATION_QUERY, fixture.scope).fetchone()
                )
                report["scenarios"].append(
                    {
                        "placement": placement,
                        "indexed": indexed,
                        "queries": measurements,
                    }
                )
                print(
                    placement,
                    "indexed=" + str(indexed),
                    {
                        key: round(value["p50_ms"], 3)
                        for key, value in measurements.items()
                    },
                    flush=True,
                )
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
