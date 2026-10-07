# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""
The COMELEC lifecycle preset and how the janitor applies it (VOTE-LIFECYCLE).
Run with `python3 -m unittest test_lifecycle_preset` from this directory.
"""

import unittest
from datetime import datetime, timedelta, timezone
from zoneinfo import ZoneInfo

import lifecycle_preset
from patch import canonical_zone

PRESET = lifecycle_preset.load("templates/COMELEC/lifecycle.json")


class ComelecPresetTest(unittest.TestCase):
    def test_eighty_configured_zones_for_the_104_posts_with_manila_primary(self):
        zones = PRESET["timezones"]
        posts = PRESET["posts"]
        self.assertEqual(len(posts), 104)
        self.assertEqual(len({lifecycle_preset.post_of(p["post"]) for p in posts}), 104)
        self.assertEqual(zones["primary"], "Asia/Manila")
        self.assertEqual(zones["logs"], "primary")
        self.assertEqual(len(zones["configured"]), 80)
        self.assertEqual(len(set(zones["configured"])), 80)
        self.assertIn(zones["primary"], zones["configured"])
        # Every configured zone is a Post's zone or the primary, and every
        # Post's zone is configured.
        post_zones = {p["timezone"] for p in posts}
        self.assertEqual(post_zones | {zones["primary"]}, set(zones["configured"]))

    def test_every_zone_is_a_canonical_tz_database_name(self):
        for zone in PRESET["timezones"]["configured"]:
            self.assertEqual(canonical_zone(zone), zone)
            ZoneInfo(zone)

    def test_the_zones_the_ticket_names_are_there(self):
        # +05:30, +05:45, +03:30, +06:30, Cairo's DST and both ends of the
        # day, at the opening of 9 April 2028.
        opening = datetime(2028, 4, 9, tzinfo=timezone.utc)
        offsets = {opening.astimezone(ZoneInfo(p["timezone"])).utcoffset() for p in PRESET["posts"]}
        for hours, minutes in [(5, 30), (5, 45), (3, 30), (6, 30)]:
            self.assertIn(timedelta(hours=hours, minutes=minutes), offsets)
        zones = set(PRESET["timezones"]["configured"])
        for zone in ["Africa/Cairo", "America/Toronto", "Pacific/Honolulu", "Pacific/Pago_Pago"]:
            self.assertIn(zone, zones)

    def test_synthetic_additions_are_explicit_and_not_annex_a(self):
        self.assertEqual(PRESET["fixture_provenance"]["status"], "synthetic-expanded-fixture")
        self.assertEqual(sum(p["source"] == "workbook" for p in PRESET["posts"]), 77)
        self.assertEqual(sum(p["source"] == "synthetic" for p in PRESET["posts"]), 27)
        self.assertIn("Official Annex A was not available", PRESET["fixture_provenance"]["limitation"])

    def test_the_policies(self):
        self.assertEqual(
            PRESET["lifecycle_policies"],
            {"initialization_scope": "post-and-country", "unsigned_scheduled_close": "run-as-system"},
        )


class ApplyTest(unittest.TestCase):
    def test_the_event_gets_the_preset_unless_it_has_its_own(self):
        event = lifecycle_preset.apply_event({"presentation": {"locked_down": "not-locked-down"}}, PRESET)
        self.assertEqual(event["presentation"]["timezones"], PRESET["timezones"])
        self.assertEqual(event["presentation"]["lifecycle_policies"], PRESET["lifecycle_policies"])
        self.assertEqual(event["presentation"]["locked_down"], "not-locked-down")
        # A copy: changing the event doesn't change the preset.
        event["presentation"]["timezones"]["configured"].append("UTC")
        self.assertEqual(len(PRESET["timezones"]["configured"]), 80)

        own = {"configured": ["UTC"], "primary": "UTC", "logs": "election"}
        event = lifecycle_preset.apply_event({"presentation": {"timezones": own}}, PRESET)
        self.assertEqual(event["presentation"]["timezones"], own)
        self.assertEqual(event["presentation"]["lifecycle_policies"], PRESET["lifecycle_policies"])

    def test_elections_get_their_posts_zone_unless_the_sheet_gave_one(self):
        elections = [
            {"alias": "DUBAI PCG - General Election", "presentation": {}},
            {"alias": "DUBAI PCG - Test Voting", "presentation": {}},
            {"alias": "Kathmandu PCG - General Election", "presentation": {}},
            {"alias": "TOKYO PE - General Election", "presentation": {"timezone": "Asia/Manila"}},
            {"alias": "NOWHERE PE - General Election", "presentation": {}},
        ]
        with self.assertLogs(level="WARNING"):
            lifecycle_preset.apply_posts(elections, PRESET)
        zones = [election["presentation"].get("timezone") for election in elections]
        self.assertEqual(zones, ["Asia/Dubai", "Asia/Dubai", "Asia/Kathmandu", "Asia/Manila", None])


if __name__ == "__main__":
    unittest.main()
