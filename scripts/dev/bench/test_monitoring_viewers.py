# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

import base64
import json
import unittest

from .__main__ import parser
from .monitoring_viewers import (
    PROFILES,
    Cycle,
    ScreenProfile,
    ViewerSetup,
    count_renderer_calls,
    latency_summary,
    markdown_table,
    parse_docker_stats,
    parse_table_stats,
    percentile,
    read_deltas,
    render_error,
    render_requests,
    selector_values,
    token_expiry,
    viewer_counts,
    viewer_setups,
    widget_width,
)


class ViewerCountsTest(unittest.TestCase):
    def test_a_comma_separated_list(self):
        self.assertEqual(viewer_counts("10, 100,1000"), [10, 100, 1000])

    def test_counts_must_be_positive(self):
        with self.assertRaises(ValueError):
            viewer_counts("10,0")
        with self.assertRaises(ValueError):
            viewer_counts("")


class WidthTest(unittest.TestCase):
    def test_a_half_width_cell_on_a_wide_screen(self):
        profile = ScreenProfile(
            "desktop", container_width=1600, stacked=False, weight=1
        )
        # (1600 * 6 / 12) - 16 gutter - 32 padding = 752, down to 40 px steps.
        self.assertEqual(widget_width(profile, 6), 720)
        self.assertEqual(widget_width(profile, 12), 1520)

    def test_stacked_screens_draw_every_cell_full_width(self):
        profile = ScreenProfile("phone", container_width=358, stacked=True, weight=1)
        self.assertEqual(widget_width(profile, 6), widget_width(profile, 12))
        # Never narrower than the portal's minimum render width.
        self.assertEqual(widget_width(profile, 6), 360)

    def test_the_default_profiles_give_a_few_buckets(self):
        widths = {widget_width(profile, 6) for profile in PROFILES} | {
            widget_width(profile, 12) for profile in PROFILES
        }
        self.assertGreaterEqual(len(widths), 4)
        self.assertLessEqual(len(widths), 12)


class SetupTest(unittest.TestCase):
    def setups(self, count=1000, seed=7, post_share=0.2):
        return viewer_setups(
            count,
            seed=seed,
            dark_share=0.3,
            locales={"en-US": 0.8, "es": 0.2},
            post_share=post_share,
            posts=["p1", "p2"],
        )

    def test_the_same_seed_gives_the_same_viewers(self):
        self.assertEqual(self.setups(), self.setups())
        self.assertNotEqual(self.setups(seed=7), self.setups(seed=8))

    def test_shares_are_roughly_kept(self):
        setups = self.setups()
        dark = sum(setup.color_scheme == "DARK" for setup in setups)
        spanish = sum(setup.locale == "es" for setup in setups)
        posts = sum(setup.post is not None for setup in setups)
        self.assertTrue(230 < dark < 370, dark)
        self.assertTrue(140 < spanish < 260, spanish)
        self.assertTrue(140 < posts < 260, posts)
        self.assertEqual({setup.index for setup in setups}, set(range(1000)))

    def test_no_posts_means_every_viewer_sees_the_whole_event(self):
        setups = viewer_setups(
            50,
            seed=1,
            dark_share=0.0,
            locales={"en": 1.0},
            post_share=1.0,
            posts=[],
        )
        self.assertEqual({setup.post for setup in setups}, {None})
        self.assertEqual({setup.color_scheme for setup in setups}, {"LIGHT"})


VOTING_ACTIVITY = {
    "selectors": {
        "day": {
            "options_from": "event_days",
            "when": {"selector": "grain", "in": ["hour"]},
        },
        "grain": {"default": "hour", "options": {"day": "Daily", "hour": "Hourly"}},
    }
}
GRAIN_FIRST = {
    "selectors": {
        "grain": {"default": "hour", "options": {"day": "Daily", "hour": "Hourly"}},
        "day": {
            "options_from": "event_days",
            "when": {"selector": "grain", "in": ["hour"]},
        },
    }
}


class SelectorValuesTest(unittest.TestCase):
    def test_no_selectors(self):
        self.assertEqual(selector_values({}, {}, []), {})

    def test_the_default_then_the_dashboard_value(self):
        widget = {
            "selectors": {
                "milestone": {
                    "default": "opened",
                    "options": {"closed": "", "opened": ""},
                }
            }
        }
        self.assertEqual(selector_values(widget, {}, []), {"milestone": "opened"})
        self.assertEqual(
            selector_values(widget, {"milestone": "closed"}, []),
            {"milestone": "closed"},
        )
        # A value that is not an option falls through, as in the portal.
        self.assertEqual(
            selector_values(widget, {"milestone": "gone"}, []), {"milestone": "opened"}
        )

    def test_the_first_option_without_a_default(self):
        widget = {"selectors": {"by": {"options": {"category": "", "post": ""}}}}
        self.assertEqual(selector_values(widget, {}, []), {"by": "category"})

    def test_a_condition_on_a_later_selector_hides_it(self):
        # The portal walks selectors in document order; `day` names `grain`,
        # which is not settled yet, so it is hidden.
        self.assertEqual(
            selector_values(VOTING_ACTIVITY, {}, ["2026-09-30"]), {"grain": "hour"}
        )

    def test_dynamic_options_take_the_latest_day(self):
        self.assertEqual(
            selector_values(GRAIN_FIRST, {}, ["2026-09-29", "2026-09-30"]),
            {"grain": "hour", "day": "2026-09-30"},
        )
        self.assertEqual(
            selector_values(GRAIN_FIRST, {"grain": "day"}, ["2026-09-30"]),
            {"grain": "day"},
        )
        # No days yet: the selector has no value and is not sent.
        self.assertEqual(selector_values(GRAIN_FIRST, {}, []), {"grain": "hour"})


