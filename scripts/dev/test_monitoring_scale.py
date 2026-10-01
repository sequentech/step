# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Tests for scripts.dev.monitoring_scale's planning, without a stack."""

import base64
import contextlib
import csv
import hashlib
import io
import random
import re
import sys
import tempfile
import unittest
import urllib.parse
from collections import Counter
from datetime import UTC, date, datetime, timedelta
from pathlib import Path

from scripts.dev import monitoring_scale as scale
from scripts.dev.monitoring_scale import (
    Account,
    Area,
    SeedError,
    VotePlan,
)

TENANT = "11111111-1111-4111-8111-111111111111"
EVENT = "22222222-2222-4222-8222-222222222222"
ON = date(2026, 9, 30)
NOW = datetime(2026, 9, 30, 12, tzinfo=UTC)
# What the importer accepts as a header (import_users.rs HEADER_RE).
HEADER = re.compile(r"^[a-zA-Z0-9._-]+$")

AREAS = [
    Area("a1", "North", ("e-once",)),
    Area("a2", "South", ("e-once", "e-twice")),
    Area("a3", "Sea", ("e-unlimited",)),
]
LIMITS = {"e-once": 1, "e-twice": 2, "e-unlimited": 0}


def accounts(count, areas=AREAS):
    return [
        Account(f"user-{index}", f"{scale.PREFIX}{index:06d}", areas[index % 3].id)
        for index in range(count)
    ]


def age_band(years):
    if years <= 24:
        return "18-24"
    if years <= 39:
        return "25-39"
    return "40-59" if years <= 59 else "60+"


def age(born, on):
    return on.year - born.year - ((on.month, on.day) < (born.month, born.day))


class Limits(unittest.TestCase):
    def test_voter_count_needs_force_above_the_limit(self):
        scale.check_voter_count(scale.MAX_VOTERS, force=False)
        with self.assertRaises(SeedError):
            scale.check_voter_count(scale.MAX_VOTERS + 1, force=False)
        scale.check_voter_count(scale.MAX_VOTERS + 1, force=True)
        with self.assertRaises(SeedError):
            scale.check_voter_count(0, force=True)

    def test_shares_and_ids_are_checked(self):
        scale.check_share("--turnout", 0.0)
        scale.check_share("--turnout", 1.0)
        with self.assertRaises(SeedError):
            scale.check_share("--turnout", 1.5)
        scale.check_uuid("--event", EVENT)
        with self.assertRaises(SeedError):
            scale.check_uuid("--event", "not-an-id")


class Batching(unittest.TestCase):
    def test_batches_cover_every_item_once_in_order(self):
        items = list(range(2501))
        chunks = list(scale.batches(items, 1000))
        self.assertEqual([len(chunk) for chunk in chunks], [1000, 1000, 501])
        self.assertEqual([item for chunk in chunks for item in chunk], items)
        self.assertEqual(scale.batch_count(2501, 1000), 3)
        self.assertEqual(scale.batch_count(0, 1000), 0)
        self.assertEqual(list(scale.batches([], 10)), [])

    def test_a_batch_size_below_one_is_refused(self):
        with self.assertRaises(SeedError):
            list(scale.batches([1], 0))


class Areas(unittest.TestCase):
    def test_areas_come_with_their_elections_and_ambiguous_names_are_left_out(self):
        areas = [
            {"id": "a1", "name": "North"},
            {"id": "a2", "name": "South"},
            {"id": "a3", "name": "Twin"},
            {"id": "a4", "name": "Twin"},
            {"id": "a5", "name": "Idle"},
        ]
        links = [
            {"area_id": "a1", "contest_id": "c1"},
            {"area_id": "a2", "contest_id": "c1"},
            {"area_id": "a2", "contest_id": "c2"},
            {"area_id": "a2", "contest_id": "c3"},
            {"area_id": "a3", "contest_id": "c1"},
        ]
        contests = [
            {"id": "c1", "election_id": "e1"},
            {"id": "c2", "election_id": "e2"},
            {"id": "c3", "election_id": "e2"},
        ]
        usable, warnings = scale.usable_areas(areas, links, contests)
        self.assertEqual(
            usable, [Area("a1", "North", ("e1",)), Area("a2", "South", ("e1", "e2"))]
        )
        self.assertTrue(any("Twin" in warning for warning in warnings))
        self.assertTrue(any("no election" in warning for warning in warnings))

    def test_areas_without_elections_are_used_only_when_none_has_one(self):
        usable, warnings = scale.usable_areas([{"id": "a1", "name": "Only"}], [], [])
        self.assertEqual(usable, [Area("a1", "Only", ())])
        self.assertTrue(any("counts none" in warning for warning in warnings))
        self.assertEqual(scale.usable_areas([], [], []), ([], []))


