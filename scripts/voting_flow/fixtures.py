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
