# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""T1: the sample configuration holds every value the generator reads, and
loads onto an event's Post, country, contests and candidates."""

import json
import unittest

import load_sample_config as loader

CONFIG = json.loads(loader.DEFAULT_CONFIG.read_text())
KIT = loader.HERE / "kit"

# What windmill's `get_annotations` requires (eml_generator.rs).
EVENT_KEYS = {
    "miru:election-event-id",
    "miru:election-event-name",
    "miru:sbei-users",
    "miru:root-ca",
    "miru:intermediate-cas",
    "miru:use-root-ca",
}
POST_KEYS = {
    "miru:election-id",
    "miru:election-name",
    "miru:geographical-region",
    "miru:voting-center",
    "miru:precinct-code",
    "pollcenter_code",
}
COUNTRY_KEYS = {
    "miru:area-ccs-servers",
    "miru:area-station-id",
    "miru:area-station-name",
    "miru:area-threshold",
    "miru:area-trustee-users",
    "miru:area-country",
    "miru:registered-voters",
}
CONTEST_KEYS = {"miru:contest-id", "miru:contest-name"}
CANDIDATE_KEYS = {
    "miru:candidate-id",
    "miru:candidate-name",
    "miru:candidate-setting",
    "miru:candidate-affiliation-id",
    "miru:candidate-affiliation-registered-name",
    "miru:candidate-affiliation-party",
}


def row(row_id, name):
    return {"id": row_id, "presentation": {"i18n": {"en": {"name": name}}}}


class FakeHasura:
    def __init__(self, contests):
        self.contests = contests
        self.updates = []

    def query(self, query, variables=None):
        if "sequent_backend_area_contest" in query:
            return {"sequent_backend_area_contest": [{"contest": c} for c in self.contests]}
        table = query.split("update_sequent_backend_")[1].split("(")[0]
        self.updates.append((table, variables["id"], variables["annotations"]))
        return {f"update_sequent_backend_{table}": {"affected_rows": 1}}


class SampleConfiguration(unittest.TestCase):
    def test_every_value_the_generator_reads_is_present(self):
        self.assertLessEqual(EVENT_KEYS, set(CONFIG["election_event"]["annotations"]))
        self.assertLessEqual(POST_KEYS, set(CONFIG["post"]["annotations"]))
        self.assertLessEqual(COUNTRY_KEYS, set(CONFIG["country"]["annotations"]))
        for contest in CONFIG["post"]["contests"]:
            self.assertLessEqual(CONTEST_KEYS, set(contest["annotations"]))
            for candidate in contest["candidates"]:
                self.assertLessEqual(CANDIDATE_KEYS, set(candidate["annotations"]))

    def test_the_country_needs_two_of_its_three_sbeis_and_is_received_by_hand(self):
        country = CONFIG["country"]["annotations"]
        sbeis = CONFIG["election_event"]["annotations"]["miru:sbei-users"]
        self.assertEqual(country["miru:area-threshold"], "2")
        self.assertEqual(country["miru:area-trustee-users"], [s["miru_id"] for s in sbeis])
        post = CONFIG["post"]["annotations"]["miru:election-id"]
        self.assertTrue(all(s["miru_election_id"] == post for s in sbeis))
        (destination,) = country["miru:area-ccs-servers"]
        self.assertEqual(destination["address"], "")

    def test_the_keys_and_cas_are_the_kits(self):
        event = CONFIG["election_event"]["annotations"]
        self.assertEqual(event["miru:root-ca"], (KIT / "root-ca.pem").read_text())
        self.assertEqual(
            event["miru:intermediate-cas"], (KIT / "intermediate-ca.pem").read_text()
        )
        (destination,) = CONFIG["country"]["annotations"]["miru:area-ccs-servers"]
        self.assertEqual(
            destination["public_key_pem"], (KIT / "ccs-manila-public-key.pem").read_text()
        )
        for sbei in event["miru:sbei-users"]:
            self.assertTrue((KIT / f"{sbei['username']}.p12").is_file())

    def test_annotations_are_stored_as_strings(self):
        values = loader.annotation_values(CONFIG["country"]["annotations"])
        self.assertTrue(all(isinstance(value, str) for value in values.values()))
        servers = json.loads(values["miru:area-ccs-servers"])
        self.assertEqual(servers[0]["name"], "CCS Manila (SBOC)")
        self.assertEqual(values["miru:area-threshold"], "2")