class Voters(unittest.TestCase):
    def setUp(self):
        self.voters = scale.generate_voters(10_000, AREAS, random.Random(7), ON)

    def test_voters_are_marked_and_unique(self):
        usernames = [voter.username for voter in self.voters]
        self.assertEqual(len(set(usernames)), 10_000)
        for voter in self.voters:
            self.assertTrue(voter.username.startswith(scale.PREFIX))
            self.assertTrue(voter.email.endswith(scale.EMAIL_DOMAIN))
            self.assertTrue(scale.is_seeded_user(vars(voter) | {"email": voter.email}))

    def test_the_same_seed_gives_the_same_voters(self):
        again = scale.generate_voters(10_000, AREAS, random.Random(7), ON)
        self.assertEqual(again, self.voters)

    def test_voters_spread_over_areas_countries_and_bands(self):
        self.assertEqual({voter.area for voter in self.voters}, set(AREAS))
        countries = Counter(
            voter.country.split("/")[0] for voter in self.voters if voter.country
        )
        self.assertGreaterEqual(len(countries), 100)
        for voter in self.voters:
            if voter.country:
                self.assertEqual(len(voter.country.split("/")), 2, voter.country)
        # Long-tailed: the first country has many more voters than the last.
        self.assertGreater(
            countries[scale.COUNTRIES[0]], 5 * countries.get(scale.COUNTRIES[-1], 1)
        )
        ages = [
            age(date.fromisoformat(voter.date_of_birth), ON) for voter in self.voters
        ]
        self.assertGreaterEqual(min(ages), 18)
        self.assertLessEqual(max(ages), 90)
        bands = Counter(
            age_band(a) for a in ages
        )
        for band, share in (("18-24", 0.15), ("25-39", 0.35), ("40-59", 0.35)):
            self.assertAlmostEqual(bands[band] / len(ages), share, delta=0.03)
        self.assertEqual({voter.sex for voter in self.voters}, {"M", "F", ""})
        verified = sum(voter.verified for voter in self.voters) / len(self.voters)
        self.assertAlmostEqual(verified, scale.VERIFIED_SHARE, delta=0.03)

    def test_a_birthday_on_29_february_still_gives_an_age(self):
        born = scale.birth_date(random.Random(1), date(2028, 2, 29))
        self.assertTrue(18 <= age(date.fromisoformat(born), date(2028, 2, 29)) <= 90)

    def test_no_area_is_refused(self):
        with self.assertRaises(SeedError):
            scale.generate_voters(1, [], random.Random(1), ON)


class Census(unittest.TestCase):
    def test_the_csv_is_what_the_importer_reads(self):
        voters = scale.generate_voters(50, AREAS, random.Random(3), ON)
        secret = scale.credential("pw", b"0123456789abcdef")
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "voters.csv"
            scale.write_census(path, voters, "voter", secret)
            with path.open(newline="", encoding="utf-8") as handle:
                rows = list(csv.reader(handle))
        header, body = rows[0], rows[1:]
        self.assertEqual(tuple(header), scale.CENSUS_COLUMNS)
        for column in header:
            self.assertRegex(column, HEADER)
        self.assertEqual(len(body), 50)
        for row, voter in zip(body, voters, strict=True):
            record = dict(zip(header, row, strict=True))
            self.assertEqual(record["username"], voter.username)
            self.assertEqual(record["area_name"], voter.area.name)
            self.assertEqual(record["group_name"], "voter")
            self.assertEqual(
                record["sequent.read-only.id-card-number-validated"],
                "VERIFIED" if voter.verified else "",
            )
            # The importer splits attribute values on "|".
            self.assertFalse(any("|" in cell for cell in row))

    def test_the_credential_is_keycloaks_pbkdf2_sha256(self):
        salt = b"0123456789abcdef"
        secret = scale.credential("Mon-scale-2026!", salt)
        self.assertEqual(base64.b64decode(secret.salt), salt)
        expected = hashlib.pbkdf2_hmac("sha256", b"Mon-scale-2026!", salt, 27_500, 32)
        self.assertEqual(base64.b64decode(secret.hashed), expected)
        self.assertEqual(secret.iterations, 27_500)


