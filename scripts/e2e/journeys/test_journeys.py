# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Backend journeys, run in order against one fresh stack (scripts/e2e/run.sh).

Each journey builds on the previous ones and is skipped when one it needs did
not pass. Skipped journeys fail the run; every scenario must pass.
"""

import collections
import json
import re
import unittest
import uuid
from datetime import datetime, timezone

from . import bootstrap, fixtures
from .client import (
    B4_URL,
    ENV,
    OUTPUT,
    S3_URL,
    TENANT_ID,
    TENANT_REALM,
    GraphQLError,
    Hasura,
    Keycloak,
    StepCli,
    event_realm,
    http_get,
    jwt_claims,
    wait_until,
)
from .voting import Portal, candidate_name

ELECTORAL_LOG = """
query ($eventId: String) {
  listElectoralLog(election_event_id: $eventId, limit: 500, offset: 0) {
    total { aggregate { count } }
    items { statement_kind user_id }
  }
}"""


class State:
    """What earlier journeys produced; `passed` names the journeys that passed."""

    def __init__(self):
        self.passed = set()


class BackendJourneys(unittest.TestCase):
    state = State()

    @classmethod
    def setUpClass(cls):
        OUTPUT.mkdir(parents=True, exist_ok=True)
        cls.state = State()
        cls.tag = uuid.uuid4().hex[:8]
        cls.keycloak = Keycloak()
        cls.admin = Hasura.admin()
        cls.cli = StepCli()

    # Helpers

    def requires(self, *journeys):
        missing = [journey for journey in journeys if journey not in self.state.passed]
        if missing:
            self.skipTest(f"needs {', '.join(missing)}")

    def passed(self, journey):
        self.state.passed.add(journey)

    def step(self, *args, **kwargs):
        """Run an administrator command with a fresh login (gold operations need one)."""
        bootstrap.configure_step_cli(self.cli)
        return self.cli.step(*args, **kwargs)

    def admin_token(self):
        return bootstrap.admin_token(self.keycloak)

    def electoral_log(self, event_id):
        data = Hasura(token=self.admin_token()).query(
            ELECTORAL_LOG, {"eventId": event_id}
        )["listElectoralLog"]
        return collections.Counter(item["statement_kind"] for item in data["items"])

    def wait_for_log(self, event_id, expected, timeout=120):
        """Wait until the electoral log holds at least `expected` entries per kind."""

        def complete():
            counts = self.electoral_log(event_id)
            return (
                counts
                if all(counts[kind] >= count for kind, count in expected.items())
                else None
            )

        try:
            return wait_until(
                f"electoral log entries {dict(expected)}",
                complete,
                timeout=timeout,
                interval=3,
            )
        except TimeoutError as error:
            raise AssertionError(
                f"{error}; found {dict(self.electoral_log(event_id))}"
            ) from error

    def event_rows(self, query, **variables):
        return self.admin.query(query, {"event": self.state.event_id, **variables})

    def assert_rejected(self, errors, code, message):
        self.assertIsNotNone(errors, "the vote was accepted")
        self.assertEqual(
            [(e.get("extensions") or {}).get("code") for e in errors], [code], errors
        )
        self.assertEqual(errors[0]["message"], message)

    def import_event(self, document, name):
        path = self.cli.directory / f"{name}.json"
        path.write_text(json.dumps(document))
        return StepCli.last_id(
            self.step("import-election", "--file-path", str(path), "--is-local")
        )

    def import_tasks(self, since):
        """Import task executions the tenant started since `since`."""
        return self.admin.query(
            """query ($tenant: uuid!, $since: timestamptz!) {
              sequent_backend_tasks_execution(where: {tenant_id: {_eq: $tenant}, type: {_eq: "IMPORT_ELECTION_EVENT"},
                                                      created_at: {_gte: $since}}) { execution_status logs }
            }""",
            {"tenant": TENANT_ID, "since": since.isoformat()},
        )["sequent_backend_tasks_execution"]

    def voter_votes(self, username):
        user = self.keycloak.user(event_realm(self.state.event_id), username)
        return self.admin.query(
            """query ($event: uuid!, $voter: String!) {
              sequent_backend_cast_vote(where: {election_event_id: {_eq: $event}, voter_id_string: {_eq: $voter}},
                                        order_by: {created_at: asc}) { area_id election_id status }
            }""",
            {"event": self.state.event_id, "voter": user["id"]},
        )["sequent_backend_cast_vote"]

    def vote(self, voter, candidate, token=None):
        """Cast as `voter` from its own session; returns (receipt, errors)."""
        portal = self.state.portal
        token = token or portal.login(voter)
        (file,) = portal.status(token)["get_ballot_files_urls"]["files"]
        style, _ = portal.ballot_style(file)
        ballot = portal.encrypt(style, candidate)
        receipt, errors = portal.cast(token, ballot)
        if receipt:
            self.assertEqual(receipt["ballot_id"], ballot["ballotId"])
            self.assertEqual(receipt["election_id"], ballot["electionId"])
            self.state.accepted.append(
                (voter, style["area_id"], style["contests"][0]["id"], candidate)
            )
        else:
            self.state.rejected += 1
        return receipt, errors

    # Journeys

    def test_1_super_tenant_bootstrap(self):
        """The stack's beat and windmill created the super tenant's realm, row and JWKS."""
        (tenant,) = self.admin.query(
            "query ($id: uuid!) { sequent_backend_tenant(where: {id: {_eq: $id}}) { id slug is_active } }",
            {"id": TENANT_ID},
        )["sequent_backend_tenant"]
        self.assertEqual(tenant["slug"], ENV["ENV_SLUG"].lower())
        self.assertTrue(tenant["is_active"])

        # insert_tenant renames the imported realm after the tenant slug.
        realm = self.keycloak.admin("GET", TENANT_REALM)
        self.assertEqual(realm["displayName"], tenant["slug"])

        jwks = http_get(
            f"{S3_URL}/{ENV['AWS_S3_PUBLIC_BUCKET']}/{ENV['AWS_S3_JWKS_CERTS_PATH']}"
        )
        self.assertEqual(jwks.status, 200)
        kids = bootstrap.realm_signing_kids()
        self.assertTrue(kids)
        self.assertLessEqual(kids, {key["kid"] for key in jwks.json()["keys"]})
        # Written by windmill with its cache policy, unlike the empty seed file.
        self.assertEqual(
            jwks.headers.get("Cache-Control"), ENV["AWS_S3_JWKS_CACHE_POLICY"]
        )

        token = self.admin_token()
        claims = jwt_claims(token)["https://hasura.io/jwt/claims"]
        self.assertEqual(claims["x-hasura-tenant-id"], TENANT_ID)
        rows = Hasura(token=token).query("{ sequent_backend_tenant { id } }")[
            "sequent_backend_tenant"
        ]
        self.assertEqual(rows, [{"id": TENANT_ID}])

        # Control: Hasura verifies the signature against that JWKS.
        header, payload, signature = token.split(".")
        forged = f"{header}.{payload}.{signature[:-4]}{'AAAA' if signature[-4:] != 'AAAA' else 'BBBB'}"
        errors = (
            Hasura(token=forged)
            .execute("{ sequent_backend_tenant { id } }")
            .get("errors")
        )
        self.assertEqual([e["extensions"]["code"] for e in errors], ["invalid-jwt"])
        self.passed("journey 1")

    def test_2_import_election_event(self):
        """Importing an event creates its elections, contests, areas, board and electoral log."""
        self.requires("journey 1")
        document = fixtures.election_event(ENV["VOTING_PORTAL_URL"], self.tag)
        self.state.document = document
        started = datetime.now(timezone.utc)
        event_id = self.import_event(document, "event")
        self.state.event_id = event_id

        data = self.event_rows(
            """query ($event: uuid!) {
              sequent_backend_election_event(where: {id: {_eq: $event}}) { tenant_id presentation bulletin_board_reference }
              sequent_backend_election(where: {election_event_id: {_eq: $event}}) { id external_id num_allowed_revotes presentation }
              sequent_backend_contest(where: {election_event_id: {_eq: $event}}) { id election_id presentation }
              sequent_backend_area(where: {election_event_id: {_eq: $event}}) { id name }
              sequent_backend_area_contest(where: {election_event_id: {_eq: $event}}) { area_id contest_id }
              sequent_backend_candidate(where: {election_event_id: {_eq: $event}}) { id contest_id presentation }
            }"""
        )
        (event,) = data["sequent_backend_election_event"]
        self.assertEqual(event["tenant_id"], TENANT_ID)
        self.assertEqual(
            fixtures.display_name(event),
            fixtures.display_name(document["election_event"]),
        )

        elections = {
            fixtures.display_name(e): e for e in data["sequent_backend_election"]
        }
        self.assertEqual(sorted(elections), ["E2E grace election", "E2E main election"])
        main, grace = elections["E2E main election"], elections["E2E grace election"]
        self.assertEqual(main["external_id"], f"e2e-main-{self.tag}")
        self.assertIsNone(grace["external_id"])
        self.assertEqual(main["num_allowed_revotes"], fixtures.MAIN_ALLOWED_VOTES)
        self.state.elections = {"main": main["id"], "grace": grace["id"]}

        areas = {a["name"]: a["id"] for a in data["sequent_backend_area"]}
        self.assertEqual(sorted(areas), sorted(spec.name for spec in fixtures.AREAS))
        contests = {
            fixtures.display_name(c): c for c in data["sequent_backend_contest"]
        }
        self.assertEqual(
            sorted(contests), sorted(spec.contest for spec in fixtures.AREAS)
        )
        links = {
            (link["area_id"], link["contest_id"])
            for link in data["sequent_backend_area_contest"]
        }
        candidates = collections.defaultdict(dict)
        for candidate in data["sequent_backend_candidate"]:
            candidates[candidate["contest_id"]][fixtures.display_name(candidate)] = (
                candidate["id"]
            )
        self.state.areas, self.state.contests, self.state.candidates = {}, {}, {}
        for spec in fixtures.AREAS:
            contest = contests[spec.contest]
            self.assertEqual(
                contest["election_id"], self.state.elections[spec.election]
            )
            self.assertIn((areas[spec.name], contest["id"]), links)
            self.assertEqual(sorted(candidates[contest["id"]]), sorted(spec.candidates))
            self.state.areas[spec.key] = areas[spec.name]
            self.state.contests[spec.key] = contest["id"]
            self.state.candidates[spec.key] = candidates[contest["id"]]
        self.assertEqual(len(links), len(fixtures.AREAS))
        self.assertEqual(
            [task["execution_status"] for task in self.import_tasks(started)],
            ["SUCCESS"],
        )

        board = event["bulletin_board_reference"]["database_name"]
        self.state.board = board
        boards = {
            b["name"]: b["status"]
            for b in http_get(f"{B4_URL}/boards").json()["boards"]
        }
        self.assertEqual(boards.get(board), "active")

        self.assertEqual(
            self.keycloak.admin("GET", event_realm(event_id))["realm"],
            event_realm(event_id),
        )
        # The event's electoral log database answers; an unknown event's does not.
        self.assertEqual(self.electoral_log(event_id), collections.Counter())
        with self.assertRaises(GraphQLError):
            self.electoral_log(str(uuid.uuid4()))
        self.passed("journey 2")

    def test_2b_invalid_event_document_is_rejected(self):
        """A bundle whose area links a contest it does not contain is refused before any import."""
        self.requires("journey 2")
        broken = fixtures.dangling_contest_link(self.state.document)
        path = self.cli.directory / "invalid-event.json"
        path.write_text(json.dumps(broken))
        bootstrap.configure_step_cli(self.cli)
        started = datetime.now(timezone.utc)
        code, output = self.cli.run(
            "step",
            "import-election",
            "--file-path",
            str(path),
            "--is-local",
            check=False,
        )
        self.assertNotEqual(code, 0, output)
        last = len(broken["area_contests"]) - 1
        self.assertIn("The election event bundle cannot be imported", output)
        self.assertIn(
            f"area_contests[{last}].contest_id: points at a contest that is not in the bundle",
            output,
        )

        name = fixtures.display_name(broken["election_event"])
        imported = self.admin.query(
            """query ($name: jsonb) {
              sequent_backend_election_event_aggregate(where: {presentation: {_contains: $name}}) { aggregate { count } }
            }""",
            {"name": {"i18n": {"en": {"name": name}}}},
        )["sequent_backend_election_event_aggregate"]["aggregate"]["count"]
        self.assertEqual(imported, 0)
        (task,) = self.import_tasks(started)
        self.assertEqual(task["execution_status"], "FAILED")
        self.assertIn(
            "points at a contest that is not in the bundle", json.dumps(task["logs"])
        )

    def test_3_import_voters(self):
        """Imported voters become enabled Keycloak users of the event realm, in their areas."""
        self.requires("journey 2")
        voters = fixtures.census(self.tag, {"A": 4, "B": 2, "C": 2})
        authorization = {
            "A": f"e2e-main-{self.tag}",
            "B": f"e2e-main-{self.tag}",
            "C": self.state.elections["grace"],
        }
        path = self.cli.directory / "voters.csv"
        fixtures.write_census(path, voters, authorization)
        self.step(
            "import-voters",
            "--election-event-id",
            self.state.event_id,
            "--file-path",
            str(path),
            "--is-local",
        )

        realm = event_realm(self.state.event_id)
        for voter in voters:
            user = self.keycloak.user(realm, voter.username)
            self.assertTrue(user["enabled"], voter.username)
            self.assertEqual(user["email"], f"{voter.username}@example.invalid")
            self.assertEqual(
                user["attributes"]["area-id"], [self.state.areas[voter.area]]
            )
            self.assertEqual(
                user["attributes"]["authorized-election-ids"],
                [authorization[voter.area]],
            )
            groups = [
                group["name"]
                for group in self.keycloak.admin(
                    "GET", f"{realm}/users/{user['id']}/groups"
                )
            ]
            self.assertEqual(groups, [ENV["KEYCLOAK_VOTER_GROUP_NAME"]])
        for key, area_id in self.state.areas.items():
            members = self.keycloak.admin(
                "GET",
                f"{realm}/users?q=area-id:{area_id}&briefRepresentation=true&max=100",
            )
            expected = sorted(v.username for v in voters if v.area == key)
            self.assertEqual(sorted(user["username"] for user in members), expected)
        self.state.voters = {
            key: [v.username for v in voters if v.area == key]
            for key in self.state.areas
        }
        self.passed("journey 3")

    def test_4_automatic_key_ceremony(self):
        """Two trustees generate the election key; it reaches the board and the ceremony succeeds."""
        self.requires("journey 2")
        listed = self.step("list-trustees")
        pks = {name: bootstrap.trustee_public_key(name) for name in bootstrap.TRUSTEES}
        for name, pk in pks.items():
            self.assertIn(f"name={name} public_key={pk}", listed)

        event_id = self.state.event_id
        output = self.step(
            "start-key-ceremony",
            "--election-event-id",
            event_id,
            "--threshold",
            "2",
            "--automatic",
        )
        ceremony_id = StepCli.last_id(output)

        def finished():
            bootstrap.configure_step_cli(self.cli)
            out = self.cli.step(
                "get-key-ceremony-status",
                "--election-event-id",
                event_id,
                "--key-ceremony-id",
                ceremony_id,
            )
            status = re.search(r"Keys Ceremony status: (\w+)", out).group(1)
            if status in ("FAILED", "CANCELLED"):
                raise AssertionError(f"Key ceremony {status}")
            return status == "SUCCESS"

        wait_until("the automatic key ceremony", finished, timeout=600, interval=5)

        data = self.event_rows(
            """query ($event: uuid!, $tenant: uuid!) {
              sequent_backend_keys_ceremony(where: {election_event_id: {_eq: $event}}) { id execution_status threshold settings status trustee_ids }
              sequent_backend_election(where: {election_event_id: {_eq: $event}}) { keys_ceremony_id }
              sequent_backend_trustee(where: {tenant_id: {_eq: $tenant}}) { id name }
            }""",
            tenant=TENANT_ID,
        )
        (ceremony,) = data["sequent_backend_keys_ceremony"]
        self.assertEqual(ceremony["id"], ceremony_id)
        self.assertEqual(ceremony["execution_status"], "SUCCESS")
        self.assertEqual(ceremony["threshold"], 2)
        self.assertEqual(ceremony["settings"], {"policy": "automated-ceremonies"})
        trustees = {t["name"]: t["id"] for t in data["sequent_backend_trustee"]}
        self.assertEqual(
            sorted(ceremony["trustee_ids"]),
            sorted(trustees[name] for name in bootstrap.TRUSTEES),
        )
        self.assertEqual(
            sorted((t["name"], t["status"]) for t in ceremony["status"]["trustees"]),
            [(name, "KEY_GENERATED") for name in bootstrap.TRUSTEES],
        )
        public_key = ceremony["status"]["public_key"]
        self.assertTrue(public_key)
        self.assertEqual(
            {e["keys_ceremony_id"] for e in data["sequent_backend_election"]},
            {ceremony_id},
        )

        messages = http_get(f"{B4_URL}/boards/{self.state.board}/messages/list").json()[
            "messages"
        ]
        senders = collections.defaultdict(set)
        for message in messages:
            senders[message["statement_kind"]].add(message["sender_pk"])
        self.assertEqual(senders["ConfigurationSigned"], set(pks.values()))
        # One trustee publishes the joint public key and the other signs it.
        self.assertEqual(
            senders["PublicKey"] | senders["PublicKeySigned"], set(pks.values())
        )
        self.assertEqual(len(senders["PublicKey"]), 1)

        self.wait_for_log(event_id, {"KeyGeneration": 1})
        self.state.public_key = public_key
        self.passed("journey 4")

    def test_5_publish_ballot_styles(self):
        """Publishing writes one private ballot style per area; each voter only gets their own."""
        self.requires("journey 3", "journey 4")
        event_id = self.state.event_id
        publication_id = StepCli.last_id(
            self.step("publish", "--election-event-id", event_id)
        )
        data = self.event_rows(
            """query ($event: uuid!) {
              sequent_backend_ballot_publication(where: {election_event_id: {_eq: $event}}) { id is_generated published_at annotations }
              sequent_backend_ballot_style(where: {election_event_id: {_eq: $event}, deleted_at: {_is_null: true}}) { id election_id area_id ballot_publication_id ballot_eml ballot_signature }
            }"""
        )
        (publication,) = data["sequent_backend_ballot_publication"]
        self.assertEqual(publication["id"], publication_id)
        self.assertTrue(publication["is_generated"])
        self.assertIsNotNone(publication["published_at"])
        root = publication["annotations"]["ballot_files_v1"]
        self.assertTrue(
            root.startswith(
                f"tenant-{TENANT_ID}/event-{event_id}/publication-{publication_id}/"
            ),
            root,
        )

        styles = {
            (s["election_id"], s["area_id"]): s["id"]
            for s in data["sequent_backend_ballot_style"]
        }
        expected = {
            (self.state.elections[spec.election], self.state.areas[spec.key])
            for spec in fixtures.AREAS
        }
        self.assertEqual(set(styles), expected)
        self.assertEqual(
            {s["ballot_publication_id"] for s in data["sequent_backend_ballot_style"]},
            {publication_id},
        )

        self.state.portal = Portal(event_id, self.cli)
        for spec in fixtures.AREAS:
            token = self.state.portal.login(self.state.voters[spec.key][0])
            status = self.state.portal.status(token)
            self.assertEqual(status["sequent_backend_cast_vote"], [])
            files = status["get_ballot_files_urls"]["files"]
            election_id = self.state.elections[spec.election]
            self.assertEqual(
                [(f["id"], f["election_id"]) for f in files],
                [(styles[(election_id, self.state.areas[spec.key])], election_id)],
            )
            raw_eml, wrapper = self.state.portal.ballot_eml(files[0])
            original = next(
                row
                for row in data["sequent_backend_ballot_style"]
                if row["id"] == files[0]["id"]
            )
            self.assertEqual(raw_eml, original["ballot_eml"])
            self.assertEqual(wrapper["ballot_signature"], original["ballot_signature"])
            style = json.loads(raw_eml)
            self.assertEqual(style["area_id"], self.state.areas[spec.key])
            self.assertEqual(
                [c["id"] for c in style["contests"]], [self.state.contests[spec.key]]
            )
            self.assertEqual(
                sorted(candidate_name(c) for c in style["contests"][0]["candidates"]),
                sorted(spec.candidates),
            )
            self.assertEqual(
                style["public_key"],
                {"public_key": self.state.public_key, "is_demo": False},
            )
            self.assertEqual(wrapper["id"], files[0]["id"])
            # Ballot files are private: the same object without a signature is refused.
            self.assertEqual(
                http_get(files[0]["urls"]["style_url"].split("?")[0]).status, 403
            )
        self.wait_for_log(event_id, {"ElectionPublish": 1})
        self.passed("journey 5")

    def test_5b_publish_event_with_translations(self):
        """An event whose presentation keeps the fixture's translations publishes too."""
        self.requires("journey 2")
        document = fixtures.election_event(ENV["VOTING_PORTAL_URL"], f"{self.tag}-i18n")
        event_id = self.import_event(document, "translated-event")
        publication_id = StepCli.last_id(
            self.step("publish", "--election-event-id", event_id)
        )
        styles = self.admin.query(
            """query ($event: uuid!, $publication: uuid!) {
              sequent_backend_ballot_style_aggregate(where: {election_event_id: {_eq: $event}, ballot_publication_id: {_eq: $publication}}) { aggregate { count } }
            }""",
            {"event": event_id, "publication": publication_id},
        )["sequent_backend_ballot_style_aggregate"]["aggregate"]["count"]
        self.assertEqual(styles, len(fixtures.AREAS))

    def test_6_online_voting_rules(self):
        """Voters vote and revote while voting is open; the revote limit, a second area and closing refuse votes."""
        self.requires("journey 5")
        event_id = self.state.event_id
        self.state.accepted, self.state.rejected = [], 0
        a1, a2, a3 = self.state.voters["A"][:3]
        (b1,) = self.state.voters["B"][:1]
        (c1,) = self.state.voters["C"][:1]

        self.step(
            "update-event-voting-status",
            "--election-event-id",
            event_id,
            "--voting-status",
            "OPEN",
            "--voting-channel",
            "ONLINE",
        )
        statuses = self.event_rows(
            """query ($event: uuid!) {
              sequent_backend_election_event(where: {id: {_eq: $event}}) { status }
              sequent_backend_election(where: {election_event_id: {_eq: $event}}) { status }
            }"""
        )
        self.assertEqual(
            statuses["sequent_backend_election_event"][0]["status"]["voting_status"],
            "OPEN",
        )
        self.assertEqual(
            {
                e["status"]["voting_status"]
                for e in statuses["sequent_backend_election"]
            },
            {"OPEN"},
        )

        # A vote and a revote are accepted; a third vote exceeds the limit.
        token = self.state.portal.login(a1)
        receipt, errors = self.vote(a1, "Alice", token)
        self.assertIsNone(errors)
        self.assertEqual(
            (receipt["area_id"], receipt["election_event_id"]),
            (self.state.areas["A"], event_id),
        )
        self.assertIsNone(self.vote(a1, "Bob", token)[1])
        self.assert_rejected(
            self.vote(a1, "Carol", token)[1],
            "InsertFailedExceedsAllowedRevotes",
            "InsertFailedExceedsAllowedRevotes",
        )
        own = self.state.portal.status(token)["sequent_backend_cast_vote"]
        self.assertEqual(len(own), fixtures.MAIN_ALLOWED_VOTES)

        # A voter who voted in area A cannot vote in the same election from area B.
        self.assertIsNone(self.vote(a2, "Alice")[1])
        user = self.keycloak.user(event_realm(event_id), a2)
        self.step(
            "update-voter",
            "--election-event-id",
            event_id,
            "--user-id",
            user["id"],
            "--area-id",
            self.state.areas["B"],
        )
        token = self.state.portal.login(a2)
        self.assertEqual(
            jwt_claims(token)["https://hasura.io/jwt/claims"]["x-hasura-area-id"],
            self.state.areas["B"],
        )
        self.assert_rejected(
            self.vote(a2, "Dave", token)[1],
            "CheckVotesInOtherAreasFailed",
            "Cannot insert cast vote, votes already present in other area(s)",
        )
        # The tally only counts voters in the area of their ballot; move a2 back.
        self.step(
            "update-voter",
            "--election-event-id",
            event_id,
            "--user-id",
            user["id"],
            "--area-id",
            self.state.areas["A"],
        )

        self.assertIsNone(self.vote(b1, "Erin")[1])
        self.state.c1_token = self.state.portal.login(c1)
        self.assertIsNone(self.vote(c1, "Frank", self.state.c1_token)[1])

        self.step(
            "update-event-voting-status",
            "--election-event-id",
            event_id,
            "--voting-status",
            "CLOSED",
            "--voting-channel",
            "ONLINE",
        )
        # Tokens carry whole seconds; observe a session newer than the close.
        self.state.closed_before = datetime.now(timezone.utc).timestamp()

        def session_after_close():
            token = self.state.portal.login(a3)
            return (
                token
                if jwt_claims(token)["auth_time"] > self.state.closed_before
                else None
            )

        token_after_close = wait_until(
            "a login strictly after closing",
            session_after_close,
            timeout=10,
            interval=0.2,
        )
        self.assert_rejected(
            self.vote(a3, "Carol", token_after_close)[1],
            "CheckStatusFailed",
            "Voting Status for voting_channel=ONLINE is CLOSED",
        )

        self.assertEqual(len(self.voter_votes(a1)), fixtures.MAIN_ALLOWED_VOTES)
        self.assertEqual(
            [(v["area_id"], v["status"]) for v in self.voter_votes(a2)],
            [(self.state.areas["A"], "valid")],
        )
        self.assertEqual(self.voter_votes(a3), [])
        self.wait_for_log(
            event_id,
            {
                "ElectionEventVotingPeriodOpen": 1,
                "ElectionEventVotingPeriodClose": 1,
                "CastVote": len(self.state.accepted),
                "CastVoteError": self.state.rejected,
            },
        )
        self.passed("journey 6")

    def test_6b_grace_period_admits_only_earlier_sessions(self):
        """In the grace period, a session opened before closing still votes; a newer one does not."""
        self.requires("journey 6")
        c1, c2 = self.state.voters["C"][:2]
        self.assertIsNone(self.vote(c1, "Grace", self.state.c1_token)[1])
        newer_token = self.state.portal.login(c2)
        self.assertGreater(
            jwt_claims(newer_token)["auth_time"], self.state.closed_before
        )
        errors = self.vote(c2, "Frank", newer_token)[1]
        self.assert_rejected(
            errors,
            "CheckStatusFailed",
            "Voting Status for voting_channel=ONLINE is CLOSED",
        )
        self.assertEqual(self.voter_votes(c2), [])

    def tally(self):
        """Run one electoral-results tally for the event and return its results event ID."""
        if getattr(self.state, "results_event_id", None):
            return self.state.results_event_id
        event_id = self.state.event_id
        tally_id = StepCli.last_id(
            self.step(
                "start-tally",
                "--election-event-id",
                event_id,
                "--tally-type",
                "ELECTORAL_RESULTS",
            )
        )

        def finished():
            (session,) = self.admin.query(
                "query ($id: uuid!) { sequent_backend_tally_session(where: {id: {_eq: $id}}) { execution_status } }",
                {"id": tally_id},
            )["sequent_backend_tally_session"]
            if session["execution_status"] in ("FAILED", "CANCELLED"):
                raise AssertionError(f"Tally {session['execution_status']}")
            return session["execution_status"] == "SUCCESS"

        wait_until("the electoral results tally", finished, timeout=900, interval=5)
        executions = self.admin.query(
            """query ($id: uuid!) {
              sequent_backend_tally_session_execution(where: {tally_session_id: {_eq: $id}, results_event_id: {_is_null: false}},
                                                      order_by: {current_message_id: desc}, limit: 1) { results_event_id }
            }""",
            {"id": tally_id},
        )["sequent_backend_tally_session_execution"]
        self.assertEqual(len(executions), 1, "the tally stored no results")
        self.state.results_event_id = executions[0]["results_event_id"]
        return self.state.results_event_id

    def results(self, key):
        """(contest totals, candidate votes by name, area-contest candidate votes by name) for an area's contest."""
        data = self.admin.query(
            """query ($results: uuid!, $contest: uuid!, $area: uuid!) {
              sequent_backend_results_contest(where: {results_event_id: {_eq: $results}, contest_id: {_eq: $contest}}) { total_votes total_valid_votes }
              sequent_backend_results_contest_candidate(where: {results_event_id: {_eq: $results}, contest_id: {_eq: $contest}}) { candidate_id cast_votes }
              sequent_backend_results_area_contest_candidate(where: {results_event_id: {_eq: $results}, contest_id: {_eq: $contest}, area_id: {_eq: $area}}) { candidate_id cast_votes }
            }""",
            {
                "results": self.state.results_event_id,
                "contest": self.state.contests[key],
                "area": self.state.areas[key],
            },
        )
        names = {
            candidate_id: name
            for name, candidate_id in self.state.candidates[key].items()
        }
        (contest,) = data["sequent_backend_results_contest"]
        by_name = {
            names[r["candidate_id"]]: r["cast_votes"]
            for r in data["sequent_backend_results_contest_candidate"]
        }
        by_area = {
            names[r["candidate_id"]]: r["cast_votes"]
            for r in data["sequent_backend_results_area_contest_candidate"]
        }
        return contest, by_name, by_area

    def expected(self, key):
        """Votes per candidate from each voter's last accepted ballot in the area's contest."""
        final = {}
        for voter, area, contest, candidate in self.state.accepted:
            if contest == self.state.contests[key]:
                final[voter] = candidate
        counts = collections.Counter(final.values())
        return {name: counts[name] for name in fixtures.AREA[key].candidates}

    def test_7_tally_matches_cast_ballots(self):
        """After closing, the tally counts each voter's last accepted ballot, per contest and area."""
        self.requires("journey 6")
        self.tally()
        for key in ("A", "B"):
            expected = self.expected(key)
            contest, by_name, by_area = self.results(key)
            self.assertEqual(by_name, expected, f"Contest {key} results")
            self.assertEqual(by_area, expected, f"Contest {key} results in area {key}")
            self.assertEqual(contest["total_valid_votes"], sum(expected.values()))
            self.assertEqual(contest["total_votes"], sum(expected.values()))
        self.wait_for_log(self.state.event_id, {"TallyClose": 1})
        self.passed("journey 7")

    def test_7b_tally_counts_voters_listed_by_election_id(self):
        """Voters authorized by election ID have their accepted ballots counted."""
        self.requires("journey 6")
        self.tally()
        expected = self.expected("C")
        self.assertTrue(sum(expected.values()))
        contest, by_name, by_area = self.results("C")
        self.assertEqual(by_name, expected, "Contest C results")
        self.assertEqual(by_area, expected, "Contest C results in area C")
        self.assertEqual(contest["total_valid_votes"], sum(expected.values()))
