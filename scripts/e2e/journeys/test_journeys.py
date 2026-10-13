# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Backend journeys, run in order against one fresh stack (scripts/e2e/run.sh).

Each journey builds on the previous ones and is skipped when one it needs did
not pass. Skipped journeys fail the run; every scenario must pass.
"""

import base64
import collections
import copy
import hashlib
import json
import re
import subprocess
import unittest
import uuid
import zipfile
from datetime import datetime, timezone
from pathlib import Path

from . import bootstrap, fixtures
from .client import (
    B4_URL,
    ENV,
    OUTPUT,
    ROOT,
    S3_URL,
    TENANT_ID,
    TENANT_REALM,
    Hasura,
    GraphQLError,
    Http,
    Keycloak,
    StepCli,
    StepCliError,
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

    def verify_public_transcripts(self, event_id, ballot_ciphertexts=None):
        """An anonymous Chromium observer verifies all real public proofs."""
        controller = ENV.get("STEP_E2E_BROWSER_TRUSTEES_URL") or ENV.get(
            "STEP_E2E_SERVER_CUSTODY_BROWSER_URL"
        )
        if not controller:
            return
        boards = self.admin.query(
            """query ($event: uuid!) {
              sequent_backend_protocol_board(where: {election_event_id: {_eq: $event}}) {
                id name parent_id manager_message
              }
            }""",
            {"event": event_id},
        )["sequent_backend_protocol_board"]
        parents = {b["id"]: b for b in boards if b["parent_id"] is None}
        self.assertTrue(parents)
        children = [b for b in boards if b["parent_id"] is not None]
        self.assertTrue(children)
        inclusion_checked = False

        def encoded(message):
            self.assertTrue(message.startswith("\\x"))
            return base64.b64encode(bytes.fromhex(message[2:])).decode()

        for child in children:
            parent = parents[child["parent_id"]]
            anchors = {
                "tally_id": str(uuid.UUID(child["id"]).int),
                "ballot_ciphertexts": [],
            }
            result = Http().request(
                "POST",
                controller + "/verify",
                json_body={
                    "boardUrl": B4_URL,
                    "dkgBoard": parent["name"],
                    "tallyBoard": child["name"],
                    "configurationMessage": encoded(parent["manager_message"]),
                    "ballotsMessage": encoded(child["manager_message"]),
                    "anchors": anchors,
                },
            )
            self.assertEqual(result.status, 200, result.text)
            result = result.json()
            self.assertEqual(result["report"]["status"], "verified", result["report"])
            self.assertEqual(
                result["report"]["ballot_count"], len(result["report"]["plaintexts"])
            )
            self.assertEqual(result["wrongAnchor"]["status"], "invalid")
            self.assertEqual(result["truncated"]["status"], "incomplete")
            if ballot_ciphertexts and result["report"]["ballot_count"]:
                anchors["ballot_ciphertexts"] = ballot_ciphertexts
                response = Http().request(
                    "POST",
                    controller + "/verify",
                    json_body={
                        "boardUrl": B4_URL,
                        "dkgBoard": parent["name"],
                        "tallyBoard": child["name"],
                        "configurationMessage": encoded(parent["manager_message"]),
                        "ballotsMessage": encoded(child["manager_message"]),
                        "anchors": anchors,
                    },
                )
                self.assertEqual(response.status, 200, response.text)
                result = response.json()
                self.assertEqual(
                    result["report"]["status"], "verified", result["report"]
                )
                self.assertTrue(
                    all(
                        b["occurrences"] == 1
                        for b in result["report"]["ballot_inclusions"]
                    )
                )
                inclusion_checked = True
            evidence = OUTPUT / "public-verifier" / child["name"]
            evidence.mkdir(parents=True, exist_ok=True)
            for name in ("transcript", "anchors", "report"):
                (evidence / f"{name}.json").write_text(
                    json.dumps(result[name], indent=2)
                )
        if ballot_ciphertexts:
            self.assertTrue(
                inclusion_checked,
                "the cast ballot must occur in a nonempty verified batch",
            )

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
        """The voter's own cast votes in the event's ballot box, in the order cast."""
        token = self.state.portal.login(username)
        return Hasura(token=token).query(
            """query ($event: String!) {
              get_voter_cast_votes(election_event_id: $event) { area_id election_id status }
            }""",
            {"event": self.state.event_id},
        )["get_voter_cast_votes"]

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
              sequent_backend_protocol_board(where: {election_event_id: {_eq: $event}}) { id }
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

        self.assertTrue(event["bulletin_board_reference"]["database_name"])
        # Protocol boards are created by the native key ceremony, after import.
        self.assertEqual(data["sequent_backend_protocol_board"], [])

        self.assertEqual(
            self.keycloak.admin("GET", event_realm(event_id))["realm"],
            event_realm(event_id),
        )
        # The event's electoral log starts empty. An event without an electoral-log
        # database reads as empty too, never as another event's records.
        self.assertEqual(self.electoral_log(event_id), collections.Counter())
        self.assertEqual(self.electoral_log(str(uuid.uuid4())), collections.Counter())
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
              sequent_backend_protocol_board(where: {election_event_id: {_eq: $event}, parent_id: {_is_null: true}}) { name keys_ceremony_id manager_message }
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

        (board,) = data["sequent_backend_protocol_board"]
        self.assertEqual(board["keys_ceremony_id"], ceremony_id)
        self.assertTrue(board["manager_message"])
        self.assertTrue(ceremony["status"]["public_key_hash"])
        self.state.board = board["name"]
        messages = http_get(f"{B4_URL}/boards/{self.state.board}/messages/list").json()[
            "messages"
        ]
        self.assertGreater(len(messages), len(bootstrap.TRUSTEES))
        self.assertEqual({message["version"] for message in messages}, {"2"})

        self.wait_for_log(event_id, {"KeyGeneration": 1})
        self.state.public_key = public_key
        self.state.keys_ceremony_id = ceremony_id
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
            self.assertEqual(status["cast_votes"], [])
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
                {
                    "public_key": self.state.public_key,
                    "is_demo": False,
                    "keys_ceremony_id": self.state.keys_ceremony_id,
                    "configuration_id": str(uuid.UUID(self.state.keys_ceremony_id).int),
                },
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
        """Voters vote and revote while voting is open; the revote limit, an area change and closing are refused."""
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
        own = self.state.portal.status(token)["cast_votes"]
        self.assertEqual(len(own), fixtures.MAIN_ALLOWED_VOTES)

        # A voter who has voted cannot be moved to area B, so they stay in area A.
        self.assertIsNone(self.vote(a2, "Alice")[1])
        user = self.keycloak.user(event_realm(event_id), a2)
        with self.assertRaisesRegex(
            StepCliError, "Cannot change area-id of a voter who has cast a ballot"
        ):
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
        self.verify_public_transcripts(self.state.event_id)
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

    def test_8_native_archive_preserves_crypto_and_results(self):
        """Remapped archive recovery preserves signed messages, key domains and results."""
        self.requires("journey 7")
        query = """query ($event: uuid!) {
          sequent_backend_protocol_board(where: {election_event_id: {_eq: $event}}) { id name parent_id keys_ceremony_id manager_message }
          sequent_backend_keys_ceremony(where: {election_event_id: {_eq: $event}}) { id execution_status status }
          sequent_backend_ballot_style(where: {election_event_id: {_eq: $event}, deleted_at: {_is_null: true}}) { ballot_eml }
          sequent_backend_ballot_publication(where: {election_event_id: {_eq: $event}, deleted_at: {_is_null: true}}) { id is_generated published_at annotations }
          sequent_backend_results_contest(where: {election_event_id: {_eq: $event}}) { total_votes total_valid_votes }
        }"""
        source = self.event_rows(query)
        source_boards = source["sequent_backend_protocol_board"]
        self.assertGreater(len(source_boards), 1, "expected both DKG and tally boards")

        def message_hashes(name):
            response = http_get(f"{B4_URL}/boards/{name}/messages")
            self.assertEqual(response.status, 200)
            hashes = collections.Counter()
            for message in response.json()["messages"]:
                self.assertEqual(message["version"], "2")
                if message.get("download_url"):
                    body = http_get(message["download_url"])
                    self.assertEqual(body.status, 200)
                    canonical = body.body
                else:
                    canonical = base64.b64decode(
                        message["content_type"]["message"], validate=True
                    )
                hashes[hashlib.sha256(canonical).hexdigest()] += 1
            self.assertTrue(
                hashes, "the recovered board must contain signed protocol messages"
            )
            return hashes

        original_messages = {
            board["manager_message"]: message_hashes(board["name"])
            for board in source_boards
        }
        archive_dir = self.cli.directory / "archive"
        archive_dir.mkdir()
        output = self.step(
            "export-election-event",
            "--election-event-id",
            self.state.event_id,
            "--bulletin-board",
            "--publications",
            "--tally",
            "--include-voters",
            "--encrypted",
            "--output-dir",
            str(archive_dir),
        )
        password = re.search(r"Export password: (\S+)", output)
        self.assertIsNotNone(
            password, "encrypted native export must return its password"
        )
        encrypted = archive_dir / "election_event_export.ezip"
        self.assertTrue(encrypted.is_file())
        self.assertFalse(
            zipfile.is_zipfile(encrypted), "native manager keys must be encrypted"
        )
        archive = archive_dir / "recovered.zip"
        decrypted = subprocess.run(
            [
                "openssl",
                "enc",
                "-aes-256-cbc",
                "-d",
                "-in",
                str(encrypted),
                "-out",
                str(archive),
                "-pass",
                "stdin",
                "-md",
                "md5",
            ],
            input=password.group(1) + "\n",
            capture_output=True,
            text=True,
            timeout=30,
        )
        self.assertEqual(decrypted.returncode, 0, decrypted.stderr)
        self.assertTrue(zipfile.is_zipfile(archive))
        recovered_id = StepCli.last_id(
            self.step("import-election", "--file-path", str(archive), "--is-local")
        )
        self.assertNotEqual(recovered_id, self.state.event_id)
        recovered = self.admin.query(query, {"event": recovered_id})
        recovered_boards = recovered["sequent_backend_protocol_board"]
        self.assertEqual(len(recovered_boards), len(source_boards))
        self.assertEqual(
            {b["manager_message"] for b in recovered_boards}, set(original_messages)
        )
        self.assertTrue(
            {b["name"] for b in source_boards}.isdisjoint(
                b["name"] for b in recovered_boards
            )
        )
        for board in recovered_boards:
            self.assertEqual(
                message_hashes(board["name"]),
                original_messages[board["manager_message"]],
            )
            if board["parent_id"]:
                self.assertIn(
                    board["parent_id"],
                    {b["id"] for b in recovered_boards if b["parent_id"] is None},
                )
        (ceremony,) = recovered["sequent_backend_keys_ceremony"]
        self.assertNotEqual(ceremony["id"], self.state.keys_ceremony_id)
        self.assertEqual(ceremony["execution_status"], "SUCCESS")
        self.assertEqual(ceremony["status"]["public_key"], self.state.public_key)
        self.assertEqual(
            {b["keys_ceremony_id"] for b in recovered_boards}, {ceremony["id"]}
        )
        styles = recovered["sequent_backend_ballot_style"]
        self.assertEqual(len(styles), len(fixtures.AREAS))
        for style in styles:
            public_key = json.loads(style["ballot_eml"])["public_key"]
            self.assertEqual(public_key["public_key"], self.state.public_key)
            self.assertEqual(public_key["keys_ceremony_id"], ceremony["id"])
            # Signed protocol domains survive platform UUID and board-name remapping.
            self.assertEqual(
                public_key["configuration_id"],
                str(uuid.UUID(self.state.keys_ceremony_id).int),
            )
        (original_publication,) = source["sequent_backend_ballot_publication"]
        (publication,) = recovered["sequent_backend_ballot_publication"]
        self.assertNotEqual(publication["id"], original_publication["id"])
        self.assertTrue(publication["is_generated"])
        # Import preserves verified key material and generated ballots, but
        # resets the live publication gate for the new event.
        self.assertTrue(original_publication["published_at"])
        self.assertIsNone(publication["published_at"])
        self.assertTrue(
            publication["annotations"]["ballot_files_v1"].startswith(
                f"tenant-{TENANT_ID}/event-{recovered_id}/publication-{publication['id']}/"
            )
        )
        expected = sorted(sum(self.expected(key).values()) for key in ("A", "B", "C"))
        results = recovered["sequent_backend_results_contest"]
        self.assertEqual(sorted(r["total_votes"] for r in results), expected)
        self.assertEqual(sorted(r["total_valid_votes"] for r in results), expected)
        self.state.encrypted_archive = encrypted
        self.state.archive_password = password.group(1)
        self.state.archive_board_count = len(source_boards)
        self.passed("journey 8")

    def test_9_encrypted_native_archive_import(self):
        """The authenticated import API decrypts and restores a complete native archive."""
        self.requires("journey 8")
        api = Hasura(token=self.admin_token())
        contents = self.state.encrypted_archive.read_bytes()
        upload = api.query(
            """mutation ($size: Int!) {
              get_upload_url(name: "native-recovery.ezip", media_type: "application/ezip",
                             size: $size, is_public: false, is_local: true) { url document_id }
            }""",
            {"size": len(contents)},
        )["get_upload_url"]
        response = Http().request(
            "PUT",
            upload["url"],
            data=contents,
            headers={"Content-Type": "application/ezip"},
        )
        self.assertEqual(response.status, 200)
        imported = api.query(
            """mutation ($tenant: String!, $document: String!, $password: String!) {
              import_election_event(tenant_id: $tenant, document_id: $document, password: $password) {
                id error problems task_execution { id }
              }
            }""",
            {
                "tenant": TENANT_ID,
                "document": upload["document_id"],
                "password": self.state.archive_password,
            },
        )["import_election_event"]
        self.assertIsNone(imported["error"], imported)
        self.assertTrue(imported["id"], imported)
        task_id = imported["task_execution"]["id"]

        def complete():
            task = self.admin.query(
                """query ($tenant: uuid!, $id: uuid!) {
                  sequent_backend_tasks_execution_by_pk(tenant_id: $tenant, id: $id) { execution_status logs }
                }""",
                {"tenant": TENANT_ID, "id": task_id},
            )["sequent_backend_tasks_execution_by_pk"]
            if task["execution_status"] in ("SUCCESS", "FAILED"):
                return task
            return None

        task = wait_until("encrypted native archive recovery", complete, timeout=180)
        self.assertEqual(task["execution_status"], "SUCCESS", task["logs"])
        recovered = self.admin.query(
            """query ($event: uuid!) {
              sequent_backend_protocol_board_aggregate(where: {election_event_id: {_eq: $event}}) { aggregate { count } }
              sequent_backend_keys_ceremony(where: {election_event_id: {_eq: $event}}) { execution_status status }
              sequent_backend_results_contest(where: {election_event_id: {_eq: $event}}) { total_votes total_valid_votes }
              sequent_backend_ballot_style(where: {election_event_id: {_eq: $event}, deleted_at: {_is_null: true}}) { ballot_eml }
              sequent_backend_ballot_publication(where: {election_event_id: {_eq: $event}, deleted_at: {_is_null: true}}) { is_generated published_at }
            }""",
            {"event": imported["id"]},
        )
        self.assertNotEqual(imported["id"], self.state.event_id)
        self.assertEqual(
            recovered["sequent_backend_protocol_board_aggregate"]["aggregate"]["count"],
            self.state.archive_board_count,
        )
        (ceremony,) = recovered["sequent_backend_keys_ceremony"]
        self.assertEqual(ceremony["execution_status"], "SUCCESS")
        self.assertEqual(ceremony["status"]["public_key"], self.state.public_key)
        self.assertEqual(
            len(recovered["sequent_backend_ballot_style"]), len(fixtures.AREAS)
        )
        (publication,) = recovered["sequent_backend_ballot_publication"]
        self.assertTrue(publication["is_generated"])
        # Encrypted API import applies the same publication reset as CLI import.
        self.assertIsNone(publication["published_at"])
        expected = sorted(sum(self.expected(key).values()) for key in ("A", "B", "C"))
        results = recovered["sequent_backend_results_contest"]
        self.assertEqual(sorted(r["total_votes"] for r in results), expected)
        self.assertEqual(sorted(r["total_valid_votes"] for r in results), expected)
        self.passed("journey 9")

    def test_a_combined_contest_weighted_voting(self):
        """Native combined ballots pass admission, weight batches and Velvet counting."""
        self.requires("journey 4")
        document = fixtures.election_event(
            ENV["VOTING_PORTAL_URL"], self.tag + "-combined"
        )
        document["election_event"]["presentation"].update(
            contest_encryption_policy="multiple-contests",
            weighted_voting_policy="voters-weighted-voting",
            batch_anonymity_policy="warn",
        )
        # Area A receives both main-election contests in one ciphertext.
        document["area_contests"].append(
            {
                "id": str(uuid.uuid4()),
                "area_id": document["areas"][0]["id"],
                "contest_id": document["contests"][1]["id"],
            }
        )
        event_id = self.import_event(document, "combined-weighted")
        voters = fixtures.census(self.tag + "-combined", {"A": 2})
        path = self.cli.directory / "combined-voters.csv"
        fixtures.write_census(
            path,
            voters,
            {"A": f"e2e-main-{self.tag}-combined"},
            weights={voter.username: weight for voter, weight in zip(voters, (3, 5))},
        )
        self.step(
            "import-voters",
            "--election-event-id",
            event_id,
            "--file-path",
            str(path),
            "--is-local",
        )
        ceremony = StepCli.last_id(
            self.step(
                "start-key-ceremony",
                "--election-event-id",
                event_id,
                "--threshold",
                "2",
                "--automatic",
            )
        )

        def keys_ready():
            (row,) = self.admin.query(
                "query ($id: uuid!) { sequent_backend_keys_ceremony(where: {id: {_eq: $id}}) { execution_status } }",
                {"id": ceremony},
            )["sequent_backend_keys_ceremony"]
            self.assertNotIn(row["execution_status"], ("FAILED", "CANCELLED"))
            return row["execution_status"] == "SUCCESS"

        wait_until("combined ballot native keys", keys_ready, timeout=600)
        self.step("publish", "--election-event-id", event_id)
        self.step(
            "update-event-voting-status",
            "--election-event-id",
            event_id,
            "--voting-status",
            "OPEN",
            "--voting-channel",
            "ONLINE",
        )
        portal = Portal(event_id, self.cli)
        for index, voter in enumerate(voters):
            token = portal.login(voter.username)
            (file,) = portal.status(token)["get_ballot_files_urls"]["files"]
            style, _ = portal.ballot_style(file)
            self.assertEqual(len(style["contests"]), 2)
            self.assertEqual(
                style["election_event_presentation"]["contest_encryption_policy"],
                "multiple-contests",
            )
            self.assertEqual(
                style["public_key"]["configuration_id"], str(uuid.UUID(ceremony).int)
            )
            selections = {
                contest["id"]: (
                    ("Alice", "Bob")[index]
                    if any(
                        candidate_name(candidate) == "Alice"
                        for candidate in contest["candidates"]
                    )
                    else ("Dave", "Erin")[index]
                )
                for contest in style["contests"]
            }
            ballot = portal.encrypt(style, selections)
            content = json.loads(ballot["content"])
            self.assertIsInstance(content["contests"], str)
            if index == 0:
                malformed = copy.deepcopy(ballot)
                broken = copy.deepcopy(content)
                broken["contests"] = base64.b64encode(
                    base64.b64decode(broken["contests"])[:-1]
                ).decode()
                malformed["content"] = json.dumps(broken)
                receipt, errors = portal.cast(token, malformed)
                self.assertIsNone(receipt)
                self.assertEqual(
                    [e["extensions"]["code"] for e in errors],
                    ["DeserializeContestsFailed"],
                )
                # Keep the published envelope but substitute a valid native
                # ciphertext encrypted under another configuration domain.
                foreign_style = copy.deepcopy(style)
                foreign_style["public_key"]["configuration_id"] = str(
                    int(style["public_key"]["configuration_id"]) + 1
                )
                foreign = portal.encrypt(foreign_style, selections)
                substituted = copy.deepcopy(ballot)
                other_content = copy.deepcopy(content)
                other_content["contests"] = json.loads(foreign["content"])["contests"]
                substituted["content"] = json.dumps(other_content)
                receipt, errors = portal.cast(token, substituted)
                self.assertIsNone(receipt)
                self.assertEqual(
                    [e["extensions"]["code"] for e in errors], ["PokValidationFailed"]
                )
                stored = Hasura(token=token).query(
                    "query ($event: String!) { get_voter_cast_votes(election_event_id: $event) { area_id election_id status } }",
                    {"event": event_id},
                )["get_voter_cast_votes"]
                self.assertEqual(stored, [])
            receipt, errors = portal.cast(token, ballot)
            self.assertIsNone(errors)
            self.assertEqual(receipt["ballot_id"], ballot["ballotId"])
        self.step(
            "update-event-voting-status",
            "--election-event-id",
            event_id,
            "--voting-status",
            "CLOSED",
            "--voting-channel",
            "ONLINE",
        )
        previous = self.state
        self.state = State()
        self.state.event_id = event_id
        try:
            results_id = self.tally()
        finally:
            self.state = previous
        results = self.admin.query(
            """query ($results: uuid!, $event: uuid!) {
              sequent_backend_results_contest(where: {results_event_id: {_eq: $results}}) { contest_id total_votes total_valid_votes }
              sequent_backend_results_contest_candidate(where: {results_event_id: {_eq: $results}}) { contest_id candidate_id cast_votes }
              sequent_backend_candidate(where: {election_event_id: {_eq: $event}}) { id contest_id presentation }
            }""",
            {"results": results_id, "event": event_id},
        )
        active_contests = {contest["id"] for contest in style["contests"]}
        counted = [
            r
            for r in results["sequent_backend_results_contest"]
            if r["contest_id"] in active_contests
        ]
        self.assertEqual(len(counted), 2)
        self.assertEqual(
            [(r["total_votes"], r["total_valid_votes"]) for r in counted],
            [(8, 8), (8, 8)],
        )
        names = {
            c["id"]: fixtures.display_name(c)
            for c in results["sequent_backend_candidate"]
        }
        by_name = {
            names[r["candidate_id"]]: r["cast_votes"]
            for r in results["sequent_backend_results_contest_candidate"]
            if r["contest_id"] in active_contests
        }
        self.assertEqual(
            by_name, {"Alice": 3, "Bob": 5, "Carol": 0, "Dave": 3, "Erin": 5}
        )
        self.verify_public_transcripts(event_id)

    def test_b_manual_key_ceremony_and_tally(self):
        """Each trustee proves local custody before publication and a manual tally."""
        document = fixtures.election_event(
            ENV["VOTING_PORTAL_URL"], self.tag + "-manual"
        )
        document["election_event"]["presentation"].update(
            ceremonies_policy="manual-ceremonies", batch_anonymity_policy="warn"
        )
        event_id = self.import_event(document, "manual-custody")
        ceremony_id = StepCli.last_id(
            self.step(
                "start-key-ceremony",
                "--election-event-id",
                event_id,
                "--threshold",
                "2",
            )
        )

        def ceremony():
            (row,) = self.admin.query(
                "query ($id: uuid!) { sequent_backend_keys_ceremony(where: {id: {_eq: $id}}) { execution_status settings status } }",
                {"id": ceremony_id},
            )["sequent_backend_keys_ceremony"]
            self.assertNotIn(row["execution_status"], ("FAILED", "CANCELLED"), row)
            return row

        def generated():
            row = ceremony()
            return row if row["status"].get("public_key_hash") else None

        # Optional real-browser participants use the same journey and independent
        # vote/result oracle. The owned native daemons must be stopped by the runner.
        browser_url = ENV.get("STEP_E2E_BROWSER_TRUSTEES_URL")
        server_browser_url = ENV.get("STEP_E2E_SERVER_CUSTODY_BROWSER_URL")

        def server_browser_call(operation, name, tally_id=None):
            # Use the repository's synthetic OTP mode only during real UI login;
            # restore the realm's actual configuration even on browser failure.
            config = bootstrap.email_otp_config(self.keycloak)
            original = config["config"].get("test-mode", "false")
            config["config"]["test-mode"] = "true"
            self.keycloak.admin(
                "PUT",
                f"{TENANT_REALM}/authentication/config/{config['id']}",
                config,
                expect=(204,),
            )
            try:
                response = Http().post(
                    server_browser_url + operation,
                    json_body={
                        "name": name,
                        "password": ENV[f"{name.upper()}_PASSWORD"],
                        "eventId": event_id,
                        "ceremonyId": ceremony_id,
                        "tallyId": tally_id,
                        "otp": config["config"].get("test-mode-code", "123456"),
                    },
                    timeout=180,
                )
                self.assertEqual(response.status, 200, response.body.decode())
                return response.json()
            finally:
                config["config"]["test-mode"] = original
                self.keycloak.admin(
                    "PUT",
                    f"{TENANT_REALM}/authentication/config/{config['id']}",
                    config,
                    expect=(204,),
                )

        if server_browser_url:
            health = Http().get(server_browser_url + "/health").json()
            admin_origin = health["adminOrigin"]
            clients = self.keycloak.admin(
                "GET", f"{TENANT_REALM}/clients?clientId=admin-portal"
            )
            self.assertEqual(len(clients), 1)
            client = clients[0]
            client["redirectUris"] = [
                *client.get("redirectUris", []),
                admin_origin + "/*",
            ]
            client["webOrigins"] = [*client.get("webOrigins", []), admin_origin]
            self.keycloak.admin(
                "PUT", f"{TENANT_REALM}/clients/{client['id']}", client, expect=(204,)
            )

        def browser_call(operation, **body):
            response = Http().post(browser_url + operation, json_body=body)
            self.assertEqual(response.status, 200, response.body.decode())
            return response.json()

        if browser_url:
            for name in bootstrap.TRUSTEES:
                token = self.keycloak.password_grant(
                    TENANT_REALM,
                    "api-key-client",
                    name,
                    ENV[f"{name.upper()}_PASSWORD"],
                    ENV["API_KEY_CLIENT_SECRET"],
                )["access_token"]
                browser_call(
                    "/start",
                    name=name,
                    keys=(ROOT / f"packages/trustee/dev/{name}.toml").read_text(),
                    boardUrl=B4_URL,
                    platform=ENV["HASURA_ENDPOINT"],
                    token=token,
                )

        ready = wait_until(
            "manual native joint key verification", generated, timeout=600
        )
        self.assertEqual(ready["execution_status"], "IN_PROGRESS")
        self.assertEqual(ready["settings"]["policy"], "manual-ceremonies")
        self.assertEqual(
            {t["status"] for t in ready["status"]["trustees"]}, {"KEY_GENERATED"}
        )
        with self.assertRaisesRegex(
            StepCliError, "must complete before ballot publication"
        ):
            self.step("publish", "--election-event-id", event_id)
        publications = self.admin.query(
            "query ($event: uuid!) { sequent_backend_ballot_publication(where: {election_event_id: {_eq: $event}}) { published_at } }",
            {"event": event_id},
        )["sequent_backend_ballot_publication"]
        self.assertTrue(publications)
        self.assertTrue(all(p["published_at"] is None for p in publications))
        joint_key = ready["status"]["public_key"]
        trustee_clients = {}
        requests = {}
        apis = {}
        for name in bootstrap.TRUSTEES:
            cli = StepCli()
            cli.step(
                "config",
                "--tenant-id",
                TENANT_ID,
                "--endpoint-url",
                ENV["HASURA_ENDPOINT"],
                "--keycloak-url",
                ENV["KEYCLOAK_URL"],
                "--keycloak-user",
                name,
                "--keycloak-password",
                ENV[f"{name.upper()}_PASSWORD"],
                "--keycloak-client-id",
                "api-key-client",
                "--keycloak-client-secret",
                ENV["API_KEY_CLIENT_SECRET"],
            )
            trustee_clients[name] = cli
            token = self.keycloak.password_grant(
                TENANT_REALM,
                "api-key-client",
                name,
                ENV[f"{name.upper()}_PASSWORD"],
                ENV["API_KEY_CLIENT_SECRET"],
            )["access_token"]
            self.assertEqual(jwt_claims(token)["trustee"], name)
            apis[name] = Hasura(token=token, role="trustee-ceremony")
            request = apis[name].query(
                """mutation ($event: String!, $ceremony: String!) {
                  get_custody_request(object: {election_event_id: $event, keys_ceremony_id: $ceremony}) { custody_request_json }
                }""",
                {"event": event_id, "ceremony": ceremony_id},
            )["get_custody_request"]["custody_request_json"]
            requests[name] = request
            self.assertEqual(
                json.loads(request),
                {
                    "tenant_id": TENANT_ID,
                    "election_event_id": event_id,
                    "keys_ceremony_id": ceremony_id,
                    "tally_session_id": None,
                    "trustee_name": name,
                    **bootstrap.trustee_public_keys(name),
                },
            )

        request_query = """mutation ($input: jsonb!) {
          request_server_custody(object: $input)
        }"""
        if not browser_url:
            # The human uploads the saved public file; no trustee-host command
            # produces a proof, and another trustee's file must be refused.
            with self.assertRaisesRegex(GraphQLError, "does not match this trustee"):
                apis["trustee1"].query(
                    request_query,
                    {
                        "input": {
                            "election_event_id": event_id,
                            "keys_ceremony_id": ceremony_id,
                            "custody_request_json": requests["trustee2"],
                        }
                    },
                )

        def attest(name, request):
            if browser_url:
                return browser_call("/attest", name=name, request=request)["proof"]
            scoped = json.loads(request)
            tally_id = scoped["tally_session_id"]
            target = (
                {"tally_session_id": tally_id}
                if tally_id
                else {"keys_ceremony_id": ceremony_id}
            )
            saved_file = requests[name]
            saved_path = trustee_clients[name].directory / "custody-request.json"
            saved_path.write_text(saved_file)
            input = {
                "election_event_id": event_id,
                **target,
                "custody_request_json": saved_path.read_text(),
                "retry_policy": "KEEP",
            }

            def ready_proof():
                reply = apis[name].query(request_query, {"input": input})[
                    "request_server_custody"
                ]
                self.assertIn(reply["status"], ("PENDING", "READY"), reply)
                return (
                    reply.get("custody_proof_json")
                    if reply["status"] == "READY"
                    else None
                )

            proof = wait_until(
                f"{name}'s server custody proof", ready_proof, timeout=120
            )
            self.assertEqual(saved_path.read_text(), saved_file)
            self.assertEqual(json.loads(proof)["request"], scoped)
            updates = apis[name].query("query { trustee_event_updates }")[
                "trustee_event_updates"
            ]
            self.assertFalse(
                any(action.get("request") == scoped for action in updates["actions"])
            )
            return proof

        check_query = """mutation ($event: String!, $ceremony: String!, $proof: String!) {
          confirm_key_custody(object: {election_event_id: $event, keys_ceremony_id: $ceremony,
                                    custody_proof_json: $proof}) { is_valid }
        }"""
        # A valid proof from the other trustee must not complete this trustee's step.
        wrong = apis["trustee1"].query(
            check_query,
            {
                "event": event_id,
                "ceremony": ceremony_id,
                "proof": attest("trustee2", requests["trustee2"]),
            },
        )["confirm_key_custody"]
        self.assertFalse(wrong["is_valid"])
        self.assertEqual(
            {t["status"] for t in ceremony()["status"]["trustees"]},
            {"CUSTODY_REQUEST_RETRIEVED"},
        )
        for index, name in enumerate(bootstrap.TRUSTEES):
            # Server proof production is independently exercised above and
            # leaves custody awaiting this explicit human approval.
            before = ceremony()
            self.assertEqual(
                next(
                    t["status"]
                    for t in before["status"]["trustees"]
                    if t["name"] == name
                ),
                "CUSTODY_REQUEST_RETRIEVED",
            )
            if server_browser_url:
                browser_result = server_browser_call("/key-custody", name)
                self.assertEqual(browser_result["file"], requests[name])
                self.assertEqual(
                    json.loads(browser_result["proof"])["request"],
                    json.loads(requests[name]),
                )
            else:
                proof = attest(name, requests[name])
                self.assertTrue(
                    apis[name].query(
                        check_query,
                        {
                            "event": event_id,
                            "ceremony": ceremony_id,
                            "proof": proof,
                        },
                    )["confirm_key_custody"]["is_valid"]
                )
            row = ceremony()
            self.assertEqual(
                row["execution_status"], "IN_PROGRESS" if index == 0 else "SUCCESS"
            )
            self.assertEqual(row["status"]["public_key"], joint_key)
        if browser_url:
            # Browser-generated proofs are accepted by the actual custody endpoint.
            for name in bootstrap.TRUSTEES:
                result = apis[name].query(
                    check_query,
                    {
                        "event": event_id,
                        "ceremony": ceremony_id,
                        "proof": attest(name, requests[name]),
                    },
                )["confirm_key_custody"]
                self.assertTrue(result["is_valid"])
            # Resume one identity from IndexedDB; restore the other in a completely
            # fresh browser, preserving the DKG commitments used by tally union.
            self.assertGreater(browser_call("/restart", name="trustee1")["stores"], 0)
            self.assertTrue(
                browser_call("/restore-in-new-browser", name="trustee2")["restored"]
            )

        # Rechecking uses the saved public request and cannot downgrade custody.
        self.assertTrue(
            apis["trustee1"].query(
                check_query,
                {
                    "event": event_id,
                    "ceremony": ceremony_id,
                    "proof": attest("trustee1", requests["trustee1"]),
                },
            )["confirm_key_custody"]["is_valid"]
        )
        self.assertEqual(
            {t["status"] for t in ceremony()["status"]["trustees"]},
            {"CUSTODY_CONFIRMED"},
        )

        voters = fixtures.census(self.tag + "-manual", {"A": 1})
        path = self.cli.directory / "manual-voters.csv"
        fixtures.write_census(path, voters, {"A": f"e2e-main-{self.tag}-manual"})
        self.step(
            "import-voters",
            "--election-event-id",
            event_id,
            "--file-path",
            str(path),
            "--is-local",
        )
        self.step("publish", "--election-event-id", event_id)
        self.step(
            "update-event-voting-status",
            "--election-event-id",
            event_id,
            "--voting-status",
            "OPEN",
            "--voting-channel",
            "ONLINE",
        )
        portal = Portal(event_id, self.cli)
        token = portal.login(voters[0].username)
        (file,) = portal.status(token)["get_ballot_files_urls"]["files"]
        style, _ = portal.ballot_style(file)
        self.assertEqual(style["public_key"]["public_key"], joint_key)
        self.assertEqual(
            style["public_key"]["configuration_id"], str(uuid.UUID(ceremony_id).int)
        )
        ballot = portal.encrypt(style, "Alice")
        locate_query = """query ($event: uuid!, $election: uuid!, $ballot: String!) {
          locate_ballot(election_event_id: $event, election_id: $election, ballot_id: $ballot) {
            status ballot_id content
          }
        }"""
        locate_variables = {
            "event": event_id,
            "election": ballot["electionId"],
            "ballot": ballot["ballotId"],
        }
        voter_api = Hasura(token=token)
        self.assertEqual(
            voter_api.query(locate_query, locate_variables)["locate_ballot"]["status"],
            "not-found",
        )
        receipt, errors = portal.cast(token, ballot)
        self.assertIsNone(errors)
        self.assertEqual(receipt["ballot_id"], ballot["ballotId"])
        located = voter_api.query(locate_query, locate_variables)["locate_ballot"]
        self.assertEqual(located["status"], "found")
        self.assertEqual(located["ballot_id"], ballot["ballotId"])
        self.assertEqual(json.loads(located["content"]), json.loads(ballot["content"]))
        self.step(
            "update-event-voting-status",
            "--election-event-id",
            event_id,
            "--voting-status",
            "CLOSED",
            "--voting-channel",
            "ONLINE",
        )
        tally_id = StepCli.last_id(
            self.step(
                "start-tally",
                "--election-event-id",
                event_id,
                "--tally-type",
                "ELECTORAL_RESULTS",
            )
        )
        for name in bootstrap.TRUSTEES:
            request = json.loads(requests[name])
            request["tally_session_id"] = tally_id
            proof = attest(name, json.dumps(request))
            if not browser_url:
                executions = self.admin.query(
                    "query($tally: uuid!) { sequent_backend_tally_session_execution(where: {tally_session_id: {_eq: $tally}}, order_by: {created_at: desc}, limit: 1) { status } }",
                    {"tally": tally_id},
                )["sequent_backend_tally_session_execution"]
                self.assertEqual(
                    next(
                        t["status"]
                        for t in executions[0]["status"]["trustees"]
                        if t["name"] == name
                    ),
                    "WAITING",
                )
                # A key-ceremony proof cannot authorize this tally.
                replay = apis[name].query(
                    """mutation($event: String!, $tally: String!, $proof: String!) {
                  authorize_tally_custody(object: {election_event_id: $event, tally_session_id: $tally,
                    custody_proof_json: $proof}) { is_valid }
                }""",
                    {
                        "event": event_id,
                        "tally": tally_id,
                        "proof": attest(name, requests[name]),
                    },
                )
                self.assertFalse(replay["authorize_tally_custody"]["is_valid"])
            if server_browser_url:
                browser_result = server_browser_call("/tally-custody", name, tally_id)
                self.assertEqual(browser_result["file"], requests[name])
                self.assertEqual(browser_result["proof"], proof)
            else:
                result = apis[name].query(
                    """mutation ($event: String!, $tally: String!, $proof: String!) {
                      authorize_tally_custody(object: {election_event_id: $event, tally_session_id: $tally,
                        custody_proof_json: $proof}) { is_valid }
                    }""",
                    {"event": event_id, "tally": tally_id, "proof": proof},
                )["authorize_tally_custody"]
                self.assertTrue(result["is_valid"])

        self.step(
            "update-tally",
            "--election-event-id",
            event_id,
            "--tally-id",
            tally_id,
            "--status",
            "IN_PROGRESS",
        )

        def tallied():
            (row,) = self.admin.query(
                "query ($id: uuid!) { sequent_backend_tally_session(where: {id: {_eq: $id}}) { execution_status } }",
                {"id": tally_id},
            )["sequent_backend_tally_session"]
            self.assertNotIn(row["execution_status"], ("FAILED", "CANCELLED"), row)
            return row["execution_status"] == "SUCCESS"

        wait_until("manual native tally", tallied, timeout=900)
        counted = self.admin.query(
            """query ($event: uuid!, $contest: uuid!) {
              sequent_backend_results_contest(where: {election_event_id: {_eq: $event}, contest_id: {_eq: $contest}}) { total_votes total_valid_votes }
              sequent_backend_results_contest_candidate(where: {election_event_id: {_eq: $event}, contest_id: {_eq: $contest}}) { candidate_id cast_votes }
            }""",
            {"event": event_id, "contest": style["contests"][0]["id"]},
        )
        self.assertEqual(
            counted["sequent_backend_results_contest"],
            [{"total_votes": 1, "total_valid_votes": 1}],
        )
        alice = next(
            c["id"]
            for c in style["contests"][0]["candidates"]
            if candidate_name(c) == "Alice"
        )
        self.assertEqual(
            next(
                r["cast_votes"]
                for r in counted["sequent_backend_results_contest_candidate"]
                if r["candidate_id"] == alice
            ),
            1,
        )
        if browser_url:
            health = Http().get(browser_url + "/health").json()
            self.assertEqual(health["failures"], [])
            for name in bootstrap.TRUSTEES:
                status = browser_call("/status", name=name)
                self.assertGreater(status["cycles"], 0)
                self.assertTrue(
                    any(board.startswith("tally_") for board in status["boards"])
                )
        if server_browser_url:
            health = Http().get(server_browser_url + "/health").json()
            self.assertEqual(health["failures"], [])
            self.assertEqual(sorted(health["clients"]), sorted(bootstrap.TRUSTEES))
        cast_content = json.loads(ballot["content"])
        self.verify_public_transcripts(event_id, cast_content["contests"])
        self.passed("manual ceremony")

    def test_c_trustee_graphql_board_api(self):
        """The public API scopes board discovery and acknowledges authorized halts."""
        document = fixtures.election_event(
            ENV["VOTING_PORTAL_URL"], self.tag + "-graphql-halt"
        )
        document["election_event"]["presentation"].update(
            ceremonies_policy="manual-ceremonies", batch_anonymity_policy="warn"
        )
        event_id = self.import_event(document, "graphql-halt")
        ceremony_id = StepCli.last_id(
            self.step(
                "start-key-ceremony",
                "--election-event-id",
                event_id,
                "--threshold",
                "2",
            )
        )
        token = self.keycloak.password_grant(
            TENANT_REALM,
            "api-key-client",
            "trustee1",
            ENV["TRUSTEE1_PASSWORD"],
            ENV["API_KEY_CLIENT_SECRET"],
        )["access_token"]
        api = Hasura(token=token, role="trustee-ceremony")
        listing = "query { list_trustee_boards { boards { name kind parent } } }"
        board_rows = wait_until(
            "the GraphQL halt fixture's DKG board",
            lambda: self.admin.query(
                "query($id: uuid!) { sequent_backend_protocol_board(where: {keys_ceremony_id: {_eq: $id}, parent_id: {_is_null: true}}) { name } }",
                {"id": ceremony_id},
            )["sequent_backend_protocol_board"],
            timeout=120,
        )
        name = board_rows[0]["name"]
        expected_board = {"name": name, "kind": "DKG", "parent": None}

        def available_boards():
            boards = api.query(listing)["list_trustee_boards"]["boards"]
            return boards if expected_board in boards else None

        # The board row is committed before asynchronous ceremony initialization
        # makes it available to its assigned trustees.
        boards = wait_until(
            "the GraphQL halt fixture's assigned trustee board",
            available_boards,
            timeout=120,
        )
        self.assertIn(expected_board, boards)
        updates = api.query("query { trustee_event_updates }")["trustee_event_updates"]
        self.assertIn(expected_board, updates["boards"])
        self.assertEqual(updates["actions"], [])

        anonymous = Http().post(
            ENV["HASURA_ENDPOINT"],
            json_body={"query": listing},
            headers={"x-hasura-role": "trustee-ceremony"},
        )
        self.assertTrue(anonymous.json().get("errors"))
        self.assertTrue(
            Hasura(token=token, role="trustee-read").execute(listing).get("errors")
        )
        self.assertTrue(
            api.execute(
                'query { list_trustee_boards(trustee_name: "trustee2") { boards { name } } }'
            ).get("errors")
        )
        mutation = "mutation($report: jsonb!) { report_trustee_board(report: $report) { accepted } }"
        report = {
            "board": "dkg_" + uuid.uuid4().hex,
            "kind": "HALTED",
            "detail": "E2E halt report",
        }
        self.assertTrue(api.execute(mutation, {"report": report}).get("errors"))
        report["board"] = name
        self.assertEqual(
            api.query(mutation, {"report": report}),
            {"report_trustee_board": {"accepted": True}},
        )
        (ceremony,) = self.admin.query(
            "query($id: uuid!) { sequent_backend_keys_ceremony(where: {id: {_eq: $id}}) { execution_status status } }",
            {"id": ceremony_id},
        )["sequent_backend_keys_ceremony"]
        self.assertEqual(ceremony["execution_status"], "FAILED")
        if ENV.get("STEP_E2E_BROWSER_TRUSTEES_URL"):
            health = Http().get(ENV["STEP_E2E_BROWSER_TRUSTEES_URL"] + "/health").json()
            self.assertEqual(health["failures"], [])
        self.passed("trustee GraphQL API")
