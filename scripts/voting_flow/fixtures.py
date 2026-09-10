# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Readable, parameterized fixtures; no production identifiers or credentials."""

from dataclasses import dataclass, field
from uuid import UUID, uuid4

from psycopg.types.json import Jsonb

INSERT_VOTE = """
    INSERT INTO sequent_backend.cast_vote
        (tenant_id, election_event_id, election_id, voter_id_string, area_id,
         status, content, ballot_id, cast_ballot_signature)
    VALUES (%s, %s, %s, %s, %s, %s, %s, 'ballot', %s)
"""


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

    def vote_parameters(self, voter, area=None, status="valid", content="ballot"):
        """Return INSERT parameters, using the default area unless an override is supplied."""
        return (*self.scope, voter, area or self.area, status, content, bytes(64))

    def vote(self, connection, voter, area=None, status="valid"):
        """Insert a fixture ballot through the real eligibility trigger."""
        connection.execute(INSERT_VOTE, self.vote_parameters(voter, area, status))


@dataclass
class VotingEvent:
    """An event with real election endpoints and a shared population of areas."""

    elections: list
    areas: list

    def for_voter(self, voter_number):
        # A returning voter keeps the same election and area in both seeded and
        # measured ballots. Areas vary between voters, never within their history.
        """Return the election assigned to this voter by the same mapping used for seeded ballots."""
        return self.elections[voter_number % len(self.elections)]

    def area_for_voter(self, voter_number):
        """Return the stable area assignment used for both prior and measured ballots."""
        return self.areas[voter_number % len(self.areas)]

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
        "sequent_backend.cast_vote, "
        "sequent_backend.area, sequent_backend.election_event, sequent_backend.secret"
    )
