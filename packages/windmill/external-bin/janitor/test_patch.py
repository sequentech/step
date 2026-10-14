# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""
Scheduled events of the janitor bundle in local time (VOTE-LIFECYCLE).
Run with `python3 -m unittest test_patch` from this directory.
"""

import unittest
from datetime import datetime, timezone
from zoneinfo import ZoneInfo

from patch import apply_schedule_time_zones, schedule_cron_config

TENANT = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5"
EVENT = "e0000000-0000-4000-8000-000000000000"

# Whole hours, the fractional offsets and Cairo.
POST_ZONES = [
    "Asia/Dubai",
    "Asia/Kolkata",
    "Asia/Kathmandu",
    "Asia/Tehran",
    "Asia/Yangon",
    "Africa/Cairo",
    "America/Toronto",
]

OVERSEAS = {"primary": "Asia/Manila", "posts": POST_ZONES}
ASSOCIATION = {"primary": "Europe/Madrid", "posts": ["Europe/Madrid", "Atlantic/Canary"]}


def election_id(index):
    return f"00000000-0000-4000-8000-{index:012}"


def scheduled_event(processor, date, election=None, zone=None):
    """A scheduled event as templates/scheduledEvent.hbs renders it."""
    return {
        "event_processor": processor,
        "cron_config": {"cron": None, "scheduled_date": date, "timezone": zone},
        "event_payload": {"election_id": election},
        "task_id": f"tenant_{TENANT}_event_{EVENT}_"
        + (f"election_{election}_" if election else "")
        + processor,
    }


def bundle(config, scheduled_events):
    zones = [config["primary"]] + [z for z in config["posts"] if z != config["primary"]]
    return {
        "election_event": {
            "presentation": {
                "timezones": {"configured": zones, "primary": config["primary"], "logs": "election"}
            }
        },
        "elections": [
            {"id": election_id(index), "alias": f"post-{index}", "presentation": {"timezone": zone}}
            for index, zone in enumerate(config["posts"])
        ],
        "scheduled_events": scheduled_events,
    }


def expected(local, zone_name):
    zone = ZoneInfo(zone_name)
    instant = datetime.fromisoformat(local).replace(tzinfo=zone).astimezone(timezone.utc)
    return instant, zone


class ScheduleTimeZonesTest(unittest.TestCase):
    def test_post_rows_open_at_midnight_local_and_the_close_is_in_the_primary(self):
        for config in (OVERSEAS, ASSOCIATION):
            events = [
                scheduled_event("START_VOTING_PERIOD", "2028-04-09T00:00", election_id(index))
                for index in range(len(config["posts"]))
            ]
            events.append(scheduled_event("END_VOTING_PERIOD", "2028-05-08 19:00"))
            data = bundle(config, events)
            apply_schedule_time_zones(data)
            for index, zone_name in enumerate(config["posts"]):
                cron = data["scheduled_events"][index]["cron_config"]
                instant, zone = expected("2028-04-09T00:00", zone_name)
                self.assertEqual(cron["local"], "2028-04-09T00:00")
                self.assertEqual(cron["timezone"], zone_name)
                stored = datetime.fromisoformat(cron["scheduled_date"].replace("Z", "+00:00"))
                self.assertEqual(stored, instant)
                self.assertEqual(stored.utcoffset(), instant.astimezone(zone).utcoffset())
            close = data["scheduled_events"][-1]
            self.assertEqual(close["cron_config"]["timezone"], config["primary"])
            self.assertEqual(close["cron_config"]["local"], "2028-05-08T19:00")
            self.assertEqual(
                close["task_id"], f"tenant_{TENANT}_event_{EVENT}_END_VOTING_PERIOD"
            )
            self.assertEqual(close["event_payload"], {"election_id": None})

    def test_fractional_offsets_are_kept_in_the_instant(self):
        for zone, offset in [
            ("Asia/Kolkata", "+05:30"),
            ("Asia/Kathmandu", "+05:45"),
            ("Asia/Tehran", "+03:30"),
            ("Asia/Yangon", "+06:30"),
        ]:
            cron = schedule_cron_config("2028-04-09T00:00", zone, "UTC")
            self.assertEqual(cron["scheduled_date"], f"2028-04-09T00:00:00{offset}")

    def test_the_row_zone_wins_and_aliases_are_canonical(self):
        data = bundle(
            OVERSEAS,
            [scheduled_event("START_VOTING_PERIOD", "2028-04-09T00:00", election_id(0), "Asia/Calcutta")],
        )
        apply_schedule_time_zones(data)
        self.assertEqual(data["scheduled_events"][0]["cron_config"]["timezone"], "Asia/Kolkata")

    def test_an_instant_keeps_its_moment_and_gains_the_wall_time(self):
        cron = schedule_cron_config("2028-04-08T20:00:00+00:00", None, "Asia/Dubai")
        self.assertEqual(cron["local"], "2028-04-09T00:00")
        self.assertEqual(cron["scheduled_date"], "2028-04-09T00:00:00+04:00")

    def test_a_date_cell_is_a_wall_time(self):
        cron = schedule_cron_config(datetime(2028, 4, 9), None, "Europe/Madrid")
        self.assertEqual(cron["scheduled_date"], "2028-04-09T00:00:00+02:00")
        cron = schedule_cron_config("2028-01-09T00:00", None, "Europe/London")
        self.assertEqual(cron["scheduled_date"], "2028-01-09T00:00:00Z")

    def test_a_time_clocks_skip_is_refused_and_a_repeated_one_uses_the_first(self):
        with self.assertRaises(ValueError):
            schedule_cron_config("2028-03-12T02:30", "America/Toronto", "UTC")
        with self.assertRaises(ValueError):
            schedule_cron_config("2028-04-28T00:30", "Africa/Cairo", "UTC")
        cron = schedule_cron_config("2028-11-05T01:30", "America/Toronto", "UTC")
        self.assertEqual(cron["scheduled_date"], "2028-11-05T01:30:00-04:00")

    def test_unknown_zones_and_unconfigured_post_zones_are_refused(self):
        with self.assertRaises(ValueError):
            schedule_cron_config("2028-04-09T00:00", "Mars/Olympus", "UTC")
        data = bundle(ASSOCIATION, [])
        data["elections"][0]["presentation"]["timezone"] = "Asia/Tokyo"
        with self.assertRaises(ValueError):
            apply_schedule_time_zones(data)

    def test_a_trailing_z_is_utc(self):
        cron = schedule_cron_config("2028-04-08T20:00:00Z", None, "Asia/Dubai")
        self.assertEqual(cron["scheduled_date"], "2028-04-09T00:00:00+04:00")

    def test_links_are_canonical_and_abbreviations_refused(self):
        cron = schedule_cron_config("2028-04-09T00:00", "US/Eastern", "UTC")
        self.assertEqual(cron["timezone"], "America/New_York")
        for refused in ("EST", "Japan"):
            with self.assertRaises(ValueError):
                schedule_cron_config("2028-04-09T00:00", refused, "UTC")

    def test_configured_and_post_zones_compare_canonical(self):
        data = bundle(ASSOCIATION, [scheduled_event("START_VOTING_PERIOD", "2028-04-09T00:00", election_id(0))])
        data["election_event"]["presentation"]["timezones"]["configured"].append("Asia/Calcutta")
        data["elections"][0]["presentation"]["timezone"] = "Asia/Kolkata"
        apply_schedule_time_zones(data)
        self.assertEqual(data["scheduled_events"][0]["cron_config"]["timezone"], "Asia/Kolkata")


if __name__ == "__main__":
    unittest.main()