class Cleanup(unittest.TestCase):
    def test_only_accounts_with_both_marks_are_seeded(self):
        seeded = {
            "username": "mon-scale-000001",
            "email": "mon-scale-000001@mon-scale.invalid",
        }
        self.assertTrue(scale.is_seeded_user(seeded))
        for user in (
            {"username": "mon-scale-000001", "email": "someone@example.com"},
            {"username": "mon-scale-000001"},
            {"username": "voter-1", "email": "voter-1@mon-scale.invalid"},
            {"username": "x-mon-scale-1", "email": "x@mon-scale.invalid"},
            {"email": "mon-scale-1@mon-scale.invalid"},
        ):
            self.assertFalse(scale.is_seeded_user(user), user)

    def test_marked_rows_are_the_events_annotated_ones(self):
        self.assertEqual(
            scale.marked_rows(TENANT, EVENT),
            {
                "tenant_id": {"_eq": TENANT},
                "election_event_id": {"_eq": EVENT},
                "annotations": {"_has_key": scale.MARKER},
            },
        )

    def test_the_area_is_read_back_from_the_account(self):
        user = {"id": "u", "username": "mon-scale-1", "attributes": {"area-id": ["a1"]}}
        self.assertEqual(scale.seeded_account(user), Account("u", "mon-scale-1", "a1"))
        self.assertIsNone(scale.seeded_account({"id": "u", "username": "n"}).area_id)


class FakeKeycloak:
    """The admin REST calls the cleanup makes, against a list of users."""

    def __init__(self, users, groups=()):
        self.users = list(users)
        self.groups = list(groups)
        self.deleted = []

    def admin_token(self):
        return "token"

    def admin(self, method, path, json_body=None, expect=(200,)):
        route, _, query = path.partition("?")
        if method == "DELETE":
            self.deleted.append(route.rsplit("/", 1)[1])
            return None
        if route.endswith("/groups"):
            return self.groups
        arguments = dict(urllib.parse.parse_qsl(query))
        matching = [
            user for user in self.users if arguments["username"] in user["username"]
        ]
        first, count = int(arguments["first"]), int(arguments["max"])
        return matching[first : first + count]


def fake_stack(keycloak):
    stack = object.__new__(scale.Stack)
    stack.keycloak = keycloak
    stack.realm = "realm"
    stack.voter_group = "voter"
    return stack


class CleanupStack(unittest.TestCase):
    def test_only_seeded_users_are_listed_across_pages_and_deleted(self):
        seeded = [
            {"id": f"s{index}", "username": f"mon-scale-{index:06d}",
             "email": f"mon-scale-{index:06d}@mon-scale.invalid"}
            for index in range(2500)
        ]  # fmt: skip
        others = [
            {"id": "o1", "username": "mon-scale-lookalike", "email": "a@example.com"},
            {"id": "o2", "username": "voter-1", "email": "v@mon-scale.invalid"},
        ]
        keycloak = FakeKeycloak(seeded[:1200] + others + seeded[1200:])
        stack = fake_stack(keycloak)
        users = stack.seeded_users()
        self.assertEqual([user["id"] for user in users], [u["id"] for u in seeded])
        self.assertEqual(stack.delete_users(users, workers=4), 2500)
        self.assertEqual(sorted(keycloak.deleted), sorted(u["id"] for u in seeded))

    def test_deleting_an_unmarked_user_is_refused(self):
        keycloak = FakeKeycloak([])
        stack = fake_stack(keycloak)
        with self.assertRaises(SeedError):
            stack.delete_users([{"id": "o1", "username": "voter-1"}], workers=2)
        self.assertEqual(keycloak.deleted, [])

    def test_the_voter_group_must_exist(self):
        fake_stack(FakeKeycloak([], [{"name": "voter"}])).check_voter_group()
        with self.assertRaises(SeedError):
            fake_stack(FakeKeycloak([], [{"name": "voters"}])).check_voter_group()


