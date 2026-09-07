# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Read-only Hasura voter-context regressions against a populated local environment.

Apply the ballot-style election relationship first. Run inside devenv with
ADMIN_SECRET and optionally VOTER_CONTEXT_HASURA_URL. Uses Hasura admin role
impersonation to test the same row permissions applied to validated JWT claims;
this does not test JWT verification itself. No ballots or identifiers are logged.
"""

import json
import os
from pathlib import Path
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parents[1]
ENDPOINT = os.environ.get(
    "VOTER_CONTEXT_HASURA_URL", "http://graphql-engine:8080/v1/graphql"
)
QUERY = (
    (ROOT / "packages/voting-portal/src/queries/GetVoterContext.ts")
    .read_text()
    .split("gql`", 1)[1]
    .split("`", 1)[0]
)


def query(document, variables=None, claims=None):
    """Execute a local GraphQL request without printing returned voter content."""
    headers = {
        "Content-Type": "application/json",
        "x-hasura-admin-secret": os.environ.get("ADMIN_SECRET", "admin"),
        **(claims or {}),
    }
    request = urllib.request.Request(
        ENDPOINT,
        data=json.dumps({"query": document, "variables": variables or {}}).encode(),
        headers=headers,
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        result = json.load(response)
    if result.get("errors"):
        raise AssertionError("Hasura rejected the voter-context regression query")
    return result["data"]


def main():
    """Verify successful access and rejection of independently mismatched scopes."""
    styles = query(
        """query {
        sequent_backend_ballot_style(limit: 1, where: {deleted_at: {_is_null: true}, area_id: {_is_null: false}}) {
            tenant_id election_event_id election_id area_id
        }
    }"""
    )["sequent_backend_ballot_style"]
    if not styles:
        raise AssertionError("A populated local ballot-style fixture is required")
    style = styles[0]
    variables = {
        "tenantId": style["tenant_id"],
        "electionEventId": style["election_event_id"],
    }
    claims = {
        "x-hasura-role": "user",
        "x-hasura-tenant-id": style["tenant_id"],
        "x-hasura-area-id": style["area_id"],
        "x-hasura-election-event-id": style["election_event_id"],
        "x-hasura-authorized-election-ids": "{" + style["election_id"] + "}",
        "x-hasura-user-id": str(uuid.uuid4()),
    }
    result = query(QUERY, variables, claims)
    assert result["sequent_backend_ballot_style"]
    for row in result["sequent_backend_ballot_style"]:
        for field in ("tenant_id", "area_id", "election_event_id", "election_id"):
            assert row[field] == style[field]
        assert row["election"]["id"] == style["election_id"]
        assert row["election"]["tenant_id"] == style["tenant_id"]
        assert row["election"]["election_event_id"] == style["election_event_id"]
    assert result["sequent_backend_cast_vote"] == []
    election_query = (
        "query($id: uuid!) { sequent_backend_election(where: {id: {_eq: $id}}) { id } }"
    )
    assert query(election_query, {"id": style["election_id"]}, claims)[
        "sequent_backend_election"
    ]
    assert (
        query(
            election_query,
            {"id": style["election_id"]},
            {**claims, "x-hasura-authorized-election-ids": "{}"},
        )["sequent_backend_election"]
        == []
    )
    for name in (
        "x-hasura-tenant-id",
        "x-hasura-area-id",
        "x-hasura-election-event-id",
        "x-hasura-authorized-election-ids",
    ):
        changed = {
            **claims,
            name: "{}" if name.endswith("election-ids") else str(uuid.uuid4()),
        }
        result = query(QUERY, variables, changed)
        assert result["sequent_backend_ballot_style"] == []
        assert result["sequent_backend_cast_vote"] == []
    for name in ("tenantId", "electionEventId"):
        result = query(QUERY, {**variables, name: str(uuid.uuid4())}, claims)
        assert result["sequent_backend_ballot_style"] == []
        assert result["sequent_backend_cast_vote"] == []
        assert result["sequent_backend_election_event"] == []
    print(
        "Voter context: scoped access, nested election isolation, empty eligibility and tenant/event/area mismatches passed"
    )


if __name__ == "__main__":
    main()
