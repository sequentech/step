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
        return self.tenant, self.event, self.election

    def create(self, connection, limit=3):
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
        return (
            f"tenant_{self.tenant}_event_{self.event}_"
            f"election_{self.election}_{endpoint}_VOTING_PERIOD"
        )

    def schedule(self, connection, endpoint, date):
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
        return (*self.scope, voter, area or self.area, status, content, bytes(64))

    def vote(self, connection, voter, area=None, status="valid"):
        connection.execute(INSERT_VOTE, self.vote_parameters(voter, area, status))
