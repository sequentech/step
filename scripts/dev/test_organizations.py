# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Tests for scripts.dev.scenario.organizations: the signing organizations'
fixtures. Expectations come from each organization's own file, so a value
hardcoded in the builder fails one of the two organizations."""

import csv
import io
import json
import tempfile
import unittest
from pathlib import Path

from scripts.dev.scenario import organizations

PRESET = organizations.ROOT / (
    "packages/windmill/external-bin/janitor/templates/COMELEC/signing.json"
)


def users_of(profile):
    return list(csv.DictReader(io.StringIO(organizations.admin_users_csv(profile))))


class OrganizationsTest(unittest.TestCase):
    def test_both_organizations_are_listed(self):
        self.assertEqual(
            organizations.names(), ["post-qualification", "student-council"]
        )

    def test_unknown_organization_is_refused(self):
        with self.assertRaises(organizations.OrganizationError):
            organizations.load("an-organization-nobody-wrote")

    def test_post_qualification_has_three_sbei_accounts_per_post(self):
        # SW-F-0300: four Posts and three SBEI credentials for each Post.
        profile = organizations.load("post-qualification")
        self.assertEqual(len(profile["posts"]), 4)
        users = users_of(profile)
        for post in profile["posts"]:
            label = organizations.post_label(profile, post)
            of_post = [user for user in users if user["permission_labels"] == label]
            self.assertEqual(len(of_post), 3, post["name"])
            self.assertEqual({user["group_name"] for user in of_post}, {"sbei"})
            self.assertEqual(
                [user["title"] for user in of_post],
                profile["signers"][0]["per_post"],
            )

    def test_sbei_titles_are_the_ones_the_janitor_preset_gives(self):
        # The demo and a janitor delivery title their SBEIs alike.
        preset = json.loads(PRESET.read_text())
        profile = organizations.load("post-qualification")
        self.assertEqual(
            profile["signers"][0]["per_post"],
            [preset["sbei_titles"][role] for role in sorted(preset["sbei_titles"])],
        )

    def test_post_qualification_groups_are_the_client_realms(self):
        # The demo's groups sign what the janitor's client realm lets them.
        realm = json.loads((PRESET.parent / "keycloakAdmin.hbs").read_text())
        signs = {
            group["name"]: [
                role for role in group.get("realmRoles", []) if role.startswith("sign-")
            ]
            for group in realm["groups"]
        }
        for group in organizations.load("post-qualification")["groups"]:
            self.assertEqual(group["permissions"], signs[group["name"]], group["name"])

    def test_each_organization_can_satisfy_its_own_rules(self):
        # Enough distinct signers for every rule that needs signatures: per
        # Post for Post actions, event-wide for the configuration, and a
        # trustee for the trustee steps.
        trustee_actions = {"key-ceremony", "tally-key"}
        event_actions = {"approve-configuration"}
        for name in organizations.names():
            profile = organizations.load(name)
            permissions = {
                group["name"]: set(group["permissions"]) for group in profile["groups"]
            }
            users = users_of(profile)
            every_label = {
                organizations.post_label(profile, post) for post in profile["posts"]
            }
            for rule in profile["signing_rules"]:
                if rule["requirement"] != "required":
                    continue
                action = rule["action"]
                signers = [
                    user
                    for user in users
                    if f"sign-{action}" in permissions[user["group_name"]]
                ]
                needed = 1 if action in trustee_actions else rule["signatures"]
                if action in trustee_actions or action in event_actions:
                    holding = [
                        user
                        for user in signers
                        if set(user["permission_labels"].split("|")) == every_label
                    ]
                    self.assertGreaterEqual(len(holding), needed, f"{name} {action}")
                    continue
                for post in profile["posts"]:
                    label = organizations.post_label(profile, post)
                    of_post = [
                        user
                        for user in signers
                        if label in user["permission_labels"].split("|")
                    ]
                    self.assertGreaterEqual(
                        len(of_post), needed, f"{name} {action} {post['name']}"
                    )

    def test_the_student_councils_issuer_key_matches_its_certificate(self):
        profile = organizations.load("student-council")
        key = (organizations.DIRECTORY / profile["issuer_key"]).read_text()
        self.assertIn("PRIVATE KEY", key)
        license_text = (
            organizations.DIRECTORY / f"{profile['issuer_key']}.license"
        ).read_text()
        self.assertIn("TEST ONLY", license_text)

    def test_rules_come_from_the_preset_or_the_organization(self):
        preset = json.loads(PRESET.read_text())
        qualification = organizations.load("post-qualification")
        self.assertEqual(qualification["signing_rules"], preset["signing_rules"])
        self.assertEqual(qualification["signing_checks"], preset["signing_checks"])
        council = organizations.load("student-council")
        raw = json.loads((organizations.DIRECTORY / "student-council.json").read_text())
        self.assertEqual(council["signing_rules"], raw["signing_rules"])
        self.assertNotEqual(council["signing_rules"], qualification["signing_rules"])

    def test_every_signer_holds_the_labels_of_the_posts_they_sign_for(self):
        for name in organizations.names():
            profile = organizations.load(name)
            labels = {
                organizations.post_label(profile, post) for post in profile["posts"]
            }
            groups = {group["name"] for group in profile["groups"]}
            for user in users_of(profile):
                held = set(user["permission_labels"].split("|"))
                self.assertTrue(held, user["username"])
                self.assertLessEqual(held, labels, user["username"])
                self.assertIn(user["group_name"], groups)
                self.assertTrue(user["title"], user["username"])
                self.assertEqual(user["password"], profile["password"])

    def test_event_wide_signers_hold_every_post(self):
        profile = organizations.load("student-council")
        labels = {organizations.post_label(profile, post) for post in profile["posts"]}
        commission = [
            user
            for user in users_of(profile)
            if user["group_name"] == "Electoral Commission"
        ]
        self.assertEqual(len(commission), 3)
        for user in commission:
            self.assertEqual(set(user["permission_labels"].split("|")), labels)

    def test_usernames_are_unique(self):
        for name in organizations.names():
            usernames = [
                user["username"] for user in users_of(organizations.load(name))
            ]
            self.assertEqual(len(usernames), len(set(usernames)), name)

    def test_the_bundle_has_one_election_and_area_per_post(self):
        for name in organizations.names():
            profile = organizations.load(name)
            bundle = organizations.election_event(profile, "http://127.0.0.1:3000", "t")
            posts = profile["posts"]
            self.assertEqual(len(bundle["elections"]), len(posts))
            self.assertEqual(len(bundle["areas"]), len(posts))
            self.assertEqual(
                sorted(
                    election["permission_label"] for election in bundle["elections"]
                ),
                sorted(organizations.post_label(profile, post) for post in posts),
            )
            self.assertEqual(
                sorted(area["name"] for area in bundle["areas"]),
                sorted(post["name"] for post in posts),
            )
            election_ids = {election["id"] for election in bundle["elections"]}
            contest_ids = {contest["id"] for contest in bundle["contests"]}
            area_ids = {area["id"] for area in bundle["areas"]}
            for contest in bundle["contests"]:
                self.assertIn(contest["election_id"], election_ids)
            for link in bundle["area_contests"]:
                self.assertIn(link["area_id"], area_ids)
                self.assertIn(link["contest_id"], contest_ids)
            self.assertEqual(bundle["signing_rules"], profile["signing_rules"])
            self.assertEqual(bundle["signing_checks"], profile["signing_checks"])

    def test_tenant_settings_carry_the_display_name_and_overrides(self):
        council = organizations.tenant_settings(organizations.load("student-council"))
        raw = json.loads((organizations.DIRECTORY / "student-council.json").read_text())
        self.assertEqual(council["display_name"], raw["tenant"]["display_name"])
        self.assertEqual(
            council["i18n"]["en"][
                "adminPortal:signing.actions.generate-election-returns.label"
            ],
            "Certify faculty results",
        )
        self.assertEqual(
            council["i18n"]["en"]["adminPortal:signing.terms.post"], "Faculty"
        )

    def test_realm_groups_grant_each_group_its_sign_permissions(self):
        for name in organizations.names():
            profile = organizations.load(name)
            realm = organizations.realm_groups(profile)
            self.assertEqual(realm["ifResourceExists"], "SKIP")
            self.assertEqual(
                {group["name"]: group["realmRoles"] for group in realm["groups"]},
                {group["name"]: group["permissions"] for group in profile["groups"]},
            )

    def test_write_produces_the_loadable_files(self):
        with tempfile.TemporaryDirectory() as directory:
            written = organizations.write("student-council", Path(directory))
            names = sorted(path.name for path in written)
            self.assertEqual(
                names,
                [
                    "admin-users.csv",
                    "election-event.json",
                    "realm-groups.json",
                    "tenant-settings.json",
                    "trusted-issuer.pem",
                ],
            )
            issuer = (
                Path(directory) / "student-council" / "trusted-issuer.pem"
            ).read_text()
            self.assertIn("BEGIN CERTIFICATE", issuer)
            written = organizations.write("post-qualification", Path(directory))
            self.assertNotIn("trusted-issuer.pem", [path.name for path in written])


if __name__ == "__main__":
    unittest.main()
