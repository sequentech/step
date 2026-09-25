# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""The voting portal's protocol: login, published files, encryption and casting."""

import json
import subprocess

from .client import ENV, ROOT, TENANT_ID, Hasura, Keycloak, event_realm, http_get
from .fixtures import VOTER_PASSWORD


def portal_query(name):
    """The GraphQL document the voting portal sends, read from its source."""
    source = (ROOT / f"packages/voting-portal/src/queries/{name}.ts").read_text()
    return source.split("gql`", 1)[1].split("`", 1)[0].strip()


GET_VOTER_STATUS = portal_query("GetVoterStatus")
INSERT_CAST_VOTE = portal_query("InsertCastVote")


def candidate_name(candidate):
    """The English name a ballot style shows for a candidate."""
    i18n = ((candidate.get("presentation") or {}).get("i18n") or {}).get("en") or {}
    return (
        candidate.get("name")
        or (candidate.get("name_i18n") or {}).get("en")
        or i18n.get("name")
    )


class Portal:
    """One voter session per login, as a browser on the voting portal would have."""

    def __init__(self, election_event_id, cli):
        self.election_event_id = election_event_id
        self.cli = cli
        self.keycloak = Keycloak()
        self.redirect_uri = f"{ENV['VOTING_PORTAL_URL'].rstrip('/')}/tenant/{TENANT_ID}/event/{election_event_id}/login"

    def login(self, username):
        tokens = self.keycloak.browser_login(
            event_realm(self.election_event_id),
            "voting-portal",
            self.redirect_uri,
            username,
            VOTER_PASSWORD,
        )
        return tokens["access_token"]

    def status(self, token):
        """GetVoterStatus: the voter's ballot files and their own cast votes."""
        return Hasura(token=token).query(
            GET_VOTER_STATUS, {"electionEventId": self.election_event_id}
        )

    @staticmethod
    def ballot_style(file):
        """Rebuild the published ballot style from the voter's signed URLs."""
        event = http_get(file["urls"]["event_url"])
        style = http_get(file["urls"]["style_url"])
        if event.status != 200 or style.status != 200:
            raise AssertionError(f"Ballot files answered {event.status}/{style.status}")
        style = style.json()
        if style.get("ballot_eml_prefix") is None:
            return json.loads(style["ballot_eml"]), style
        text = (
            style["ballot_eml_prefix"]
            + event.json()["ballot_eml_presentation"]
            + style["ballot_eml_suffix"]
        )
        return json.loads(text), style

    def encrypt(self, style, candidate):
        """Encrypt a vote for `candidate` (a display name) with step-cli's ballot encoder."""
        (contest,) = style["contests"]
        names = [candidate_name(c) for c in contest["candidates"]]
        if candidate not in names:
            raise AssertionError(f"{candidate} is not on the ballot: {names}")
        choices = [
            {
                "contest_id": contest["id"],
                "is_explicit_invalid": False,
                "is_decline_to_vote": False,
                "is_blank_ballot": False,
                "invalid_errors": [],
                "invalid_alerts": [],
                "choices": [
                    {
                        "id": c["id"],
                        "selected": 0 if name == candidate else -1,
                        "write_in_text": None,
                    }
                    for c, name in zip(contest["candidates"], names)
                ],
            }
        ]
        style_path = self.cli.directory / "style.json"
        choices_path = self.cli.directory / "choices.json"
        style_path.write_text(json.dumps(style))
        choices_path.write_text(json.dumps(choices))
        result = subprocess.run(
            [
                str(self.cli.binary),
                "load",
                "encrypt",
                str(style_path),
                str(choices_path),
                "1",
            ],
            capture_output=True,
            text=True,
            check=True,
            env={**ENV, "HOME": str(self.cli.directory)},
        )
        (line,) = result.stdout.strip().splitlines()
        return json.loads(line)

    @staticmethod
    def cast(token, ballot):
        """InsertCastVote as the portal sends it; returns (receipt, errors)."""
        result = Hasura(token=token).execute(
            INSERT_CAST_VOTE,
            {
                "electionId": ballot["electionId"],
                "ballotId": ballot["ballotId"],
                "content": ballot["content"],
            },
        )
        receipt = (result.get("data") or {}).get("insert_cast_vote")
        return receipt, result.get("errors")
