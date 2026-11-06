# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Tests for regroup_voters_by_attribute.py against in-memory Hasura and Keycloak.

Run: python3 scripts/test_regroup_voters_by_attribute.py
"""

import copy
import io
import json
import os
import sys
import tempfile
import unittest
import urllib.error
import uuid
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import regroup_voters_by_attribute as regroup

TENANT = "10000000-0000-4000-8000-000000000001"
EVENT = "10000000-0000-4000-8000-000000000002"
OTHER_TENANT = "20000000-0000-4000-8000-000000000001"
SOURCE = "10000000-0000-4000-8000-0000000000a0"
OTHER_AREA = "10000000-0000-4000-8000-0000000000b0"
CONTESTS = ["10000000-0000-4000-8000-0000000000c1", "10000000-0000-4000-8000-0000000000c2"]
ATTRIBUTE = "service_unit"
CLOSED = {"voting_status": "CLOSED", "kiosk_voting_status": "NOT_STARTED"}


def user_id(number):
    return f"30000000-0000-4000-8000-{number:012d}"


class FakeHasura:
    def __init__(self):
        self.event = {"id": EVENT, "tenant_id": TENANT, "status": dict(CLOSED)}
        self.elections = [{"id": "e1", "status": dict(CLOSED)}]
        self.areas = [
            {"id": SOURCE, "name": "All", "type": None, "parent_id": None,
             "presentation": {"allow_early_voting": "no_early_voting",
                              "i18n": {"en": {"name": "All voters"}}}, "annotations": None},
            {"id": OTHER_AREA, "name": "Elsewhere", "type": None, "parent_id": None,
             "presentation": None, "annotations": None},
        ]
        self.area_contests = [{"id": f"ac{i}", "area_id": SOURCE, "contest_id": c}
                              for i, c in enumerate(CONTESTS)]
        self.cast_votes = []
        self.tally_sessions = [{"id": "baseline", "execution_status": "SUCCESS"}]
        self.contests = [{"id": c, "election_id": "e1"} for c in CONTESTS]
        self.live_styles = [["e1", SOURCE, c] for c in CONTESTS]
        self.extra_fingerprints = {"tally_session": {"baseline": "h1"}}
        self.before_apply = None
        self.apply_calls = 0

    def load(self, tenant_id, election_event_id):
        found = tenant_id == self.event["tenant_id"] and election_event_id == self.event["id"]
        return copy.deepcopy({
            "event": self.event if found else None,
            "elections": self.elections, "areas": self.areas,
            "area_contests": self.area_contests, "cast_votes": self.cast_votes,
            "contests": self.contests,
            "tally_sessions": self.tally_sessions, "live_styles": self.live_styles,
        })

    def fingerprints(self, tenant_id, election_event_id):
        votes = {v["id"]: f"{v['voter_id_string']}|{v['status']}" for v in self.cast_votes}
        return copy.deepcopy({**self.extra_fingerprints, "cast_vote": votes})

    def apply(self, tenant_id, election_event_id, source_area_id, areas, area_contests, moves):
        if self.before_apply:
            self.before_apply()
        self.apply_calls += 1
        self.areas += copy.deepcopy(areas)
        self.area_contests += copy.deepcopy(area_contests)
        moved = {}
        for target, ids in moves.items():
            moved[target] = 0
            for vote in self.cast_votes:
                if vote["id"] in ids and vote["area_id"] == source_area_id:
                    vote["area_id"] = target
                    moved[target] += 1
        return len(areas), len(area_contests), moved


class FakeKeycloak:
    def __init__(self):
        self.users = {}
        self.puts = 0
        self.fail_after_puts = None

    def add(self, number, value, area=SOURCE, weight="1", **extra):
        attributes = {regroup.AREA_ID_ATTRIBUTE: [area], regroup.VOTE_WEIGHT_ATTRIBUTE: [weight],
                      "ssn4": ["ssn-sentinel"], **extra}
        if value is not None:
            attributes[ATTRIBUTE] = value if isinstance(value, list) else [value]
        self.users[user_id(number)] = {"id": user_id(number), "username": f"u{number}",
                                       "enabled": True, "attributes": attributes}

    def list_users(self, realm):
        assert realm == regroup.event_realm(TENANT, EVENT)
        return copy.deepcopy(list(self.users.values()))

    def get_user(self, realm, uid):
        return copy.deepcopy(self.users[uid])

    def put_user(self, realm, user):
        if self.fail_after_puts is not None and self.puts >= self.fail_after_puts:
            raise urllib.error.URLError("connection reset")
        self.puts += 1
        self.users[user["id"]] = copy.deepcopy(user)


class Args:
    def __init__(self, directory, **overrides):
        self.tenant_id = TENANT
        self.election_event_id = EVENT
        self.source_area_id = SOURCE
        self.attribute = ATTRIBUTE
        self.blank_policy = "fail"
        self.resume = False
        self.hasura_endpoint = "https://hasura.replica.example"
        self.hasura_source = "backend-db"
        self.keycloak_url = "https://keycloak.replica.example"
        self.keycloak_admin_realm = "master"
        self.plan = os.path.join(directory, "plan.json")
        self.journal = os.path.join(directory, "journal.json")
        self.__dict__.update(overrides)


class RegroupTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.mkdtemp()
        self.hasura = FakeHasura()
        self.keycloak = FakeKeycloak()
        # Unit A: three voters, two voted (one twice: an earlier discarded vote).
        self.keycloak.add(1, "Unit A", weight="100")
        self.keycloak.add(2, "Unit A", weight="250")
        self.keycloak.add(3, "Unit A", weight="7")
        self.keycloak.add(4, "Unit B", weight="318000")
        self.keycloak.add(5, "Elsewhere voter", area=OTHER_AREA)
        self.vote(1, "valid")
        self.vote(1, "discarded")
        self.vote(2, "valid")
        self.vote(4, "valid")
        self.vote(5, "valid", area=OTHER_AREA)

    def vote(self, number, status, area=SOURCE):
        self.hasura.cast_votes.append({
            "id": str(uuid.uuid4()), "voter_id_string": user_id(number), "area_id": area,
            "election_id": "e1", "status": status})

    def publish(self):
        """What generating and publishing a new ballot publication yields."""
        self.hasura.live_styles = sorted(
            ["e1", ac["area_id"], ac["contest_id"]] for ac in self.hasura.area_contests)

    def run_command(self, command, **overrides):
        context = regroup.Context(Args(self.directory, **overrides), self.hasura, self.keycloak)
        out = io.StringIO()
        try:
            code = regroup.COMMANDS[command](context, out)
        except regroup.RegroupError as error:
            return 2, {"outcome": "REFUSED", "problems": error.problems}
        except urllib.error.URLError as error:
            return 1, {"outcome": "ERROR", "problems": [str(error)]}
        return code, json.loads(out.getvalue())

    def problems(self, **overrides):
        code, result = self.run_command("plan", **overrides)
        self.assertEqual(code, 2)
        return " | ".join(result["problems"])

    def area_named(self, name):
        return next(a for a in self.hasura.areas if a["name"] == name)

    def test_plan_reports_exact_mapping_and_writes_nothing(self):
        code, result = self.run_command("plan")
        self.assertEqual((code, result["outcome"]), (0, "PLANNED"))
        self.assertEqual(result["counts"]["voters_in_scope"], 4)
        self.assertEqual(result["counts"]["voters_out_of_scope"], 1)
        self.assertEqual(result["counts"]["areas_to_create"], 2)
        self.assertEqual(result["counts"]["contests_to_create"], 4)
        self.assertEqual(result["counts"]["cast_votes_to_move"], 4)
        self.assertEqual(result["areas"]["Unit A"],
                         {"voters": 3, "voted": 2, "voted_weight": 350, "cast_votes": 3})
        self.assertEqual(result["areas"]["Unit B"]["voted_weight"], 318000)
        self.assertEqual(len(result["warnings"]), 2)
        self.assertEqual(self.hasura.apply_calls, 0)
        self.assertEqual(self.keycloak.puts, 0)
        self.assertEqual(os.stat(Args(self.directory).plan).st_mode & 0o777, 0o600)

    def test_apply_moves_voters_and_all_their_votes_then_verifies(self):
        self.run_command("plan")
        code, result = self.run_command("apply")
        self.assertEqual((code, result["outcome"]), (3, "PUBLICATION_PENDING"))
        unit_a = self.area_named("Unit A")
        self.assertEqual(unit_a["annotations"][regroup.TOOL_ANNOTATION]["source_area_id"], SOURCE)
        self.assertEqual(unit_a["presentation"], {"allow_early_voting": "no_early_voting"})
        self.assertEqual(unit_a["name"], "Unit A")
        self.assertEqual({v["area_id"] for v in self.hasura.cast_votes
                          if v["voter_id_string"] == user_id(1)}, {unit_a["id"]})
        self.assertEqual(self.keycloak.users[user_id(3)]["attributes"]["area-id"], [unit_a["id"]])
        self.assertEqual(self.keycloak.users[user_id(3)]["attributes"]["ssn4"], ["ssn-sentinel"])
        self.assertEqual(self.keycloak.users[user_id(5)]["attributes"]["area-id"], [OTHER_AREA])
        self.assertTrue(all(v["area_id"] == OTHER_AREA for v in self.hasura.cast_votes
                            if v["voter_id_string"] == user_id(5)))

        self.publish()
        code, result = self.run_command("verify")
        self.assertEqual((code, result["outcome"]), (0, "READY_FOR_TALLY"))

    def test_journal_is_durable_before_the_first_mutation(self):
        self.run_command("plan")
        journal = Args(self.directory).journal

        def check():
            with open(journal) as handle:
                recorded = json.load(handle)
            self.assertEqual(len(recorded["plan"]["user_moves"]), 4)
            self.assertIn("baseline", recorded["fingerprints"]["tally_session"])
        self.hasura.before_apply = check
        self.run_command("apply")
        self.assertEqual(self.hasura.apply_calls, 1)

    def test_apply_refuses_to_reuse_a_journal(self):
        self.run_command("plan")
        Path(Args(self.directory).journal).write_text("{}")
        with self.assertRaises(FileExistsError):
            self.run_command("apply")
        self.assertEqual(self.hasura.apply_calls, 0)

    def test_repeat_run_is_a_no_op(self):
        self.run_command("plan")
        self.run_command("apply")
        code, result = self.run_command("plan")
        self.assertEqual(code, 0)
        self.assertEqual(result["counts"]["areas"], 2)
        for key in ("areas_to_create", "contests_to_create", "users_to_move", "cast_votes_to_move"):
            self.assertEqual(result["counts"][key], 0, key)
        puts = self.keycloak.puts
        code, _ = self.run_command("apply", journal=os.path.join(self.directory, "j2.json"))
        self.assertEqual(code, 3)
        self.assertEqual(self.keycloak.puts, puts)
        self.assertEqual(len([a for a in self.hasura.areas if a["name"] == "Unit A"]), 1)

    def assert_refused_before_any_write(self, code, result):
        self.assertEqual(code, 2)
        self.assertIn("changed since the plan", result["problems"][0])
        self.assertEqual((self.hasura.apply_calls, self.keycloak.puts), (0, 0))
        self.assertFalse(os.path.exists(Args(self.directory).journal))

    def test_plan_binds_hashed_voter_and_ballot_invariants(self):
        self.run_command("plan")
        text = Path(Args(self.directory).plan).read_text()
        plan = json.loads(text)
        for key in ("voter_invariants", "ballot_identity"):
            self.assertRegex(plan[key], "^[0-9a-f]{64}$")
        self.assertNotIn("ssn-sentinel", text)
        code, result = self.run_command("apply")
        self.assertEqual((code, result["outcome"]), (3, "PUBLICATION_PENDING"))

    def test_apply_refuses_eligibility_or_enabled_changes_before_any_write(self):
        for change in ("eligibility", "enabled"):
            with self.subTest(change=change):
                self.setUp()
                self.run_command("plan")
                user = self.keycloak.users[user_id(3)]
                if change == "enabled":
                    user["enabled"] = False
                else:
                    user["attributes"]["authorized-election-ids"] = ["other-election"]
                self.assert_refused_before_any_write(*self.run_command("apply"))

    def test_apply_refuses_a_balanced_weight_swap(self):
        self.run_command("plan")
        before = json.loads(Path(Args(self.directory).plan).read_text())["summary"]
        attributes = lambda n: self.keycloak.users[user_id(n)]["attributes"]
        attributes(1)["vote-weight"], attributes(2)["vote-weight"] = (
            attributes(2)["vote-weight"], attributes(1)["vote-weight"])
        state = self.hasura.load(TENANT, EVENT)
        swapped = regroup.build_plan(state, self.keycloak.list_users(regroup.event_realm(TENANT, EVENT)),
                                     TENANT, EVENT, SOURCE, ATTRIBUTE, regroup.BlankPolicy.FAIL)
        self.assertEqual(swapped["summary"], before)
        self.assert_refused_before_any_write(*self.run_command("apply"))

    def test_apply_refuses_ballots_handed_to_another_voter(self):
        self.run_command("plan")
        first = next(v for v in self.hasura.cast_votes
                     if v["voter_id_string"] == user_id(1) and v["status"] == "valid")
        second = next(v for v in self.hasura.cast_votes if v["voter_id_string"] == user_id(2))
        first["voter_id_string"], second["voter_id_string"] = user_id(2), user_id(1)
        self.assert_refused_before_any_write(*self.run_command("apply"))

    def test_apply_refuses_a_balanced_valid_discarded_status_swap(self):
        self.run_command("plan")
        before = json.loads(Path(Args(self.directory).plan).read_text())["summary"]
        votes = [v for v in self.hasura.cast_votes if v["voter_id_string"] == user_id(1)]
        self.assertEqual(sorted(v["status"] for v in votes), ["discarded", "valid"])
        votes[0]["status"], votes[1]["status"] = votes[1]["status"], votes[0]["status"]
        state = self.hasura.load(TENANT, EVENT)
        swapped = regroup.build_plan(state, self.keycloak.list_users(regroup.event_realm(TENANT, EVENT)),
                                     TENANT, EVENT, SOURCE, ATTRIBUTE, regroup.BlankPolicy.FAIL)
        self.assertEqual(swapped["summary"], before)
        self.assert_refused_before_any_write(*self.run_command("apply"))

    def test_apply_refuses_when_the_event_changed_after_review(self):
        self.run_command("plan")
        self.keycloak.add(6, "Unit C")
        code, result = self.run_command("apply")
        self.assertEqual(code, 2)
        self.assertIn("changed since the plan", result["problems"][0])
        self.assertEqual(self.hasura.apply_calls, 0)
        self.assertFalse(os.path.exists(Args(self.directory).journal))

    def test_apply_refuses_a_plan_for_another_scope(self):
        self.run_command("plan")
        code, result = self.run_command("apply", attribute="ethnic_code")
        self.assertEqual(code, 2)
        self.assertIn("attribute differs", result["problems"][0])

    def test_concurrent_keycloak_change_is_detected_not_overwritten(self):
        self.run_command("plan")

        def someone_moves_voter():
            self.keycloak.users[user_id(3)]["attributes"]["area-id"] = [OTHER_AREA]
        self.hasura.before_apply = someone_moves_voter
        code, result = self.run_command("apply")
        self.assertEqual(code, 2)
        self.assertIn("changed concurrently", " ".join(result["problems"]))
        self.assertEqual(self.keycloak.users[user_id(3)]["attributes"]["area-id"], [OTHER_AREA])

    def test_concurrent_cast_vote_change_stops_before_keycloak(self):
        self.run_command("plan")

        def someone_moves_vote():
            self.hasura.cast_votes[0]["area_id"] = OTHER_AREA
        self.hasura.before_apply = someone_moves_vote
        code, result = self.run_command("apply")
        self.assertEqual(code, 2)
        self.assertIn("not in the source area any more", result["problems"][0])
        self.assertEqual(self.hasura.cast_votes[0]["area_id"], OTHER_AREA)
        self.assertEqual(self.keycloak.puts, 0)

    def test_verify_detects_a_changed_original_tally(self):
        self.run_command("plan")
        self.run_command("apply")
        self.hasura.extra_fingerprints["tally_session"]["baseline"] = "tampered"
        code, result = self.run_command("verify")
        self.assertEqual(code, 2)
        self.assertIn("tally_session", " ".join(result["problems"]))

    def test_verify_detects_a_changed_cast_vote(self):
        self.run_command("plan")
        self.run_command("apply")
        self.hasura.cast_votes[0]["status"] = "discarded"
        code, result = self.run_command("verify")
        self.assertEqual(code, 2)

    def test_verify_requires_every_election_area_contest_style(self):
        self.run_command("plan")
        self.run_command("apply")
        self.publish()
        unit_a = self.area_named("Unit A")["id"]
        self.hasura.live_styles.remove(["e1", unit_a, CONTESTS[1]])
        code, result = self.run_command("verify")
        self.assertEqual((code, result["outcome"]), (3, "PUBLICATION_PENDING"))
        self.assertIn("1 (election, area, contest)", result["notes"][0])

    def test_verify_rejects_unexpected_styles_for_regrouped_areas(self):
        self.run_command("plan")
        self.run_command("apply")
        self.publish()
        self.hasura.live_styles.append(["e2", self.area_named("Unit B")["id"], CONTESTS[0]])
        code, result = self.run_command("verify")
        self.assertEqual(code, 2)
        self.assertIn("not expected", result["problems"][0])

    def test_verify_detects_changed_weight_or_eligibility(self):
        for change in ({"vote-weight": ["999"]}, {"authorized-election-ids": ["other"]}, "disable"):
            with self.subTest(change=change):
                self.setUp()
                self.run_command("plan")
                self.run_command("apply")
                user = self.keycloak.users[user_id(2)]
                if change == "disable":
                    user["enabled"] = False
                else:
                    user["attributes"].update(change)
                code, result = self.run_command("verify")
                self.assertEqual(code, 2)
                self.assertIn("changed in something other than their area", " ".join(result["problems"]))

    def interrupted_apply(self):
        self.run_command("plan")
        self.keycloak.fail_after_puts = 2
        code, _ = self.run_command("apply")
        self.assertEqual(code, 1)
        self.keycloak.fail_after_puts = None
        return Path(Args(self.directory).journal).read_bytes()

    def test_resume_finishes_from_the_original_journal_without_rewriting_it(self):
        journal = self.interrupted_apply()
        code, result = self.run_command("apply", resume=True)
        self.assertEqual((code, result["outcome"]), (3, "PUBLICATION_PENDING"))
        self.assertEqual(Path(Args(self.directory).journal).read_bytes(), journal)
        self.assertEqual(len([a for a in self.hasura.areas if a["name"] == "Unit A"]), 1)
        self.assertEqual(len(self.hasura.area_contests), len(CONTESTS) * 3)
        self.publish()
        code, result = self.run_command("verify")
        self.assertEqual((code, result["outcome"]), (0, "READY_FOR_TALLY"))

    def test_resume_refuses_work_the_original_plan_did_not_record(self):
        self.interrupted_apply()
        self.keycloak.add(6, "Unit C")
        code, result = self.run_command("apply", resume=True)
        self.assertEqual(code, 2)
        self.assertIn("not in the original plan", " ".join(result["problems"]))

    def test_resume_still_detects_changes_made_after_the_first_mutation(self):
        self.interrupted_apply()
        self.hasura.extra_fingerprints["tally_session"]["baseline"] = "changed meanwhile"
        code, result = self.run_command("apply", resume=True)
        self.assertEqual(code, 2)
        self.assertIn("tally_session", " ".join(result["problems"]))

    def test_other_environment_is_refused_before_any_write(self):
        other = {"hasura_endpoint": "https://hasura.production.example",
                 "hasura_source": "other-db",
                 "keycloak_url": "https://keycloak.production.example",
                 "keycloak_admin_realm": "other"}
        self.run_command("plan")
        for name, value in other.items():
            with self.subTest(command="apply", field=name):
                code, result = self.run_command("apply", **{name: value})
                self.assertEqual(code, 2)
                self.assertIn("environment differs", result["problems"][0])
                self.assertFalse(os.path.exists(Args(self.directory).journal))
                self.assertEqual((self.hasura.apply_calls, self.keycloak.puts), (0, 0))

        self.interrupted_apply()
        snapshot = (copy.deepcopy(self.hasura.cast_votes), copy.deepcopy(self.keycloak.users),
                    self.hasura.apply_calls, Path(Args(self.directory).journal).read_bytes())
        for command, extra in (("apply", {"resume": True}), ("verify", {}), ("revert", {})):
            for name, value in other.items():
                with self.subTest(command=command, field=name):
                    code, result = self.run_command(command, **extra, **{name: value})
                    self.assertEqual(code, 2)
                    self.assertIn("environment differs", result["problems"][0])
                    self.assertEqual(
                        (self.hasura.cast_votes, self.keycloak.users, self.hasura.apply_calls,
                         Path(Args(self.directory).journal).read_bytes()), snapshot)

    def test_equivalent_urls_match_and_the_journal_holds_no_secret(self):
        self.run_command("plan")
        code, _ = self.run_command("apply", hasura_endpoint="HTTPS://Hasura.Replica.Example/",
                                   keycloak_url="https://KEYCLOAK.replica.example/")
        self.assertEqual(code, 3)
        journal = json.loads(Path(Args(self.directory).journal).read_text())
        self.assertEqual(journal["plan"]["environment"]["hasura_endpoint"],
                         "https://hasura.replica.example")

    def test_urls_with_credentials_or_queries_are_refused(self):
        for url in ("https://admin:secret@hasura.example", "https://hasura.example?admin_secret=x",
                    "https://hasura.example#x", "ftp://hasura.example", "hasura.example"):
            with self.subTest(url=url):
                with self.assertRaises(regroup.RegroupError):
                    regroup.Context(Args(self.directory, hasura_endpoint=url),
                                    self.hasura, self.keycloak)

    def test_resume_refuses_a_voter_changed_during_the_interruption(self):
        journal = self.interrupted_apply()
        self.keycloak.users[user_id(4)]["attributes"]["vote-weight"] = ["5"]
        writes = (self.hasura.apply_calls, self.keycloak.puts)
        code, result = self.run_command("apply", resume=True)
        self.assertEqual(code, 2)
        self.assertIn("a voter changed since the first apply", " ".join(result["problems"]))
        self.assertEqual((self.hasura.apply_calls, self.keycloak.puts), writes)
        self.assertEqual(Path(Args(self.directory).journal).read_bytes(), journal)

    def test_resume_refuses_a_journal_without_invariants(self):
        self.interrupted_apply()
        path = Path(Args(self.directory).journal)
        journal = json.loads(path.read_text())
        del journal["plan"]["voter_invariants"]
        path.write_text(json.dumps(journal))
        recorded = path.read_bytes()
        writes = (self.hasura.apply_calls, self.keycloak.puts)
        code, result = self.run_command("apply", resume=True)
        self.assertEqual(code, 2)
        self.assertIn("predates voter and ballot invariants", result["problems"][0])
        self.assertEqual((self.hasura.apply_calls, self.keycloak.puts), writes)
        self.assertEqual(path.read_bytes(), recorded)

    def test_verify_detects_ballot_owner_drift_after_apply(self):
        self.run_command("plan")
        self.run_command("apply")
        self.publish()
        first = next(v for v in self.hasura.cast_votes
                     if v["voter_id_string"] == user_id(1) and v["status"] == "valid")
        second = next(v for v in self.hasura.cast_votes if v["voter_id_string"] == user_id(2))
        first["voter_id_string"], second["voter_id_string"] = user_id(2), user_id(1)
        code, result = self.run_command("verify")
        self.assertEqual(code, 2)
        self.assertIn("changed owner, status or number", " ".join(result["problems"]))

    def test_revert_refuses_once_a_new_tally_exists(self):
        self.run_command("plan")
        self.run_command("apply")
        self.hasura.tally_sessions.append({"id": "second", "execution_status": "SUCCESS"})
        puts = self.keycloak.puts
        code, result = self.run_command("revert")
        self.assertEqual(code, 2)
        self.assertIn("created after apply: second", result["problems"][0])
        self.assertEqual(self.keycloak.puts, puts)

    def test_revert_refuses_while_a_tally_is_running(self):
        self.run_command("plan")
        self.run_command("apply")
        self.hasura.tally_sessions.append({"id": "second", "execution_status": "STARTED"})
        puts = self.keycloak.puts
        code, result = self.run_command("revert")
        self.assertEqual(code, 2)
        self.assertIn("tally sessions not finished", result["problems"][0])
        self.assertEqual(self.keycloak.puts, puts)

    def test_revert_fails_and_overwrites_nothing_after_concurrent_changes(self):
        self.run_command("plan")
        self.run_command("apply")
        self.keycloak.users[user_id(3)]["attributes"]["area-id"] = [OTHER_AREA]
        moved_vote = next(v for v in self.hasura.cast_votes if v["voter_id_string"] == user_id(4))
        moved_vote["area_id"] = OTHER_AREA
        code, result = self.run_command("revert")
        self.assertEqual(code, 2)
        problems = " | ".join(result["problems"])
        self.assertIn("changed concurrently", problems)
        self.assertIn("1 cast vote(s) are not back", problems)
        self.assertEqual(self.keycloak.users[user_id(3)]["attributes"]["area-id"], [OTHER_AREA])
        self.assertEqual(moved_vote["area_id"], OTHER_AREA)
        self.assertEqual(self.keycloak.users[user_id(1)]["attributes"]["area-id"], [SOURCE])

    def test_revert_restores_voters_and_votes(self):
        before_votes = copy.deepcopy(self.hasura.cast_votes)
        before_users = copy.deepcopy(self.keycloak.users)
        self.run_command("plan")
        self.run_command("apply")
        code, result = self.run_command("revert")
        self.assertEqual((code, result["outcome"]), (0, "REVERTED"))
        self.assertEqual(self.hasura.cast_votes, before_votes)
        self.assertEqual(self.keycloak.users, before_users)

    def test_blank_values_fail_by_default(self):
        self.keycloak.add(6, None)
        self.keycloak.add(7, "")
        self.keycloak.add(8, "   ")
        self.assertIn("3 voter(s) have no service_unit", self.problems())

    def test_blank_values_can_explicitly_stay_in_the_source_area(self):
        self.keycloak.add(6, "")
        self.vote(6, "valid")
        self.run_command("plan", blank_policy="keep-in-source-area")
        code, result = self.run_command("apply", blank_policy="keep-in-source-area")
        self.assertEqual(code, 3)
        self.assertEqual(result["counts"]["voters_kept_in_source_area"], 1)
        self.assertEqual(result["areas"]["All"]["voted"], 1)
        self.assertEqual(self.keycloak.users[user_id(6)]["attributes"]["area-id"], [SOURCE])

    def test_ambiguous_attribute_values_are_refused(self):
        self.keycloak.add(6, ["Unit A", "Unit B"])
        self.keycloak.add(7, " Unit A")
        self.keycloak.add(8, "unit a")
        problems = self.problems()
        self.assertIn("2 values for service_unit", problems)
        self.assertIn("whitespace", problems)
        self.assertIn("differ only by case", problems)

    def test_area_name_used_by_another_area_is_refused(self):
        self.keycloak.add(6, "elsewhere")
        self.assertIn("already used by area", self.problems())

    def test_duplicate_regrouped_areas_are_refused(self):
        marker = {regroup.TOOL_ANNOTATION: {"source_area_id": SOURCE, "attribute": ATTRIBUTE}}
        for number in (1, 2):
            self.hasura.areas.append({"id": f"10000000-0000-4000-8000-00000000d00{number}",
                                      "name": "Unit A", "annotations": marker})
        self.assertIn("several areas named 'Unit A'", self.problems())

    def test_open_voting_running_tally_and_unresolved_votes_are_refused(self):
        self.hasura.elections[0]["status"]["voting_status"] = "OPEN"
        self.hasura.event["status"]["kiosk_voting_status"] = "PAUSED"
        self.hasura.tally_sessions.append({"id": "running", "execution_status": "IN_PROGRESS"})
        self.vote(3, "in-progress")
        problems = self.problems()
        for expected in ("online voting is 'OPEN'", "kiosk_voting_status is 'PAUSED'",
                         "tally sessions not finished: running", "still in-progress"):
            self.assertIn(expected, problems)

    def test_event_from_another_tenant_is_not_found(self):
        self.assertIn("not found in tenant", self.problems(tenant_id=OTHER_TENANT))

    def test_source_area_must_belong_to_the_event(self):
        self.assertIn("not in this event",
                      self.problems(source_area_id="10000000-0000-4000-8000-0000000000ff"))

    def test_votes_outside_the_voters_area_and_orphan_votes_are_refused(self):
        self.vote(3, "discarded", area=OTHER_AREA)
        self.vote(99, "valid")
        problems = self.problems()
        self.assertIn("not the source area", problems)
        self.assertIn("belongs to no voter", problems)

    def test_non_canonical_uuid_is_refused(self):
        with self.assertRaises(regroup.RegroupError):
            regroup.parse_uuid("not-a-uuid", "--tenant-id")
        with self.assertRaises(regroup.RegroupError):
            regroup.parse_uuid("{" + TENANT + "}", "--tenant-id")


def recording_urlopen(requests, response_body):
    class Response(io.BytesIO):
        def __enter__(self):
            return self

        def __exit__(self, *exc):
            return False

    def urlopen(request):
        requests.append(request)
        return Response(json.dumps(response_body).encode())
    return urlopen


class HasuraRequestTest(unittest.TestCase):
    def test_every_declared_variable_is_used_and_sent(self):
        import re
        cases = {
            "areas only": ([{"id": "a"}], [], {}),
            "contests only": ([], [{"id": "c"}], {}),
            "moves only": ([], [], {"target": ["v1"]}),
            "everything": ([{"id": "a"}], [{"id": "c"}], {"target": ["v1"], "other": ["v2"]}),
            "empty move list": ([{"id": "a"}], [], {"target": []}),
        }
        for name, (areas, contests, moves) in cases.items():
            with self.subTest(name):
                requests = []
                data = {"areas": {"affected_rows": len(areas)},
                        "contests": {"affected_rows": len(contests)},
                        "m0": {"affected_rows": 1}, "m1": {"affected_rows": 1}}
                hasura = regroup.Hasura("http://h.invalid", "s", "backend-db",
                                        recording_urlopen(requests, {"data": data}))
                hasura.apply(TENANT, EVENT, SOURCE, areas, contests, moves)
                body = json.loads(requests[0].data)
                header, _, selection = body["query"].partition(") {")
                declared = set(re.findall(r"\$(\w+):", header))
                used = set(re.findall(r"\$(\w+)", selection))
                self.assertEqual(declared, used)
                self.assertEqual(declared, set(body["variables"]))

    def test_nothing_to_do_sends_no_request(self):
        requests = []
        hasura = regroup.Hasura("http://h.invalid", "s", "backend-db", recording_urlopen(requests, {}))
        self.assertEqual(hasura.apply(TENANT, EVENT, SOURCE, [], [], {"target": []}),
                         (0, 0, {"target": 0}))
        self.assertEqual(requests, [])

    def test_moves_are_guarded_by_scope_and_source_area_in_one_request(self):
        requests = []

        class Response(io.BytesIO):
            def __enter__(self):
                return self

            def __exit__(self, *exc):
                return False

        def urlopen(request):
            requests.append(request)
            data = {"areas": {"affected_rows": 1}, "contests": {"affected_rows": 0},
                    "m0": {"affected_rows": 2}}
            return Response(json.dumps({"data": data}).encode())

        hasura = regroup.Hasura("http://hasura.invalid/", "secret", "backend-db", urlopen)
        result = hasura.apply(TENANT, EVENT, SOURCE, [{"id": "a"}], [], {"target": ["v1", "v2"]})
        self.assertEqual(result, (1, 0, {"target": 2}))
        self.assertEqual(len(requests), 1)
        body = json.loads(requests[0].data)
        self.assertTrue(body["query"].startswith("mutation("))
        self.assertIn("area_id: {_eq: $s}", body["query"])
        self.assertIn("tenant_id: {_eq: $t}", body["query"])
        self.assertEqual(body["variables"]["ids0"], ["v1", "v2"])
        self.assertEqual(body["variables"]["s"], SOURCE)
        self.assertEqual(requests[0].get_header("X-hasura-admin-secret"), "secret")


    def test_fingerprints_are_read_only_on_the_configured_source(self):
        requests = []

        class Response(io.BytesIO):
            def __enter__(self):
                return self

            def __exit__(self, *exc):
                return False

        def urlopen(request):
            requests.append(json.loads(request.data))
            return Response(json.dumps({"result": [["id", "hash"], ["r1", "h"]]}).encode())

        hasura = regroup.Hasura("http://hasura.invalid", "secret", "backend-db", urlopen)
        result = hasura.fingerprints(TENANT, EVENT)
        self.assertEqual(set(result), set(regroup.FINGERPRINTED_TABLES))
        self.assertTrue(all(r["args"]["source"] == "backend-db" for r in requests))
        self.assertTrue(all(r["args"]["read_only"] for r in requests))
        cast_vote_sql = next(r["args"]["sql"] for r in requests if "cast_vote t" in r["args"]["sql"])
        self.assertIn("- 'area_id'", cast_vote_sql)

    def test_live_styles_are_read_from_the_frozen_ballot_eml(self):
        rows = [{"id": "s1", "ballot_eml": json.dumps(
            {"election_id": "e1", "area_id": "a1", "contests": [{"id": "c1"}, {"id": "c2"}]})}]
        self.assertEqual(regroup.live_style_tuples(rows), [("e1", "a1", "c1"), ("e1", "a1", "c2")])
        with self.assertRaises(regroup.RegroupError):
            regroup.live_style_tuples([{"id": "s2", "ballot_eml": None}])


if __name__ == "__main__":
    unittest.main()