DASHBOARD = {
    "dashboard": {
        "id": "overview",
        "layout": [
            {"widget": "summary", "width": 12},
            {"widget": "status", "width": 6, "values": {"milestone": "closed"}},
            {"widget": "missing", "width": 6},
        ],
    },
    "widgets": {
        "summary": {"definition": {"id": "summary", "source": "voter_turnout"}},
        "status": {
            "definition": {
                "id": "status",
                "source": "poll_status",
                "selectors": {
                    "milestone": {
                        "default": "opened",
                        "options": {"closed": "", "opened": ""},
                    }
                },
            }
        },
    },
    "sources": {
        "voter_turnout": {"producer": "CONNECTED"},
        "poll_status": {"producer": "CONNECTED"},
    },
    "snapshot": {"revision": 183},
    "event_days": ["2026-09-30"],
}


class RenderRequestsTest(unittest.TestCase):
    def setup(self, post=None):
        return ViewerSetup(
            index=0,
            profile=ScreenProfile("desktop", 1600, False, 1),
            color_scheme="DARK",
            locale="es",
            post=post,
        )

    def test_one_request_per_placed_widget_the_portal_would_draw(self):
        requests = render_requests("event-1", DASHBOARD, self.setup())
        self.assertEqual(
            [request["widgetId"] for request in requests], ["summary", "status"]
        )
        summary, status = requests
        self.assertEqual(
            summary,
            {
                "electionEventId": "event-1",
                "electionId": None,
                "dashboardId": "overview",
                "widgetId": "summary",
                "scope": {},
                "selectorValues": {},
                "snapshotRevision": 183,
                "width": 1520,
                "colorScheme": "DARK",
                "locale": "es",
            },
        )
        self.assertEqual(status["width"], 720)
        self.assertEqual(status["selectorValues"], {"milestone": "closed"})

    def test_a_viewer_on_a_post(self):
        requests = render_requests("event-1", DASHBOARD, self.setup(post="p1"))
        self.assertEqual(
            {str(request["scope"]) for request in requests}, {"{'post': 'p1'}"}
        )

    def test_a_source_that_is_not_connected_is_not_asked(self):
        dashboard = json.loads(json.dumps(DASHBOARD))
        dashboard["sources"]["poll_status"]["producer"] = "NOT_CONNECTED"
        requests = render_requests("event-1", dashboard, self.setup())
        self.assertEqual([request["widgetId"] for request in requests], ["summary"])


class RenderErrorTest(unittest.TestCase):
    def test_states_a_viewer_would_see_as_broken(self):
        self.assertIsNone(render_error({"state": "RENDERED"}))
        self.assertIsNone(render_error({"state": "NOT_CONNECTED"}))
        self.assertIsNone(render_error({"state": "SCOPE_PENDING"}))
        self.assertEqual(
            render_error({"state": "RENDER_FAILED", "reason": "RENDER_TIMEOUT"}),
            "RENDER_FAILED:RENDER_TIMEOUT",
        )
        self.assertEqual(render_error({"state": "INVALID"}), "INVALID")


class PercentileTest(unittest.TestCase):
    def test_nearest_rank(self):
        values = list(range(1, 101))
        self.assertEqual(percentile(values, 50), 50)
        self.assertEqual(percentile(values, 95), 95)
        self.assertEqual(percentile(values, 99), 99)
        self.assertEqual(percentile(values, 100), 100)
        self.assertEqual(percentile([3.0], 99), 3.0)
        self.assertIsNone(percentile([], 50))

    def test_summary_in_milliseconds(self):
        summary = latency_summary([0.010, 0.020, 0.030, 0.040])
        self.assertEqual(summary["n"], 4)
        self.assertEqual(summary["p50_ms"], 20.0)
        self.assertEqual(summary["p99_ms"], 40.0)
        self.assertEqual(summary["max_ms"], 40.0)
        self.assertEqual(latency_summary([])["n"], 0)