class Loading(unittest.TestCase):
    def test_contests_and_candidates_are_matched_in_name_order(self):
        contests = [
            dict(row("c2", "Contest B"), candidates=[row("b1", "Zed"), row("b2", "Amy")]),
            dict(row("c1", "Contest A"), candidates=[row("a1", "Alice")]),
        ]
        hasura = FakeHasura(contests)
        changed = loader.load(hasura, CONFIG, "event", "post", "country", ["admin"])
        self.assertEqual(changed, 3 + 2 + 3)
        tables = [(table, row_id) for table, row_id, _ in hasura.updates]
        self.assertEqual(
            tables,
            [
                ("election_event", "event"),
                ("election", "post"),
                ("area", "country"),
                ("contest", "c1"),
                ("contest", "c2"),
                ("candidate", "a1"),
                ("candidate", "b2"),
                ("candidate", "b1"),
            ],
        )
        updates = {row_id: values for _, row_id, values in hasura.updates}
        self.assertEqual(updates["c1"]["miru:contest-id"], "1")
        self.assertEqual(updates["c2"]["miru:contest-id"], "2")
        self.assertEqual(updates["b2"]["miru:candidate-id"], "201")
        sbeis = json.loads(updates["event"]["miru:sbei-users"])
        self.assertEqual(
            [s["username"] for s in sbeis], ["admin", "sbei-madrid-2", "sbei-madrid-3"]
        )
        # The configuration itself is unchanged.
        self.assertEqual(
            CONFIG["election_event"]["annotations"]["miru:sbei-users"][0]["username"],
            "sbei-madrid-1",
        )

    def test_an_event_with_more_than_the_configuration_describes_is_refused(self):
        three = [row(f"c{i}", f"Contest {i}") for i in range(3)]
        with self.assertRaisesRegex(ValueError, "describes 2"):
            loader.plan(CONFIG, three, {})
        many = [row(f"x{i}", f"Candidate {i}") for i in range(4)]
        with self.assertRaisesRegex(ValueError, "has 4 candidates"):
            loader.plan(CONFIG, [row("c", "Contest")], {"c": many})
        with self.assertRaisesRegex(ValueError, "4 SBEI accounts"):
            loader.with_sbeis(CONFIG, ["a", "b", "c", "d"])

    def test_a_scope_writes_only_its_part(self):
        contests = [dict(row("c1", "Contest A"), candidates=[row("a1", "Alice")])]
        ballot = FakeHasura(contests)
        loader.load(ballot, CONFIG, "event", "post", "country", scope=loader.SCOPE_BALLOT)
        self.assertEqual(
            [table for table, _, _ in ballot.updates], ["contest", "candidate"]
        )
        package = FakeHasura([])
        loader.load(package, CONFIG, "event", "post", "country", scope=loader.SCOPE_PACKAGE)
        self.assertEqual(
            [table for table, _, _ in package.updates],
            ["election_event", "election", "area"],
        )
        with self.assertRaisesRegex(ValueError, "unknown scope"):
            loader.load(package, CONFIG, "event", "post", "country", scope="other")

    def test_an_area_without_contests_in_the_post_is_refused(self):
        with self.assertRaisesRegex(ValueError, "no contest"):
            loader.load(FakeHasura([]), CONFIG, "event", "post", "country")

    def test_the_command_needs_credentials(self):
        with self.assertRaises(SystemExit):
            loader.main(
                ["--election-event-id", "e", "--election-id", "p", "--area-id", "a"]
            )


if __name__ == "__main__":
    unittest.main()
