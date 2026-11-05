# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Readable, parameterized fixtures; no production identifiers or credentials."""

from dataclasses import dataclass, field
from uuid import UUID, uuid4

from psycopg.types.json import Jsonb


@dataclass
class Election:
    tenant: UUID = field(default_factory=uuid4)
    event: UUID = field(default_factory=uuid4)
    election: UUID = field(default_factory=uuid4)
    area: UUID = field(default_factory=uuid4)
    other_area: UUID = field(default_factory=uuid4)

    @property
    def scope(self):
        """Return tenant, event and election UUIDs in production SQL parameter order."""
        return self.tenant, self.event, self.election

    def create(self, connection, limit=3):
        """Insert an open election with online voting enabled and the supplied revote limit."""
        connection.execute(
            """
            INSERT INTO sequent_backend.election
                (id, tenant_id, election_event_id, num_allowed_revotes,
                 status, presentation, voting_channels)
            VALUES (%s, %s, %s, %s, %s, %s, %s)
        """,
            (
                self.election,
                self.tenant,
                self.event,
                limit,
                Jsonb({"voting_status": "OPEN"}),
                Jsonb({"grace_period_secs": 0}),
                Jsonb({"online": True}),
            ),
        )

    def task_name(self, endpoint):
        """Return the canonical START/END voting-period task name for this election."""
        return (
            f"tenant_{self.tenant}_event_{self.event}_"
            f"election_{self.election}_{endpoint}_VOTING_PERIOD"
        )

    def schedule(self, connection, endpoint, date):
        """Insert an endpoint with canonical payload and return its generated schedule UUID."""
        row = connection.execute(
            """
            INSERT INTO sequent_backend.scheduled_event
                (tenant_id, election_event_id, task_id, event_payload, cron_config)
            VALUES (%s, %s, %s, %s, %s)
            RETURNING id
        """,
            (
                self.tenant,
                self.event,
                self.task_name(endpoint),
                Jsonb({"election_id": str(self.election)}),
                Jsonb({"scheduled_date": date}),
            ),
        ).fetchone()
        return row[0]


@dataclass
class VotingEvent:
    """An event with real election endpoints and a shared population of areas."""

    elections: list
    areas: list

    @classmethod
    def create(
        cls, connection, election_count, area_count, schedules_per_election, tenant=None
    ):
        """Seed real elections, populated topology and schedules within the event limits.

        Create area rows and two canonical endpoints per election, then fill out
        schedules_per_election with other tasks. The total includes endpoints and
        cannot exceed ten schedules per election or 200 elections per event."""
        assert 1 <= election_count <= 200
        assert 2 <= schedules_per_election <= 10
        assert area_count >= 1
        tenant = tenant or uuid4()
        event = uuid4()
        areas = [uuid4() for _ in range(area_count)]
        connection.execute(
            "INSERT INTO sequent_backend.election_event VALUES (%s, %s, %s, 'board')",
            (event, tenant, Jsonb({})),
        )
        connection.execute(
            "INSERT INTO sequent_backend.area SELECT unnest(%s::uuid[]), %s, %s, '{}'::jsonb",
            (areas, tenant, event),
        )
        connection.execute(
            "INSERT INTO sequent_backend.secret VALUES (%s, %s, 'protocol-manager', %s)",
            (tenant, event, "encrypted-signing-key" * 64),
        )
        elections = []
        for _ in range(election_count):
            election = Election(tenant=tenant, event=event, area=areas[0])
            election.create(connection, limit=100)
            election.schedule(connection, "START", "2026-10-01T10:00:00Z")
            election.schedule(connection, "END", "2026-10-01T12:00:00Z")
            # The total includes both endpoints: 200 elections × 10 means
            # exactly 2,000 schedules, not 2,000 extra rows plus endpoints.
            connection.execute(
                """
                INSERT INTO sequent_backend.scheduled_event
                    (tenant_id, election_event_id, task_id, event_payload, cron_config)
                SELECT %s, %s, %s || '-other-' || n,
                       jsonb_build_object('election_id', %s::text), '{}'::jsonb
                FROM generate_series(1, %s) n
                """,
                (
                    tenant,
                    event,
                    str(election.election),
                    str(election.election),
                    schedules_per_election - 2,
                ),
            )
            elections.append(election)
        connection.execute(
            "UPDATE sequent_backend.election SET eml = %s WHERE election_event_id = %s",
            ("election-configuration" * 1600, event),
        )
        return cls(elections, areas)


def clear_workload(connection):
    """Remove prior scenarios so table cardinalities match the report exactly."""
    connection.execute(
        "TRUNCATE sequent_backend.scheduled_event, sequent_backend.election, "
        "sequent_backend.election_voting_window, "
        "sequent_backend.area, sequent_backend.election_event, sequent_backend.secret"
    )