class TableStatsTest(unittest.TestCase):
    TSV = (
        "sequent_backend\tcast_vote\t10\t100\t5\t50\t3\n"
        "sequent_backend\tapplications\t1\t0\t0\t0\t0\n"
    )

    def test_parses_psql_unaligned_output(self):
        stats = parse_table_stats(self.TSV)
        self.assertEqual(
            stats["sequent_backend.cast_vote"],
            {
                "seq_scan": 10,
                "seq_tup_read": 100,
                "idx_scan": 5,
                "idx_tup_fetch": 50,
                "n_tup_ins": 3,
            },
        )

    def test_null_counters_are_zero(self):
        stats = parse_table_stats("public\tuser_entity\t4\t40\t\t\t0\n")
        self.assertEqual(stats["public.user_entity"]["idx_scan"], 0)

    def test_deltas_and_reads(self):
        before = parse_table_stats(self.TSV)
        after = parse_table_stats(
            "sequent_backend\tcast_vote\t12\t130\t6\t52\t3\n"
            "sequent_backend\tapplications\t1\t0\t0\t0\t0\n"
        )
        deltas = read_deltas(before, after)
        self.assertEqual(
            deltas["sequent_backend.cast_vote"],
            {
                "scans": 3,
                "tuples_read": 32,
                "seq_scan": 2,
                "idx_scan": 1,
                "n_tup_ins": 0,
            },
        )
        self.assertEqual(deltas["sequent_backend.applications"]["scans"], 0)

    def test_a_table_only_after_counts_from_zero(self):
        deltas = read_deltas({}, parse_table_stats(self.TSV))
        self.assertEqual(deltas["sequent_backend.cast_vote"]["scans"], 15)


class HarvestLogTest(unittest.TestCase):
    def test_counts_renderer_spans_not_validations_or_widget_routes(self):
        log = "\n".join(
            [
                "16┐render_widget body=Json(RenderWidgetInput { width: Some(400) })",
                "9└──┐render width=640",
                "15└──┐render width=360",
                "3└──┐validate ",
                "12   ├── INFO render width=640 closed",
            ]
        )
        self.assertEqual(count_renderer_calls(log), 2)


class DockerStatsTest(unittest.TestCase):
    def test_cpu_per_container(self):
        text = "harvest\t123.45%\t1.2GiB / 62GiB\nhasura\t7.00%\t300MiB / 62GiB\n"
        self.assertEqual(
            parse_docker_stats(text),
            {
                "harvest": {"cpu": 123.45, "memory": "1.2GiB"},
                "hasura": {"cpu": 7.0, "memory": "300MiB"},
            },
        )

    def test_blank_and_broken_lines_are_skipped(self):
        self.assertEqual(parse_docker_stats("\nharvest\t--\t\n"), {})


class TokenTest(unittest.TestCase):
    def test_expiry_of_a_jwt(self):
        claims = base64.urlsafe_b64encode(
            json.dumps({"exp": 1790000000}).encode()
        ).rstrip(b"=")
        self.assertEqual(token_expiry(f"h.{claims.decode()}.s"), 1790000000)

    def test_a_token_that_is_not_a_jwt_never_expires(self):
        self.assertIsNone(token_expiry("opaque"))


class MarkdownTest(unittest.TestCase):
    def test_one_row_per_level(self):
        levels = [
            {
                "viewers": 0,
                "operations": {},
                "errors": 0,
                "requests": 0,
                "renderer_calls": 0,
                "source_reads_per_minute": 12.0,
                "passes": 2,
                "cpu": {"harvest": {"mean": 1.0, "max": 2.0}},
            },
            {
                "viewers": 10,
                "operations": {
                    "render-widget": {
                        "n": 3,
                        "p50_ms": 10.0,
                        "p95_ms": 20.0,
                        "p99_ms": 30.0,
                    }
                },
                "errors": 1,
                "requests": 30,
                "renderer_calls": 5,
                "source_reads_per_minute": 12.0,
                "passes": 2,
                "cpu": {"harvest": {"mean": 50.5, "max": 80.0}},
            },
        ]
        table = markdown_table(levels, ["render-widget"], ["harvest"])
        lines = table.splitlines()
        self.assertEqual(len(lines), 4)
        self.assertIn("render-widget p50/p95/p99 ms", lines[0])
        self.assertIn("| 0 (idle) |", lines[2])
        self.assertIn("10 / 20 / 30", lines[3])
        self.assertIn("1 / 30", lines[3])
        self.assertIn("50.5 (80.0)", lines[3])


class CommandLineTest(unittest.TestCase):
    def test_defaults(self):
        arguments = parser().parse_args(
            [
                "monitoring-viewers",
                "--label",
                "live",
                "--tenant-id",
                "t",
                "--election-event-id",
                "e",
                "--token-command",
                "echo x",
            ]
        )
        self.assertEqual(arguments.viewers, [10, 100, 1000])
        self.assertEqual(arguments.cycle, Cycle.RELOAD.value)
        self.assertEqual(arguments.poll_interval, 30.0)
        self.assertEqual(arguments.dashboard, "overview")


if __name__ == "__main__":
    unittest.main()