class Votes(unittest.TestCase):
    def plan(self, count=6000, turnout=0.6, revote_share=0.1):
        return scale.plan_votes(
            accounts(count),
            {area.id: area for area in AREAS},
            LIMITS,
            VotePlan(turnout, revote_share, 5, NOW),
            random.Random(11),
            TENANT,
            EVENT,
            "tag",
        )

    def test_revotes_follow_the_trigger(self):
        self.assertFalse(scale.allows_revote(None))
        self.assertFalse(scale.allows_revote(1))
        self.assertTrue(scale.allows_revote(0))
        self.assertTrue(scale.allows_revote(2))

    def test_votes_are_marked_valid_and_in_the_voters_elections(self):
        votes = self.plan()
        areas = {area.id: area for area in AREAS}
        owner = {account.user_id: account for account in accounts(6000)}
        start = NOW - timedelta(days=5)
        for vote in votes:
            self.assertEqual(vote["annotations"], {scale.MARKER: "tag"})
            self.assertEqual(vote["status"], "valid")
            self.assertEqual(vote["tenant_id"], TENANT)
            self.assertEqual(vote["election_event_id"], EVENT)
            area = areas[vote["area_id"]]
            self.assertEqual(owner[vote["voter_id_string"]].area_id, area.id)
            self.assertIn(vote["election_id"], area.elections)
            at = datetime.fromisoformat(vote["created_at"])
            self.assertTrue(start <= at < NOW, at)
        days = {datetime.fromisoformat(vote["created_at"]).date() for vote in votes}
        self.assertGreaterEqual(len(days), 5)

    def test_turnout_and_revotes(self):
        votes = self.plan()
        voters = {vote["voter_id_string"] for vote in votes}
        self.assertAlmostEqual(len(voters) / 6000, 0.6, delta=0.03)
        per_election = Counter(
            (vote["voter_id_string"], vote["election_id"]) for vote in votes
        )
        self.assertLessEqual(max(per_election.values()), 2)
        revoted = {election for (_, election), n in per_election.items() if n > 1}
        self.assertEqual(revoted, {"e-twice", "e-unlimited"})

    def test_no_turnout_no_votes(self):
        self.assertEqual(self.plan(turnout=0.0), [])

    def test_voters_without_a_known_area_do_not_vote(self):
        stray = [Account("u", "mon-scale-1", None), Account("v", "mon-scale-2", "gone")]
        votes = scale.plan_votes(
            stray,
            {area.id: area for area in AREAS},
            LIMITS,
            VotePlan(1.0, 1.0, 5, NOW),
            random.Random(1),
            TENANT,
            EVENT,
            "tag",
        )
        self.assertEqual(votes, [])


class Applications(unittest.TestCase):
    def test_applications_carry_statuses_reasons_and_the_marker(self):
        applications = scale.plan_applications(
            accounts(5000), 0.3, 5, NOW, random.Random(5), TENANT, EVENT, "tag"
        )
        self.assertAlmostEqual(len(applications) / 5000, 0.3, delta=0.03)
        statuses = Counter(application["status"] for application in applications)
        self.assertEqual(set(statuses), {"ACCEPTED", "REJECTED", "PENDING"})
        for application in applications:
            annotations = application["annotations"]
            self.assertEqual(annotations[scale.MARKER], "tag")
            created = datetime.fromisoformat(application["created_at"])
            updated = datetime.fromisoformat(application["updated_at"])
            self.assertLessEqual(created, updated)
            self.assertLessEqual(updated, NOW)
            if application["status"] == "REJECTED":
                self.assertIn(annotations["rejection_reason"], scale.REJECTION_REASONS)
            else:
                self.assertNotIn("rejection_reason", annotations)
            if application["status"] == "PENDING":
                self.assertEqual(created, updated)


class Command(unittest.TestCase):
    def run_main(self, *argv):
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = scale.main(list(argv))
        return code, out.getvalue(), err.getvalue()

    def test_an_offline_dry_run_plans_without_the_stack(self):
        before = set(sys.modules)
        code, out, _ = self.run_main(
            "--event", EVENT, "--tenant", TENANT, "--voters", "25000",
            "--with-votes", "--with-applications", "--dry-run", "--offline",
        )  # fmt: skip
        self.assertEqual(code, 0)
        self.assertIn(f"tenant-{TENANT}-event-{EVENT}", out)
        self.assertIn("3 file(s) of up to 10000", out)
        self.assertIn("~15000 valid first votes", out)
        self.assertIn("~7500", out)
        loaded = set(sys.modules) - before
        self.assertNotIn("scripts.e2e.journeys.client", loaded)

    def test_unsafe_requests_are_refused(self):
        code, _, err = self.run_main(
            "--event", EVENT, "--tenant", TENANT, "--voters", "60000",
            "--dry-run", "--offline",
        )  # fmt: skip
        self.assertEqual(code, 1)
        self.assertIn("--force", err)
        code, _, err = self.run_main("--event", EVENT, "--tenant", TENANT, "--offline")
        self.assertEqual(code, 1)
        self.assertIn("--dry-run", err)
        code, _, err = self.run_main("--event", "x", "--dry-run", "--offline")
        self.assertEqual(code, 1)
        code, _, err = self.run_main(
            "--event", EVENT, "--tenant", TENANT, "--cleanup", "--dry-run", "--offline"
        )
        self.assertEqual(code, 1)
        self.assertIn("needs the stack", err)


if __name__ == "__main__":
    unittest.main()
