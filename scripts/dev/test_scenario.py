# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Tests for scripts.dev.scenario, with fake backends instead of a stack."""

import base64
import contextlib
import importlib
import io
import json
import os
import re
import socket
import tempfile
import unittest
import urllib.parse
from pathlib import Path
from unittest import mock

from scripts.dev.mode.checkout import REPOSITORY_ROOT, Checkout
from scripts.dev.mode.docker import ContainerState
from scripts.dev.mode.manifest import load_manifest
from scripts.dev.scenario import catalog, cli, runner
from scripts.dev.scenario.catalog import (
    CENSUS,
    SCENARIOS,
    STAGE_DESCRIPTIONS,
    CatalogError,
    Link,
    Scenario,
    Stage,
    find_scenario,
)
from scripts.dev.scenario.runner import (
    Outcome,
    Progress,
    ResetOutcome,
    ScenarioError,
    resume_index,
)
from scripts.dev.scenario.settings import (
    JOURNEY_OUTPUT,
    REQUIRED,
    STEP_CLI_BUILD,
    Portals,
    SettingsError,
    accepts,
    find_step_cli,
    journey_environment,
)
from scripts.dev.scenario.state import (
    OWNER_ANNOTATION,
    SCENARIO_ANNOTATION,
    Ownership,
    State,
    StateError,
    StateStore,
    new_state,
    ownership,
)
from scripts.e2e.journeys import fixtures

TENANT = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5"
PROJECT = "step_devcontainer"
KIOSK = find_scenario("kiosk-voter")
RESULTS = find_scenario("published-results")
# What .devcontainer/.env.development sets for the variables the journeys read.
DEVELOPMENT = {key: f"value-of-{key}" for key in REQUIRED} | {
    "SUPER_ADMIN_TENANT_ID": TENANT,
    "HASURA_ENDPOINT": "http://graphql-engine:8080/v1/graphql",
    "KEYCLOAK_URL": "http://keycloak:8090",
    "AWS_S3_PRIVATE_URI": "http://minio:9000",
    "B4_URL": "http://b4v6:3005",
    "VOTING_PORTAL_URL": "http://localhost:3000",
    "BALLOT_VERIFIER_URL": "http://localhost:3001",
    "RESULTS_PORTAL_URL": "http://localhost:3004",
    "KEYCLOAK_ADMIN_CLIENT_SECRET": "admin",
    "ADMIN_PORTAL_TEST_USERNAME": "admin",
    "ADMIN_PORTAL_TEST_PASSWORD": "admin",
}


JOURNEY_ENVIRONMENT = journey_environment(DEVELOPMENT, {}, Path(tempfile.gettempdir()))


def load_backend():
    """The stack backend, whose journey clients read the environment on import."""
    with mock.patch.dict(os.environ, JOURNEY_ENVIRONMENT):
        return importlib.import_module("scripts.dev.scenario.backend")


backend_module = load_backend()
journey_client = importlib.import_module("scripts.e2e.journeys.client")


def container(service, status="running", health=None, exit_code=0):
    return ContainerState(
        id=f"{service}-id",
        name=f"prefix-{service}",
        service=service,
        status=status,
        health=health,
        exit_code=exit_code,
        restart_policy="always",
        local_folder=None,
    )


class FakeBackend:
    """Events as annotations, and the stages that hold for each event."""

    tenant_id = TENANT
    project = PROJECT

    def __init__(self):
        self.events = {}
        self.held = {}
        self.calls = []
        self.fail_on = None
        self.imported = 0

    def add_event(self, event_id, annotations, stages=()):
        self.events[event_id] = dict(annotations)
        self.held[event_id] = set(stages)

    def event_annotations(self, event_id):
        return self.events.get(event_id)

    def annotated_events(self, annotations):
        return [
            event_id
            for event_id, existing in self.events.items()
            if all(existing.get(key) == value for key, value in annotations.items())
        ]

    def holds(self, stage, scenario, state):
        return stage in self.held.get(state.event_id, set())

    def run(self, stage, scenario, state):
        self.calls.append(("run", stage))
        if stage is Stage.EVENT:
            self.imported += 1
            state.event_id = f"event-{self.imported}"
            self.add_event(state.event_id, state.annotations())
        if stage is self.fail_on:
            raise ScenarioError(f"{stage.value} failed")
        self.held[state.event_id].add(stage)

    def point_portals(self, state):
        self.calls.append(("portals", state.event_id))

    def delete_event(self, event_id):
        self.calls.append(("delete", event_id))
        del self.events[event_id]


class StoreTestCase(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.store = StateStore.for_project(self.root, PROJECT)
        self.messages = []

    def say(self, message):
        self.messages.append(message)


class CatalogTest(unittest.TestCase):
    def test_names_are_unique_and_found(self):
        names = [scenario.name for scenario in SCENARIOS]
        self.assertEqual(len(names), len(set(names)))
        self.assertEqual(
            names,
            [
                "kiosk-voter",
                "completed-ceremony",
                "messaging",
                "messaging-second-organization",
                "published-results",
            ],
        )
        for name in names:
            self.assertRegex(name, r"^[a-z]+(-[a-z]+)*$")
            self.assertEqual(find_scenario(name).name, name)

    def test_messaging_scenarios_have_a_profile_and_the_single_channel_realm(self):
        from scripts.dev.scenario import messaging

        named = {s.name for s in SCENARIOS if Stage.MESSAGING in s.stages}
        self.assertEqual(named, set(messaging.PROFILES))
        realm = fixtures.election_event("http://127.0.0.1:3000", "m")[
            "keycloak_event_realm"
        ]
        self.assertFalse(messaging.has_single_channel_step(realm))
        dev = messaging.dev_event_realm(realm)
        self.assertTrue(messaging.has_single_channel_step(dev))
        self.assertNotIn("users", dev)
        self.assertEqual(dev["realm"], realm["realm"])

    def test_messaging_realm_keeps_all_fixture_portal_clients(self):
        from scripts.dev.scenario import messaging
        from scripts.dev.scenario.settings import PORTAL_CLIENTS

        fixture = fixtures.election_event("http://127.0.0.1:3000", "messaging")[
            "keycloak_event_realm"
        ]
        realm = messaging.dev_event_realm(fixture)
        imported = {client["clientId"]: client for client in realm["clients"]}
        originals = {client["clientId"]: client for client in fixture["clients"]}
        for client_id in PORTAL_CLIENTS:
            with self.subTest(client=client_id):
                self.assertIn(client_id, imported)
                self.assertEqual(
                    imported[client_id]["redirectUris"],
                    originals[client_id]["redirectUris"],
                )
                self.assertIsNot(imported[client_id], originals[client_id])
        self.assertFalse(
            imported["voting-portal-kiosk"].get("authenticationFlowBindingOverrides")
        )

    def test_every_scenario_tenant_is_a_known_organization(self):
        from scripts.dev.scenario import tenants

        for scenario in SCENARIOS:
            if scenario.tenant is not None:
                self.assertIn(scenario.tenant, tenants.ORGANIZATIONS)
        self.assertEqual(
            find_scenario("messaging-second-organization").tenant,
            tenants.STAFF_ASSOCIATION.slug,
        )
        self.assertIsNone(find_scenario("messaging").tenant)


class OrganizationTest(unittest.TestCase):
    """The second organization: its own tenant, elections and accounts."""

    def setUp(self):
        from scripts.dev.scenario import messaging, tenants

        self.messaging = messaging
        self.tenants = tenants
        self.organization = tenants.STAFF_ASSOCIATION
        self.document = fixtures.election_event("http://127.0.0.1:3000", "org")

    def names(self, items):
        return [fixtures.display_name(item) for item in items]

    def test_the_event_has_the_organization_elections_languages_and_timezones(self):
        event = self.tenants.organization_event(self.document, self.organization)
        self.assertEqual(
            self.names(event["elections"]),
            [
                "Council: Central office",
                "Council: Canary Islands office",
                "Pension board",
            ],
        )
        self.assertEqual(
            set(self.names(event["elections"])), set(self.organization.elections)
        )
        # The fixture's contests stay, so the voters and the other stages work.
        self.assertEqual(
            self.names(event["contests"]),
            [area.contest for area in fixtures.AREAS] + ["Pension board"],
        )
        board = event["elections"][2]
        (contest,) = [c for c in event["contests"] if c["election_id"] == board["id"]]
        self.assertIsNone(board["external_id"])
        self.assertEqual(
            [link["area_id"] for link in event["area_contests"]
             if link["contest_id"] == contest["id"]],
            [event["areas"][0]["id"]],
        )  # fmt: skip
        self.assertTrue(
            [c for c in event["candidates"] if c["contest_id"] == contest["id"]]
        )
        ids = [
            item["id"]
            for key in ("elections", "contests", "candidates", "area_contests")
            for item in event[key]
        ]
        self.assertEqual(len(ids), len(set(ids)))
        presentation = event["election_event"]["presentation"]
        self.assertEqual(
            presentation["language_conf"]["enabled_language_codes"], ["en", "es"]
        )
        self.assertEqual(presentation["language_conf"]["default_language_code"], "en")
        self.assertEqual(
            presentation["timezones"],
            {"configured": ["Europe/Madrid", "Atlantic/Canary"],
             "primary": "Europe/Madrid"},
        )  # fmt: skip
        for election in event["elections"]:
            self.assertEqual(
                election["presentation"]["language_conf"]["enabled_language_codes"],
                ["en", "es"],
            )
        # The fixture document itself is unchanged.
        self.assertEqual(len(self.document["elections"]), 2)

    def test_the_tenant_gets_the_organization_name_and_languages(self):
        self.assertEqual(
            self.tenants.tenant_settings(self.organization),
            {
                "display_name": "Staff Association",
                "language_conf": {
                    "enabled_language_codes": ["en", "es"],
                    "default_language_code": "en",
                },
            },
        )

    def test_restrictions_name_the_organization_elections(self):
        profile = self.messaging.SECOND_ORGANIZATION
        self.assertLessEqual(
            set(profile.election_channels), set(self.organization.elections)
        )
        elections = {name: f"id-{name}" for name in self.organization.elections}
        ids = {account.key: f"id-{account.key}" for account in profile.accounts}
        config = self.messaging.event_config(profile, ids, elections)
        self.assertEqual(
            config["election_channels"],
            {
                "id-Council: Canary Islands office": ["WHATSAPP", "SMS", "EMAIL"],
                "id-Pension board": ["EMAIL"],
            },
        )
        self.assertEqual(
            {t["language"] for c in config["channels"] for t in c["templates"]},
            {"en", "es"},
        )
        with self.assertRaisesRegex(self.messaging.MessagingError, "Pension board"):
            self.messaging.event_config(profile, ids, {"Council: Central office": "x"})

    def test_every_profile_config_has_one_entry_per_channel(self):
        for profile in self.messaging.PROFILES.values():
            ids = {account.key: f"id-{account.key}" for account in profile.accounts}
            elections = {name: name for name in profile.election_channels}
            config = self.messaging.event_config(profile, ids, elections)
            channels = [channel["channel"] for channel in config["channels"]]
            self.assertEqual(len(channels), len(set(channels)))
            for channel in config["channels"]:
                self.assertTrue(channel["purposes"])
                self.assertTrue(channel["account_id"].startswith("id-"))

    def test_the_second_organization_sends_from_its_own_senders(self):
        """The fake provider reports a message only when exactly one account
        has its sender and credential, and a tenant lists only the messages
        its accounts alone sent: the two organizations share the synthetic
        credentials, so their senders must differ."""

        def senders(profile):
            keys = ("phone_number_id", "business_account_id", "sender", "page_id",
                    "page_username", "sender_id", "from_address")  # fmt: skip
            return {
                (account.channel, key, account.sender[key])
                for account in profile.accounts
                for key in keys
                if key in account.sender
            }

        first = self.messaging.FIRST_ORGANIZATION
        second = self.messaging.SECOND_ORGANIZATION
        self.assertFalse(senders(first) & senders(second))
        self.assertEqual(
            sorted(account.channel for account in second.accounts),
            ["EMAIL", "MESSENGER", "SMS", "VIBER", "WHATSAPP"],
        )
        # Its own tenant: each account is that tenant's default.
        self.assertTrue(all(account.is_default for account in second.accounts))
        self.assertFalse(
            {a.name for a in first.accounts} & {a.name for a in second.accounts}
        )
        # Other purposes than the first organization's.
        purposes = lambda profile: {c.account: c.purposes for c in profile.channels}  # noqa: E731
        self.assertNotEqual(purposes(first), purposes(second))

    def test_unknown_name_lists_the_known_ones(self):
        with self.assertRaisesRegex(CatalogError, "kiosk-voter, completed-ceremony"):
            find_scenario("kiosk")

    def test_stages_start_with_the_event_and_keep_the_canonical_order(self):
        order = list(Stage)
        for scenario in SCENARIOS:
            self.assertEqual(scenario.stages[0], Stage.EVENT, scenario.name)
            positions = [order.index(stage) for stage in scenario.stages]
            self.assertEqual(positions, sorted(positions), scenario.name)
            self.assertEqual(len(positions), len(set(positions)), scenario.name)
            self.assertTrue(scenario.links, scenario.name)

    def test_every_stage_is_described(self):
        self.assertEqual(set(STAGE_DESCRIPTIONS), set(Stage))

    def test_kiosk_voter_opens_only_the_kiosk_channel(self):
        self.assertEqual(KIOSK.stages[-1], Stage.KIOSK_OPEN)
        self.assertNotIn(Stage.ONLINE_OPEN, KIOSK.stages)
        self.assertEqual(KIOSK.links[0], Link.KIOSK)

    def test_published_results_casts_before_closing_and_tallying(self):
        stages = RESULTS.stages
        self.assertLess(stages.index(Stage.ONLINE_OPEN), stages.index(Stage.VOTES))
        self.assertLess(stages.index(Stage.VOTES), stages.index(Stage.ONLINE_CLOSED))
        self.assertLess(stages.index(Stage.ONLINE_CLOSED), stages.index(Stage.TALLY))
        self.assertEqual(stages[-1], Stage.RESULTS)
        self.assertEqual(RESULTS.links[0], Link.RESULTS)

    def test_trustees_are_needed_by_ceremonies_and_tallies_only(self):
        self.assertTrue(all(scenario.needs_trustees for scenario in SCENARIOS))
        without = Scenario("plain", "plain", (Stage.EVENT, Stage.VOTERS), (Link.ADMIN,))
        self.assertFalse(without.needs_trustees)

    def test_census_covers_the_fixture_areas(self):
        self.assertEqual(set(CENSUS), set(fixtures.AREA))

    def test_votes_name_census_voters_and_their_area_candidates(self):
        for vote in RESULTS.votes:
            self.assertLessEqual(vote.voter, CENSUS[vote.area])
            self.assertIn(vote.candidate, fixtures.AREA[vote.area].candidates)
        usernames = [catalog.voter_username(RESULTS, vote) for vote in RESULTS.votes]
        self.assertEqual(len(usernames), len(set(usernames)))
        self.assertEqual(usernames[0], "e2e-published-results-a1")
        self.assertEqual(usernames[-1], "e2e-published-results-c1")

    def test_each_scenario_has_its_own_usernames(self):
        usernames = catalog.census_usernames(KIOSK)
        self.assertEqual(
            usernames["A"],
            [f"e2e-kiosk-voter-a{index}" for index in range(1, CENSUS["A"] + 1)],
        )
        self.assertEqual(sum(map(len, usernames.values())), sum(CENSUS.values()))


class FakeGraphql:
    """Hasura actions: answers each document with a function of its
    variables, and records the calls."""

    def __init__(self, answers):
        self.answers = answers
        self.calls = []

    def run(self, role, query, variables):
        self.calls.append((role, query, variables))
        return self.answers[query](variables)

    def documents(self, query):
        return [variables for _, document, variables in self.calls if document == query]


class MessagingSetupTest(unittest.TestCase):
    """The scenario's accounts: repaired when a run stopped half-way, and
    never taking over a tenant's own defaults."""

    def setUp(self):
        from scripts.dev.scenario import messaging

        self.messaging = messaging
        self.profile = messaging.SECOND_ORGANIZATION
        self.said = []
        self.checks = {}

    def admin(self, stored, defaults=()):
        """`stored`: account key -> its credential names (None: not stored)."""
        m = self.messaging
        rows = [
            {
                "id": f"id-{account.key}",
                "name": account.name,
                "channel": account.channel,
                "is_default": account.is_default,
                "credentials": {name: "ref" for name in stored[account.key]},
            }
            for account in self.profile.accounts
            if stored.get(account.key) is not None
        ]
        return FakeHasura(
            {
                m.ACCOUNTS: lambda _: {"sequent_backend_messaging_account": rows},
                m.NOTICE_TEMPLATES: lambda _: {
                    "sequent_backend_template": [{"id": "notice"}]
                },
                m.DEFAULT_ACCOUNTS: lambda _: {
                    "sequent_backend_messaging_account": [
                        {"name": name, "channel": channel} for name, channel in defaults
                    ]
                },
                m.ELECTIONS: lambda _: {
                    "sequent_backend_election": [
                        {
                            "id": f"election-{n}",
                            "presentation": {"i18n": {"en": {"name": name}}},
                        }
                        for n, name in enumerate(self.profile.election_channels)
                    ]
                },
            }
        )

    def graphql(self):
        m = self.messaging
        ready = {"connected": True, "production_access": True, "reason": None}
        return FakeGraphql(
            {
                m.UPSERT: lambda v: {
                    "upsert_messaging_account": {"id": f"new-{v['name']}"}
                },
                m.CREDENTIALS: lambda v: {
                    "replace_messaging_account_credentials": {"replaced": True}
                },
                m.CHECK: lambda v: {
                    "check_messaging_account": {
                        "status": self.checks.get(v["id"], ready),
                        "callback_reason": None,
                    }
                },
                m.SAVE_CONFIG: lambda v: {
                    "update_event_messaging_config": {"errors": []}
                },
            }
        )

    def complete(self):
        return {
            account.key: self.messaging.expected_credentials(account)
            for account in self.profile.accounts
        }

    def configure(self, admin, graphql):
        return self.messaging.configure(
            graphql, admin, "tenant", "event", self.profile, self.said.append
        )

    def test_existing_fake_account_is_renamed_without_creating_another_account(self):
        m = self.messaging
        profile = m.FIRST_ORGANIZATION
        email = profile.account("email")
        self.assertEqual(email.name, "Demo Email")
        renamed = []

        def rename(values):
            renamed.append(values)
            return {"update_sequent_backend_messaging_account": {"affected_rows": 1}}

        admin = FakeHasura(
            {
                m.RENAME_ACCOUNT: rename,
                m.NOTICE_TEMPLATES: lambda _: {
                    "sequent_backend_template": [{"id": "notice"}]
                },
                m.ACCOUNTS: lambda _: {
                    "sequent_backend_messaging_account": [
                        {
                            "id": "existing",
                            "name": "Fake email",
                            "sender": {
                                "provider": "AWS_SES",
                                "from_name": "Elections",
                                "from_address": "elections@fake-provider.example.org",
                            },
                            "credentials": {
                                key: "ref" for key in m.expected_credentials(email)
                            },
                        }
                    ]
                },
                m.DEFAULT_ACCOUNTS: lambda _: {"sequent_backend_messaging_account": []},
                m.ELECTIONS: lambda _: {"sequent_backend_election": []},
            }
        )
        graphql = self.graphql()
        ids = m.configure(graphql, admin, "tenant", "event", profile, self.said.append)
        self.assertEqual(ids["email"], "existing")
        self.assertEqual(len(renamed), 1)
        self.assertEqual(renamed[0]["name"], "Demo Email")
        self.assertEqual(renamed[0]["tenant"], "tenant")
        self.assertEqual(renamed[0]["id"], "existing")
        self.assertFalse(
            any(v["name"] == "Demo Email" for v in graphql.documents(m.UPSERT))
        )
        self.assertEqual(
            renamed[0]["sender"]["from_address"], "elections@demo-provider.example.org"
        )
        self.assertFalse(
            any(v["id"] == "existing" for v in graphql.documents(m.CREDENTIALS))
        )

    def test_an_account_left_without_credentials_gets_them_and_is_checked(self):
        stored = self.complete()
        viber = next(a for a in self.profile.accounts if a.channel == "VIBER")
        stored[viber.key] = set()
        graphql = self.graphql()
        self.configure(self.admin(stored), graphql)
        replaced = graphql.documents(self.messaging.CREDENTIALS)
        self.assertEqual(
            replaced,
            [
                {
                    "id": f"id-{viber.key}",
                    "credentials": viber.credentials,
                    "verify": False,
                    "report": True,
                }
            ],
        )
        self.assertFalse(graphql.documents(self.messaging.UPSERT))
        self.assertIn(
            f"id-{viber.key}",
            [v["id"] for v in graphql.documents(self.messaging.CHECK)],
        )
        self.assertTrue(any("REPORT_PASSWORD" in line for line in self.said))

    def test_generated_credentials_count(self):
        whatsapp = next(a for a in self.profile.accounts if a.channel == "WHATSAPP")
        viber = next(a for a in self.profile.accounts if a.channel == "VIBER")
        self.assertIn("VERIFY_TOKEN", self.messaging.expected_credentials(whatsapp))
        self.assertLessEqual(
            {"REPORT_USERNAME", "REPORT_PASSWORD", "API_KEY"},
            self.messaging.expected_credentials(viber),
        )

    def test_a_failed_check_names_its_own_reason(self):
        sms = next(a for a in self.profile.accounts if a.channel == "SMS")
        self.checks[f"id-{sms.key}"] = {
            "connected": False,
            "reason": "The account is in the SMS sandbox",
        }
        with self.assertRaises(self.messaging.MessagingError) as raised:
            self.configure(self.admin(self.complete()), self.graphql())
        message = str(raised.exception)
        self.assertIn("The account is in the SMS sandbox", message)
        self.assertNotIn("MESSAGING_FAKE_PROVIDER_ROUTES", message)

    def test_a_connection_failure_asks_about_the_fake_provider_routes(self):
        sms = next(a for a in self.profile.accounts if a.channel == "SMS")
        self.checks[f"id-{sms.key}"] = {
            "connected": False,
            "reason": "The provider couldn't be reached",
        }
        with self.assertRaisesRegex(
            self.messaging.MessagingError,
            "The provider couldn't be reached.*MESSAGING_FAKE_PROVIDER_ROUTES",
        ):
            self.configure(self.admin(self.complete()), self.graphql())

    def test_a_tenant_default_is_kept_and_named(self):
        graphql = self.graphql()
        self.configure(self.admin({}, defaults=[("Their own SMS", "SMS")]), graphql)
        upserts = {
            v["name"]: v["is_default"] for v in graphql.documents(self.messaging.UPSERT)
        }
        sms = next(a for a in self.profile.accounts if a.channel == "SMS")
        email = next(a for a in self.profile.accounts if a.channel == "EMAIL")
        self.assertIs(upserts[sms.name], False)
        self.assertIs(upserts[email.name], True)
        self.assertTrue(any("Their own SMS" in line for line in self.said))

    def test_the_scenarios_say_their_accounts_stay_after_reset(self):
        for name in ("messaging", "messaging-second-organization"):
            self.assertIn("stay after reset", find_scenario(name).summary)


class MessagingNoticeTemplateTest(unittest.TestCase):
    def test_missing_demo_template_has_content_and_approved_names_for_every_channel(
        self,
    ):
        from scripts.dev.scenario import messaging as m

        admin = mock.Mock()
        admin.query.side_effect = [
            {"sequent_backend_template": []},
            {"insert_sequent_backend_template_one": {"id": "notice"}},
        ]
        m.ensure_notice_template(admin, "tenant")
        row = admin.query.call_args.args[1]["object"]
        self.assertEqual(row["tenant_id"], "tenant")
        self.assertEqual(row["alias"], "Demo notification")
        content = row["template"]
        self.assertEqual(
            set(content["selected_methods"]),
            {"EMAIL", "SMS", "WHATSAPP", "VIBER", "MESSENGER"},
        )
        self.assertIn("Voting reminder", content["email"]["plaintext_body"])
        self.assertIn("{{election_event.name}}", content["email"]["subject"])
        self.assertIn("Do not share", content["sms"]["message"])
        for key in ("whatsapp", "viber", "messenger"):
            self.assertIn("{{election_event.name}}", content[key]["message"])
            self.assertIn("{{vote_url}}", content[key]["message"])
            self.assertEqual(
                content[key]["parameters"],
                ["{{user.first_name}}", "{{election_event.name}}", "{{vote_url}}"],
            )
            self.assertEqual(content[key]["provider_template"], "step_notice")
            self.assertEqual(content[key]["provider_language"], "en")

    def test_existing_demo_template_is_kept_without_writing(self):
        from scripts.dev.scenario import messaging as m

        admin = mock.Mock()
        admin.query.return_value = {"sequent_backend_template": [{"id": "existing"}]}
        m.ensure_notice_template(admin, "tenant")
        self.assertEqual(admin.query.call_count, 1)

    def test_untouched_demo_seed_is_upgraded_with_a_conditional_update(self):
        import uuid

        from scripts.dev.scenario import messaging as m

        message = (
            "Hello {{user.first_name}}, voting is open. Visit {{vote_url}} to vote."
        )
        instant = {
            "message": message,
            "parameters": ["{{user.first_name}}", "{{vote_url}}"],
            "provider_template": "step_notice",
            "provider_language": "en",
        }
        previous = {
            "alias": "Demo notification",
            "selected_methods": dict.fromkeys(
                ["EMAIL", "SMS", "WHATSAPP", "VIBER", "MESSENGER"], True
            ),
            "email": {
                "subject": "Voting is open",
                "plaintext_body": message,
                "html_body": "<p>" + message + "</p>",
            },
            "sms": {"message": message},
            "whatsapp": instant,
            "viber": instant,
            "messenger": instant,
        }
        row = {
            "id": str(uuid.uuid5(uuid.NAMESPACE_URL, "step-demo-notice:tenant")),
            "created_by": "demo-setup",
            "template": previous,
        }
        admin = mock.Mock()
        admin.query.return_value = {"sequent_backend_template": [row]}
        m.ensure_notice_template(admin, "tenant")
        self.assertEqual(admin.query.call_count, 2)
        query, variables = admin.query.call_args.args
        self.assertIn("template: {_eq: $previous}", query)
        self.assertEqual(variables["previous"], previous)
        self.assertIn("Voting reminder", variables["template"]["whatsapp"]["message"])
        for changed in (
            {"id": "custom"},
            {"created_by": "administrator"},
            {"template": {**previous, "sms": {"message": "Custom reminder"}}},
            {"template": variables["template"]},
        ):
            with self.subTest(changed=changed):
                admin.reset_mock()
                admin.query.return_value = {
                    "sequent_backend_template": [{**row, **changed}]
                }
                m.ensure_notice_template(admin, "tenant")
                self.assertEqual(admin.query.call_count, 1)


class MessagingServiceAccountTest(unittest.TestCase):
    def test_missing_tenant_attribute_is_repaired_without_losing_other_attributes(self):
        from scripts.dev.scenario import tenants

        keycloak = mock.Mock()
        keycloak.admin.side_effect = [
            [{"id": "client"}],
            {"id": "user", "attributes": {"other": ["keep"]}},
            None,
        ]
        self.assertTrue(tenants.configure_messaging_service_account(keycloak, "tenant"))
        keycloak.admin.assert_called_with(
            "PUT",
            "tenant-tenant/users/user",
            json_body={"attributes": {"other": ["keep"], "tenant-id": ["tenant"]}},
            expect=(204,),
        )

    def test_correct_service_account_needs_no_write(self):
        from scripts.dev.scenario import tenants

        keycloak = mock.Mock()
        keycloak.admin.side_effect = [
            [{"id": "client"}],
            {"id": "user", "attributes": {"tenant-id": ["tenant"]}},
        ]
        self.assertFalse(
            tenants.configure_messaging_service_account(keycloak, "tenant")
        )
        self.assertEqual(keycloak.admin.call_count, 2)

    def test_missing_service_attribute_prevents_reporting_setup_complete(self):
        from scripts.dev.scenario import tenants

        keycloak = mock.Mock()
        keycloak.admin.side_effect = [[{"id": "client"}], {"id": "user"}]
        self.assertFalse(
            tenants.messaging_service_account_configured(keycloak, "tenant")
        )
        self.assertEqual(keycloak.admin.call_count, 2)


class TenantSettingsTest(unittest.TestCase):
    """The organization's settings: written only where the tenant has none."""

    def setUp(self):
        from scripts.dev.scenario import tenants

        self.tenants = tenants
        self.organization = tenants.STAFF_ASSOCIATION

    def reconcile(self, settings):
        graphql = FakeGraphql({self.tenants.APPEND_SETTINGS: lambda v: {}})
        admin = FakeHasura(
            {
                self.tenants.TENANT_SETTINGS: lambda _: {
                    "sequent_backend_tenant_by_pk": {"settings": settings}
                }
            }
        )
        kept = self.tenants.reconcile_settings(graphql, admin, "t", self.organization)
        written = [
            v["settings"] for v in graphql.documents(self.tenants.APPEND_SETTINGS)
        ]
        return written, kept

    def test_a_new_tenant_gets_the_organization_settings(self):
        written, kept = self.reconcile(None)
        self.assertEqual(written, [self.tenants.tenant_settings(self.organization)])
        self.assertEqual(kept, [])

    def test_the_tenant_own_settings_are_kept_and_named(self):
        written, kept = self.reconcile({"display_name": "Renamed by hand", "other": 1})
        self.assertEqual(
            written,
            [
                {
                    "language_conf": self.tenants.tenant_settings(self.organization)[
                        "language_conf"
                    ]
                }
            ],
        )
        self.assertEqual(kept, ["display_name"])

    def test_nothing_is_written_when_the_settings_match(self):
        written, kept = self.reconcile(self.tenants.tenant_settings(self.organization))
        self.assertEqual((written, kept), ([], []))


class StateTest(StoreTestCase):
    def test_round_trip(self):
        state = new_state(KIOSK.name, PROJECT, TENANT)
        state.event_id = "event"
        state.ids = {"areas": {"A": "area"}}
        state.voters = {"A": ["voter"]}
        state.stages = {"event": 1.5}
        copy = State.from_document(json.loads(json.dumps(state.to_document())))
        self.assertEqual(copy, state)

    def test_saved_under_the_project(self):
        state = new_state(KIOSK.name, PROJECT, TENANT)
        self.store.save(state)
        path = self.root / ".cache/scenarios" / PROJECT / "kiosk-voter.json"
        self.assertTrue(path.is_file())
        self.assertEqual(self.store.load(KIOSK.name).owner, state.owner)
        other = StateStore.for_project(self.root, "other_devcontainer")
        self.assertIsNone(other.load(KIOSK.name))

    def test_absent_state_loads_as_none(self):
        self.assertIsNone(self.store.load(KIOSK.name))

    def test_rejects_another_version_malformed_or_foreign_files(self):
        self.store.directory.mkdir(parents=True)
        document = new_state(KIOSK.name, PROJECT, TENANT).to_document()
        path = self.store.path(KIOSK.name)
        for content, message in (
            ({**document, "version": 2}, "version 1"),
            ({**document, "scenario": "published-results"}, "published-results"),
            (
                {key: value for key, value in document.items() if key != "owner"},
                "owner",
            ),
        ):
            path.write_text(json.dumps(content))
            with self.assertRaisesRegex(StateError, message):
                self.store.load(KIOSK.name)
        path.write_text("{")
        with self.assertRaises(StateError):
            self.store.load(KIOSK.name)

    def test_owner_tokens_are_unique_and_survive_uuid_replacement(self):
        tokens = {new_state(KIOSK.name, PROJECT, TENANT).owner for _ in range(20)}
        self.assertEqual(len(tokens), 20)
        uuid = re.compile(
            r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}"
        )
        self.assertFalse(any(uuid.search(token) for token in tokens))

    def test_ownership(self):
        state = new_state(KIOSK.name, PROJECT, TENANT)
        owned = {SCENARIO_ANNOTATION: KIOSK.name, OWNER_ANNOTATION: state.owner}
        self.assertIs(ownership(state, None), Ownership.MISSING)
        self.assertIs(ownership(state, owned), Ownership.OWNED)
        self.assertIs(ownership(state, {**owned, "ivr:config": "{}"}), Ownership.OWNED)
        self.assertIs(ownership(state, {}), Ownership.FOREIGN)
        self.assertIs(
            ownership(state, {**owned, OWNER_ANNOTATION: "other"}), Ownership.FOREIGN
        )
        self.assertIs(
            ownership(state, {**owned, SCENARIO_ANNOTATION: RESULTS.name}),
            Ownership.FOREIGN,
        )

    def test_lock_excludes_a_second_command_on_the_same_scenario(self):
        with self.store.lock(KIOSK.name):
            with self.assertRaisesRegex(StateError, "another step-dev scenario"):
                with self.store.lock(KIOSK.name):
                    pass
            with self.store.lock(RESULTS.name):
                pass
        with self.store.lock(KIOSK.name):
            pass


class ResumeTest(unittest.TestCase):
    def test_nothing_holds(self):
        self.assertEqual(resume_index(RESULTS.stages, lambda stage: False), 0)

    def test_the_last_stage_holding_means_ready(self):
        stages = RESULTS.stages
        self.assertEqual(
            resume_index(stages, lambda stage: stage is Stage.RESULTS), len(stages)
        )

    def test_a_later_stage_covers_one_it_undid(self):
        # Closing voting undoes "online voting open"; the tally continues.
        held = {Stage.EVENT, Stage.VOTES, Stage.ONLINE_CLOSED}
        index = resume_index(RESULTS.stages, lambda stage: stage in held)
        self.assertEqual(RESULTS.stages[index], Stage.TALLY)

    def test_an_earlier_stage_reverted_resumes_there(self):
        # Voting closed by hand on an event that should have it open.
        stages = find_scenario("completed-ceremony").stages
        held = set(stages) - {Stage.ONLINE_OPEN}
        self.assertEqual(
            stages[resume_index(stages, lambda stage: stage in held)], Stage.ONLINE_OPEN
        )


class UpTest(StoreTestCase):
    def setUp(self):
        super().setUp()
        self.backend = FakeBackend()

    def up(self, scenario=KIOSK):
        return runner.up(scenario, self.backend, self.store, self.say)

    def runs(self):
        return [stage for kind, stage in self.backend.calls if kind == "run"]

    def test_first_up_runs_every_stage_in_order(self):
        report = self.up()
        self.assertIs(report.outcome, Outcome.CREATED)
        self.assertEqual(self.runs(), list(KIOSK.stages))
        saved = self.store.load(KIOSK.name)
        self.assertEqual(saved.event_id, "event-1")
        self.assertEqual(set(saved.stages), {stage.value for stage in KIOSK.stages})
        self.assertEqual(self.backend.events["event-1"], saved.annotations())

    def test_second_up_reuses_and_reverifies(self):
        self.up()
        self.backend.calls.clear()
        report = self.up()
        self.assertIs(report.outcome, Outcome.REUSED)
        self.assertEqual(self.runs(), [])
        self.assertIn(("portals", "event-1"), self.backend.calls)
        self.assertEqual(report.state.event_id, "event-1")
        self.assertEqual(len(self.backend.events), 1)

    def test_partial_progress_resumes(self):
        self.up()
        self.backend.held["event-1"] = {Stage.EVENT, Stage.VOTERS, Stage.KEYS}
        self.backend.calls.clear()
        report = self.up()
        self.assertIs(report.outcome, Outcome.RESUMED)
        self.assertEqual(self.runs(), [Stage.BALLOTS, Stage.KIOSK_OPEN])

    def test_a_failed_stage_keeps_the_event_for_the_next_run(self):
        self.backend.fail_on = Stage.KEYS
        with self.assertRaisesRegex(ScenarioError, "keys failed"):
            self.up()
        saved = self.store.load(KIOSK.name)
        self.assertEqual(saved.event_id, "event-1")
        self.assertEqual(set(saved.stages), {"event", "voters"})
        self.backend.fail_on = None
        self.backend.calls.clear()
        self.up()
        self.assertEqual(self.runs(), [Stage.KEYS, Stage.BALLOTS, Stage.KIOSK_OPEN])

    def test_an_event_deleted_elsewhere_is_replaced(self):
        first = self.up().state
        del self.backend.events["event-1"]
        report = self.up()
        self.assertIs(report.outcome, Outcome.CREATED)
        self.assertEqual(report.state.event_id, "event-2")
        self.assertNotEqual(report.state.owner, first.owner)
        self.assertTrue(any("no longer exists" in message for message in self.messages))

    def test_an_event_without_the_owner_annotations_is_left_alone(self):
        self.up()
        self.backend.events["event-1"] = {OWNER_ANNOTATION: "someone else"}
        before = self.store.path(KIOSK.name).read_text()
        self.backend.calls.clear()
        with self.assertRaisesRegex(ScenarioError, "left alone"):
            self.up()
        self.assertEqual(self.backend.calls, [])
        self.assertEqual(self.store.path(KIOSK.name).read_text(), before)

    def test_an_import_that_was_not_recorded_is_adopted(self):
        state = new_state(KIOSK.name, PROJECT, TENANT)
        self.store.save(state)
        self.backend.add_event("orphan", state.annotations(), {Stage.EVENT})
        self.up()
        self.assertNotIn(Stage.EVENT, self.runs())
        self.assertEqual(self.store.load(KIOSK.name).event_id, "orphan")

    def test_two_events_with_one_owner_token_stop_the_command(self):
        state = new_state(KIOSK.name, PROJECT, TENANT)
        self.store.save(state)
        self.backend.add_event("one", state.annotations())
        self.backend.add_event("two", state.annotations())
        with self.assertRaisesRegex(ScenarioError, "2 events"):
            self.up()

    def test_state_of_another_tenant_is_refused(self):
        self.store.save(new_state(KIOSK.name, PROJECT, "another-tenant"))
        with self.assertRaisesRegex(ScenarioError, "another-tenant"):
            self.up()

    def test_a_concurrent_command_is_refused(self):
        with self.store.lock(KIOSK.name):
            with self.assertRaises(StateError):
                self.up()
        self.assertEqual(self.backend.calls, [])

    def test_scenarios_get_separate_events(self):
        self.up(KIOSK)
        self.up(RESULTS)
        self.assertEqual(self.store.load(KIOSK.name).event_id, "event-1")
        self.assertEqual(self.store.load(RESULTS.name).event_id, "event-2")


class ResetTest(StoreTestCase):
    def setUp(self):
        super().setUp()
        self.backend = FakeBackend()
        self.backend.add_event("unrelated", {"ivr:config": "{}"})

    def reset(self):
        return runner.reset(KIOSK, self.backend, self.store, self.say)

    def test_deletes_only_its_own_event(self):
        runner.up(KIOSK, self.backend, self.store, self.say)
        runner.up(RESULTS, self.backend, self.store, self.say)
        self.assertIs(self.reset(), ResetOutcome.DELETED)
        self.assertEqual(
            [call for call in self.backend.calls if call[0] == "delete"],
            [("delete", "event-1")],
        )
        self.assertEqual(set(self.backend.events), {"unrelated", "event-2"})
        self.assertIsNone(self.store.load(KIOSK.name))
        self.assertIsNotNone(self.store.load(RESULTS.name))

    def test_nothing_to_reset(self):
        self.assertIs(self.reset(), ResetOutcome.NOTHING)
        self.assertEqual(self.backend.calls, [])

    def test_an_event_already_gone_only_drops_the_state(self):
        runner.up(KIOSK, self.backend, self.store, self.say)
        del self.backend.events["event-1"]
        self.assertIs(self.reset(), ResetOutcome.ALREADY_GONE)
        self.assertNotIn("delete", [call[0] for call in self.backend.calls])
        self.assertIsNone(self.store.load(KIOSK.name))

    def test_a_foreign_event_is_not_deleted(self):
        state = new_state(KIOSK.name, PROJECT, TENANT)
        state.event_id = "unrelated"
        self.store.save(state)
        with self.assertRaisesRegex(ScenarioError, "nothing was deleted"):
            self.reset()
        self.assertIn("unrelated", self.backend.events)
        self.assertIsNotNone(self.store.load(KIOSK.name))

    def test_an_unrecorded_import_is_found_by_its_annotations(self):
        state = new_state(KIOSK.name, PROJECT, TENANT)
        self.store.save(state)
        self.backend.add_event("orphan", state.annotations())
        self.assertIs(self.reset(), ResetOutcome.DELETED)
        self.assertEqual(set(self.backend.events), {"unrelated"})

    def test_up_after_reset_creates_a_new_event(self):
        first = runner.up(KIOSK, self.backend, self.store, self.say).state
        self.reset()
        second = runner.up(KIOSK, self.backend, self.store, self.say).state
        self.assertNotEqual(first.event_id, second.event_id)
        self.assertNotEqual(first.owner, second.owner)

    def test_a_failed_delete_keeps_the_state_for_retry(self):
        state = runner.up(KIOSK, self.backend, self.store, self.say).state
        with mock.patch.object(
            self.backend, "delete_event", side_effect=ScenarioError("delete timed out")
        ):
            with self.assertRaisesRegex(ScenarioError, "delete timed out"):
                self.reset()
        self.assertEqual(self.store.load(KIOSK.name).event_id, state.event_id)
        self.assertIn(state.event_id, self.backend.events)
        self.assertIs(self.reset(), ResetOutcome.DELETED)
        self.assertEqual(set(self.backend.events), {"unrelated"})

    def test_every_recovered_event_is_checked_before_any_is_deleted(self):
        state = new_state(KIOSK.name, PROJECT, TENANT)
        self.store.save(state)
        self.backend.add_event("owned", state.annotations())
        with mock.patch.object(
            self.backend, "annotated_events", return_value=["owned", "unrelated"]
        ):
            with self.assertRaisesRegex(
                ScenarioError, "unrelated.*nothing was deleted"
            ):
                self.reset()
        self.assertEqual(set(self.backend.events), {"owned", "unrelated"})
        self.assertEqual(self.backend.calls, [])
        self.assertIsNotNone(self.store.load(KIOSK.name))

    def test_another_tenant_is_rejected_before_inspecting_or_deleting_events(self):
        state = new_state(KIOSK.name, PROJECT, "another-tenant")
        state.event_id = "unrelated"
        self.store.save(state)
        with mock.patch.object(self.backend, "event_annotations") as annotations:
            with self.assertRaisesRegex(
                ScenarioError, "another-tenant.*nothing was deleted"
            ):
                self.reset()
        annotations.assert_not_called()
        self.assertEqual(set(self.backend.events), {"unrelated"})
        self.assertIsNotNone(self.store.load(KIOSK.name))

    def test_a_concurrent_reset_leaves_the_event_and_state_intact(self):
        state = runner.up(KIOSK, self.backend, self.store, self.say).state
        with self.store.lock(KIOSK.name):
            with self.assertRaisesRegex(
                StateError, "another step-dev scenario command"
            ):
                self.reset()
        self.assertIn(state.event_id, self.backend.events)
        self.assertIsNotNone(self.store.load(KIOSK.name))


class StatusTest(StoreTestCase):
    def setUp(self):
        super().setUp()
        self.backend = FakeBackend()

    def status(self):
        return runner.status(KIOSK, self.backend, self.store)

    def test_progress(self):
        self.assertIs(self.status().progress, Progress.ABSENT)
        runner.up(KIOSK, self.backend, self.store, self.say)
        self.assertIs(self.status().progress, Progress.READY)
        self.backend.held["event-1"] = {Stage.EVENT, Stage.VOTERS}
        report = self.status()
        self.assertIs(report.progress, Progress.INCOMPLETE)
        self.assertIs(report.next_stage, Stage.KEYS)
        self.backend.events["event-1"] = {}
        self.assertIs(self.status().progress, Progress.FOREIGN)
        del self.backend.events["event-1"]
        self.assertIs(self.status().progress, Progress.GONE)

    def test_status_changes_nothing(self):
        runner.up(KIOSK, self.backend, self.store, self.say)
        self.backend.calls.clear()
        self.backend.held["event-1"] = {Stage.EVENT}
        self.status()
        self.assertEqual(self.backend.calls, [])


class SettingsTest(unittest.TestCase):
    def test_journey_environment_maps_the_devcontainer_names(self):
        environment = journey_environment(DEVELOPMENT, {}, Path("/logs"))
        self.assertEqual(environment["HASURA_ADMIN_SECRET"], "admin")
        self.assertEqual(environment["ADMIN_USERNAME"], "admin")
        self.assertEqual(environment[JOURNEY_OUTPUT], "/logs")
        self.assertNotIn("PATH", environment)

    def test_process_values_win(self):
        environment = journey_environment(
            DEVELOPMENT,
            {"VOTING_PORTAL_URL": "http://localhost:3100", "ADMIN_USERNAME": "other"},
            Path("/logs"),
        )
        self.assertEqual(environment["VOTING_PORTAL_URL"], "http://localhost:3100")
        self.assertEqual(environment["ADMIN_USERNAME"], "other")

    def test_missing_settings_are_named(self):
        incomplete = {
            key: value
            for key, value in DEVELOPMENT.items()
            if key not in ("B4_URL", "API_KEY_CLIENT_SECRET")
        }
        with self.assertRaisesRegex(
            SettingsError, "API_KEY_CLIENT_SECRET, B4_URL not set.*initialize-command"
        ):
            journey_environment(incomplete, {"B4_URL": ""}, Path("/logs"))

    def test_links(self):
        portals = Portals.from_environment(
            {
                "VOTING_PORTAL_URL": "http://localhost:3000/",
                "BALLOT_VERIFIER_URL": "http://localhost:3001/",
                "RESULTS_PORTAL_URL": "http://localhost:3004",
            }
        )
        event = f"/tenant/{TENANT}/event/event"
        self.assertEqual(
            {link: portals.link(link, TENANT, "event") for link in Link},
            {
                Link.VOTING: f"http://localhost:3000{event}/login",
                Link.KIOSK: f"http://localhost:3000{event}/login?kiosk",
                Link.VERIFIER: f"http://localhost:3001{event}/start",
                Link.RESULTS: "http://localhost:3004/event",
                Link.ADMIN: "http://127.0.0.1:3002/sequent_backend_election_event/event",
            },
        )

    def test_accepts_the_import_normalized_clients(self):
        # What the backend writes into an imported realm's voting clients.
        client = {
            "rootUrl": "http://localhost:3000",
            "redirectUris": ["/*", "http://localhost:3001/*"],
            "webOrigins": ["*"],
        }
        origins = ["http://localhost:3000", "http://localhost:3001"]
        self.assertTrue(accepts(client, origins))
        self.assertFalse(accepts(client, ["http://localhost:3100"]))
        self.assertTrue(
            accepts({**client, "webOrigins": ["+"]}, ["http://localhost:3000"])
        )
        self.assertFalse(
            accepts(
                {**client, "webOrigins": ["http://localhost:3001"]},
                ["http://localhost:3000"],
            )
        )
        self.assertFalse(accepts({}, ["http://localhost:3000"]))

    def test_plus_web_origins_inherit_only_http_redirect_origins(self):
        root = "http://localhost:3000"
        verifier = "http://localhost:3001"
        client = {
            "rootUrl": root,
            "redirectUris": ["*"],
            "webOrigins": ["+"],
        }
        self.assertFalse(accepts(client, [verifier]))
        self.assertTrue(accepts({**client, "webOrigins": ["*"]}, [verifier]))
        self.assertTrue(
            accepts(
                {**client, "redirectUris": ["/*", f"{verifier}/*"]},
                [root, verifier],
            )
        )
        with_callback = {**client, "redirectUris": ["*", f"{verifier}/callback"]}
        self.assertTrue(accepts(with_callback, [verifier]))
        self.assertFalse(accepts(with_callback, [root]))

    def test_find_step_cli(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaisesRegex(SettingsError, "cargo build --release"):
                find_step_cli(None, root, "")
            with self.assertRaisesRegex(SettingsError, "does not exist"):
                find_step_cli(str(root / "missing"), root, "")
            built = root / STEP_CLI_BUILD
            built.parent.mkdir(parents=True)
            built.write_text("")
            self.assertEqual(find_step_cli(None, root, ""), built)
            self.assertEqual(find_step_cli(str(built), root, ""), built)
            on_path = root / "bin/step-cli"
            on_path.parent.mkdir()
            on_path.write_text("")
            on_path.chmod(0o755)
            self.assertEqual(find_step_cli(None, root, str(on_path.parent)), on_path)


class CommandTest(StoreTestCase):
    def context(self, output=cli.OutputFormat.JSON):
        checkout = Checkout(
            self.root,
            {"COMPOSE_PROJECT_NAME": PROJECT, "DEVCONTAINER_NAME_PREFIX": "prefix-"},
        )
        return cli.Context(
            checkout,
            self.store,
            JOURNEY_ENVIRONMENT,
            Portals.from_environment(JOURNEY_ENVIRONMENT),
            output,
        )

    def run_command(self, function, *args):
        stdout = io.StringIO()
        with (
            contextlib.redirect_stdout(stdout),
            contextlib.redirect_stderr(io.StringIO()),
        ):
            code = function(*args)
        return code, stdout.getvalue()

    def test_urls_need_a_scenario_that_is_up(self):
        with self.assertRaisesRegex(ScenarioError, "step-dev scenario up kiosk-voter"):
            cli.command_urls(self.context(), KIOSK)

    def test_urls_print_links_and_synthetic_credentials(self):
        state = new_state(KIOSK.name, PROJECT, TENANT)
        state.event_id = "event"
        state.voters = catalog.census_usernames(KIOSK)
        self.store.save(state)
        code, output = self.run_command(cli.command_urls, self.context(), KIOSK)
        document = json.loads(output)
        self.assertEqual(code, cli.EXIT_OK)
        self.assertEqual(
            document["links"]["kiosk"],
            f"http://localhost:3000/tenant/{TENANT}/event/event/login?kiosk",
        )
        self.assertEqual(list(document["links"]), ["kiosk", "verifier", "admin"])
        credentials = document["credentials"]
        self.assertIs(credentials["synthetic"], True)
        self.assertEqual(credentials["voterPassword"], "same as username")
        self.assertEqual(credentials["voters"]["A"][0], "e2e-kiosk-voter-a1")
        self.assertEqual(
            credentials["admin"], {"username": "admin", "password": "admin"}
        )

    def test_text_urls_mark_the_credentials_synthetic(self):
        state = new_state(RESULTS.name, PROJECT, TENANT)
        state.event_id = "event"
        state.voters = catalog.census_usernames(RESULTS)
        self.store.save(state)
        _, output = self.run_command(
            cli.command_urls, self.context(cli.OutputFormat.TEXT), RESULTS
        )
        self.assertIn("synthetic fixture credentials, not real people", output)
        self.assertIn("results   http://localhost:3004/event", output)
        self.assertIn("e2e-published-results-a1 Alice", output)
        self.assertIn("use the delivered email code", output)
        self.assertIn("configure a test code in the isolated development realm", output)
        self.assertNotIn("docker logs", output)

    def test_status_without_state_needs_no_backend(self):
        with mock.patch.object(cli, "_backend") as backend:
            code, output = self.run_command(
                cli.command_status, self.context(), SCENARIOS
            )
        backend.assert_not_called()
        self.assertEqual(code, cli.EXIT_OK)
        document = json.loads(output)
        self.assertEqual(
            {name: entry["state"] for name, entry in document["scenarios"].items()},
            {scenario.name: "absent" for scenario in SCENARIOS},
        )

    def test_status_reports_an_unreachable_backend(self):
        state = new_state(KIOSK.name, PROJECT, TENANT)
        state.event_id = "event"
        self.store.save(state)
        unreachable = mock.Mock()
        unreachable.event_annotations.side_effect = OSError("connection refused")
        with mock.patch.object(cli, "_backend", return_value=unreachable):
            _, output = self.run_command(cli.command_status, self.context(), [KIOSK])
        entry = json.loads(output)["scenarios"]["kiosk-voter"]
        self.assertEqual(entry["eventId"], "event")
        self.assertIn("connection refused", entry["unverified"])

    def test_reset_without_state_touches_no_backend(self):
        with mock.patch.object(cli, "_backend") as backend:
            code, output = self.run_command(
                cli.command_reset, self.context(), KIOSK, None, None
            )
        backend.assert_not_called()
        self.assertEqual(code, cli.EXIT_OK)
        self.assertEqual(
            json.loads(output),
            {"scenario": "kiosk-voter", "outcome": "nothing to reset"},
        )

    def test_reset_without_state_still_reports_text(self):
        _, output = self.run_command(
            cli.command_reset, self.context(cli.OutputFormat.TEXT), KIOSK, None, None
        )
        self.assertEqual(output, "kiosk-voter: nothing to reset\n")

    def test_all_scenarios_report_created_then_reused_as_json(self):
        backend = FakeBackend()
        backend.close = mock.Mock()
        with (
            mock.patch.object(cli, "_backend", return_value=backend),
            mock.patch.object(cli, "_prepare"),
        ):
            for scenario in SCENARIOS:
                with self.subTest(scenario=scenario.name):
                    code, output = self.run_command(
                        cli.command_up, self.context(), scenario, None, None
                    )
                    first = json.loads(output)
                    self.assertEqual(code, cli.EXIT_OK)
                    self.assertEqual(first["outcome"], "created")
                    self.assertEqual(first["scenario"], scenario.name)
                    self.assertEqual(
                        list(first["links"]), [link.value for link in scenario.links]
                    )
                    code, output = self.run_command(
                        cli.command_up, self.context(), scenario, None, None
                    )
                    second = json.loads(output)
                    self.assertEqual(code, cli.EXIT_OK)
                    self.assertEqual(second["eventId"], first["eventId"])
                    self.assertEqual(second["outcome"], "reused")
                    self.assertEqual(second["stages"], {})
        self.assertEqual(backend.close.call_count, 2 * len(SCENARIOS))

    def test_commands_close_the_backend_when_preparation_fails(self):
        state = new_state(KIOSK.name, PROJECT, TENANT)
        state.event_id = "event"
        self.store.save(state)
        for command in (cli.command_up, cli.command_reset):
            with self.subTest(command=command.__name__):
                backend = mock.Mock()
                with (
                    mock.patch.object(cli, "_backend", return_value=backend),
                    mock.patch.object(
                        cli, "_prepare", side_effect=ScenarioError("login rejected")
                    ),
                ):
                    with self.assertRaisesRegex(ScenarioError, "login rejected"):
                        command(self.context(), KIOSK, None, None)
                backend.close.assert_called_once_with()
                self.assertEqual(self.store.load(KIOSK.name).event_id, "event")


class KeycloakLoginTest(unittest.TestCase):
    def test_realm_forms_use_the_configured_backend_origin_for_login_and_otp(self):
        for origin in ("http://keycloak:8090", "http://localhost:8090", ""):
            with self.subTest(form_origin=origin):
                self.check_login(origin)

    def check_login(self, origin):
        session = mock.Mock()
        callback = "http://127.0.0.1:3002/"
        state = {}

        def response(body, url, status=200, headers=None):
            return journey_client.Response(status, headers or {}, body.encode(), url)

        def login_page(url):
            query = urllib.parse.parse_qs(urllib.parse.urlsplit(url).query)
            state.update(value=query["state"][0], nonce=query["nonce"][0])
            return response(
                f'<form id="kc-form-login" action="{origin}/realms/tenant/'
                "login-actions/"
                'authenticate?session_code=first&amp;execution=password">'
                '<input name="username"><input name="password"></form>',
                url,
            )

        def submit(url, form):
            if "session_code=first" in url:
                return response(
                    f'<form action="{origin}/realms/tenant/login-actions/'
                    'authenticate?session_code=second&amp;execution=otp">'
                    '<input name="code"></form>',
                    url,
                )
            if "session_code=second" in url:
                return response(
                    "",
                    url,
                    302,
                    {
                        "Location": (
                            f"{callback}#state={state['value']}&code=authorization"
                        )
                    },
                )
            claims = base64.urlsafe_b64encode(
                json.dumps({"nonce": state["nonce"]}).encode()
            ).decode()
            return response(
                json.dumps(
                    {
                        "access_token": "accepted",
                        "id_token": f"header.{claims}.signature",
                    }
                ),
                url,
            )

        session.get.side_effect = login_page
        session.post.side_effect = submit
        with (
            mock.patch.object(journey_client, "Http", return_value=session),
            mock.patch.object(journey_client, "KEYCLOAK_URL", "http://keycloak:8090"),
        ):
            token = journey_client.Keycloak().browser_login(
                "tenant", "admin-portal", callback, "admin", "password", otp="123456"
            )
        self.assertEqual(token["access_token"], "accepted")
        calls = session.post.call_args_list
        self.assertEqual(
            [call.args[0] for call in calls],
            [
                "http://keycloak:8090/realms/tenant/login-actions/authenticate?session_code=first&execution=password",
                "http://keycloak:8090/realms/tenant/login-actions/authenticate?session_code=second&execution=otp",
                "http://keycloak:8090/realms/tenant/protocol/openid-connect/token",
            ],
        )
        self.assertEqual(calls[0].kwargs["form"]["username"], "admin")
        self.assertEqual(calls[1].kwargs["form"]["code"], "123456")
        self.assertEqual(calls[2].kwargs["form"]["code"], "authorization")


class FakeHasura:
    """Answers each query document with a function of its variables."""

    def __init__(self, answers):
        self.answers = answers

    def query(self, query, variables=None):
        return self.answers[query](variables or {})


class FakeKeycloak:
    def __init__(self, answers=None):
        self.answers = answers or {}
        self.requests = []

    def admin(self, method, path, json_body=None, expect=(200,)):
        self.requests.append((method, path, json_body))
        for prefix, answer in self.answers.items():
            if path.startswith(prefix):
                return answer
        return None


class StackTest(StoreTestCase):
    def setUp(self):
        super().setUp()
        checkout = Checkout(
            self.root,
            {"COMPOSE_PROJECT_NAME": PROJECT, "DEVCONTAINER_NAME_PREFIX": "prefix-"},
        )
        portals = Portals("http://localhost:3000", "http://localhost:3001", "x")
        environment = mock.patch.dict(os.environ, JOURNEY_ENVIRONMENT)
        environment.start()
        self.addCleanup(environment.stop)
        self.backend = backend_module.StackBackend(
            checkout,
            load_manifest(REPOSITORY_ROOT),
            portals,
            self.store.directory,
            self.say,
            timeout=0,
        )
        self.state = new_state(RESULTS.name, PROJECT, TENANT)
        self.state.event_id = "event"
        for name in ("sleep",):
            patcher = mock.patch.object(backend_module.time, name)
            patcher.start()
            self.addCleanup(patcher.stop)

    def states(self, **overrides):
        states = {
            service: container(service) for service in backend_module.REQUIRED_SERVICES
        }
        states.update(overrides)
        return states

    def test_a_stopped_service_points_to_the_backend_mode(self):
        states = self.states(windmill=container("windmill", "exited", exit_code=137))
        with (
            mock.patch.object(backend_module, "project_states", return_value=states),
            mock.patch.object(backend_module, "probe", return_value=True),
        ):
            with self.assertRaisesRegex(
                ScenarioError, r"windmill: exited.*step-dev mode up backend"
            ):
                self.backend.check_services()

    def test_a_missing_service_points_to_the_backend_mode(self):
        states = self.states()
        del states["b4v6"]
        with (
            mock.patch.object(backend_module, "project_states", return_value=states),
            mock.patch.object(backend_module, "probe", return_value=True),
        ):
            with self.assertRaisesRegex(ScenarioError, "b4v6: not created"):
                self.backend.check_services()

    def test_services_still_starting_are_awaited(self):
        starting = self.states(harvest=container("harvest", health="starting"))
        recreated = self.states(beat=container("beat", "created"))
        answers = iter([starting, recreated, self.states()])
        self.backend.timeout = None
        with (
            mock.patch.object(
                backend_module, "project_states", side_effect=lambda _: next(answers)
            ),
            mock.patch.object(backend_module, "probe", return_value=True),
        ):
            self.backend.check_services()

    def test_services_not_ready_in_time_are_listed(self):
        states = self.states()
        with (
            mock.patch.object(backend_module, "project_states", return_value=states),
            mock.patch.object(backend_module, "probe", return_value=False),
            mock.patch.object(backend_module.time, "monotonic", side_effect=[0, 0, 5]),
        ):
            with self.assertRaisesRegex(
                ScenarioError, r"not ready after 0s: .*windmill \(probe failing\)"
            ):
                self.backend.check_services()

    def test_running_trustees_are_left_alone(self):
        states = self.states(
            trustee1=container("trustee1"), trustee2=container("trustee2")
        )
        with (
            mock.patch.object(backend_module, "project_states", return_value=states),
            mock.patch.object(backend_module, "Compose") as compose,
        ):
            self.backend.start_trustees()
        compose.assert_not_called()

    def test_stopped_trustees_are_started_without_their_dependencies(self):
        stopped = self.states(trustee1=container("trustee1", "exited", exit_code=137))
        running = self.states(
            trustee1=container("trustee1"), trustee2=container("trustee2")
        )
        answers = iter([stopped, running])
        with (
            mock.patch.object(
                backend_module, "project_states", side_effect=lambda _: next(answers)
            ),
            mock.patch.object(backend_module, "Compose") as compose,
        ):
            compose.return_value.run.return_value = 0
            self.backend.timeout = None
            self.backend.start_trustees()
        compose.return_value.run.assert_called_once_with(
            "--profile",
            "wbraid",
            "up",
            "--detach",
            "--no-deps",
            "--no-recreate",
            "trustee1",
            "trustee2",
        )

    def test_compose_hostnames_that_do_not_resolve_stop_at_once(self):
        failure = socket.gaierror(socket.EAI_NONAME, "Name or service not known")
        with mock.patch.object(
            backend_module.socket, "getaddrinfo", side_effect=failure
        ):
            with self.assertRaisesRegex(
                ScenarioError, "graphql-engine does not resolve.*devcontainer"
            ):
                self.backend.authenticate()

    def test_the_administrator_password_is_set_before_signing_in(self):
        calls = mock.Mock()
        bootstrap = backend_module.bootstrap
        with (
            mock.patch.object(backend_module.socket, "getaddrinfo"),
            mock.patch.object(bootstrap, "tenant_row", return_value={"id": TENANT}),
            mock.patch.object(bootstrap, "realm_signing_kids", return_value={"kid"}),
            mock.patch.object(bootstrap, "published_kids", return_value={"kid"}),
            mock.patch.object(bootstrap, "set_admin_password", calls.set_password),
            mock.patch.object(bootstrap, "admin_token", calls.sign_in),
            mock.patch.object(
                bootstrap, "hasura_accepts_admin_token", return_value=True
            ),
            mock.patch.object(self.backend, "_restore_otp_test_mode"),
        ):
            self.backend.authenticate()
        self.assertEqual(
            [call[0] for call in calls.mock_calls], ["set_password", "sign_in"]
        )

    def test_a_timeout_reports_the_last_observation_and_hint(self):
        with self.assertRaisesRegex(
            ScenarioError,
            r"the ceremony: not done after 0s, last observed: IN_PROGRESS; look here",
        ):
            self.backend._wait(
                "the ceremony", lambda: (False, "IN_PROGRESS"), "keys", hint="look here"
            )

    def hasura(self, **answers):
        defaults = {
            backend_module.KEYS_CEREMONIES: [],
            backend_module.TALLY_SESSIONS: [],
            backend_module.TALLY_EXECUTION: [],
            backend_module.RESULTS_PUBLICATION: [],
            backend_module.EVENT: [{"id": "event", "annotations": {}, "status": {}}],
        }
        tables = {
            query: re.search(r"(sequent_backend_\w+)\(", query).group(1)
            for query in defaults
        }
        defaults.update(
            {getattr(backend_module, key): value for key, value in answers.items()}
        )
        self.backend.admin = FakeHasura(
            {
                query: (lambda rows, table: lambda _: {table: rows})(
                    rows, tables[query]
                )
                for query, rows in defaults.items()
            }
        )

    def test_keys_hold_once_a_ceremony_succeeded(self):
        self.hasura(KEYS_CEREMONIES=[{"id": "k", "execution_status": "IN_PROGRESS"}])
        self.assertFalse(self.backend.holds(Stage.KEYS, RESULTS, self.state))
        self.hasura(
            KEYS_CEREMONIES=[
                {"id": "k2", "execution_status": "FAILED"},
                {"id": "k1", "execution_status": "SUCCESS"},
            ]
        )
        self.assertTrue(self.backend.holds(Stage.KEYS, RESULTS, self.state))

    def test_voting_status_per_channel(self):
        status = {"voting_status": "CLOSED", "kiosk_voting_status": "OPEN"}
        self.hasura(EVENT=[{"id": "event", "annotations": {}, "status": status}])
        self.assertTrue(self.backend.holds(Stage.KIOSK_OPEN, KIOSK, self.state))
        self.assertFalse(self.backend.holds(Stage.ONLINE_OPEN, RESULTS, self.state))
        self.assertTrue(self.backend.holds(Stage.ONLINE_CLOSED, RESULTS, self.state))

    def test_votes_hold_when_every_planned_voter_voted(self):
        usernames = catalog.census_usernames(RESULTS)
        users = [
            {"username": name, "id": f"id-{name}"}
            for names in usernames.values()
            for name in names
        ]
        self.backend.keycloak = FakeKeycloak({"tenant-": users})
        voted = [
            f"id-{catalog.voter_username(RESULTS, vote)}" for vote in RESULTS.votes
        ]
        bodies = []

        def administrator(voters):
            def get_users(variables):
                bodies.append(variables["body"])
                return {
                    "get_users": {
                        "items": [
                            {
                                "id": user_id,
                                "votes_info": (
                                    [{"num_votes": 1}] if user_id in voters else []
                                ),
                            }
                            for user_id in variables["body"]["user_ids"]
                        ]
                    }
                }

            return lambda: FakeHasura({backend_module.VOTED_USERS: get_users})

        self.hasura()
        self.backend._administrator = administrator(voted[:-1])
        self.assertFalse(self.backend.holds(Stage.VOTES, RESULTS, self.state))
        self.backend._administrator = administrator(voted)
        self.assertTrue(self.backend.holds(Stage.VOTES, RESULTS, self.state))
        self.assertEqual(bodies[-1]["user_ids"], sorted(set(voted)))
        self.assertTrue(bodies[-1]["show_votes_info"])

    def test_tally_holds_with_stored_results_only(self):
        self.hasura(TALLY_SESSIONS=[{"id": "t", "execution_status": "SUCCESS"}])
        self.assertFalse(self.backend.holds(Stage.TALLY, RESULTS, self.state))
        self.hasura(
            TALLY_SESSIONS=[{"id": "t", "execution_status": "SUCCESS"}],
            TALLY_EXECUTION=[{"id": "x", "results_event_id": "r"}],
        )
        self.assertTrue(self.backend.holds(Stage.TALLY, RESULTS, self.state))

    def test_results_hold_while_the_latest_publication_is_published(self):
        for status, expected in (("Published", True), ("Revoked", False)):
            self.hasura(
                RESULTS_PUBLICATION=[
                    {"id": "p", "publication_status": status, "error_message": None}
                ]
            )
            self.assertIs(
                self.backend.holds(Stage.RESULTS, RESULTS, self.state), expected
            )
        self.hasura()
        self.assertFalse(self.backend.holds(Stage.RESULTS, RESULTS, self.state))

    def test_a_failed_ceremony_is_not_retried(self):
        self.hasura(KEYS_CEREMONIES=[{"id": "k", "execution_status": "FAILED"}])
        self.backend.cli = mock.Mock()
        with (
            mock.patch.object(backend_module.bootstrap, "configure_step_cli"),
            mock.patch.object(backend_module.bootstrap, "seed_trustees"),
        ):
            with self.assertRaisesRegex(
                ScenarioError, "keys ceremony k is FAILED.*scenario reset"
            ):
                self.backend.run(Stage.KEYS, RESULTS, self.state)
        self.backend.cli.step.assert_not_called()

    def test_a_failed_tally_requires_reset_without_starting_another(self):
        self.hasura(TALLY_SESSIONS=[{"id": "t", "execution_status": "FAILED"}])
        with mock.patch.object(self.backend, "_step") as step:
            with self.assertRaisesRegex(
                ScenarioError, "tally t is FAILED.*scenario reset"
            ):
                self.backend.run(Stage.TALLY, RESULTS, self.state)
        step.assert_not_called()

    def test_a_running_tally_is_resumed_and_waits_for_stored_results(self):
        self.hasura(TALLY_EXECUTION=[{"id": "x", "results_event_id": "r"}])
        sessions = iter(
            [
                [{"id": "t", "execution_status": "IN_PROGRESS"}],
                [{"id": "t", "execution_status": "SUCCESS"}],
            ]
        )
        self.backend.admin.answers[backend_module.TALLY_SESSIONS] = lambda _: {
            "sequent_backend_tally_session": next(sessions)
        }
        with mock.patch.object(self.backend, "_step") as step:
            self.backend.run(Stage.TALLY, RESULTS, self.state)
        step.assert_not_called()

    def test_a_successful_tally_without_stored_results_is_not_ready(self):
        self.hasura(TALLY_SESSIONS=[{"id": "t", "execution_status": "SUCCESS"}])
        with mock.patch.object(self.backend, "_step") as step:
            with self.assertRaisesRegex(
                ScenarioError, "electoral results tally: not done.*SUCCESS"
            ):
                self.backend.run(Stage.TALLY, RESULTS, self.state)
        step.assert_not_called()

    def test_a_running_publication_failure_reports_the_backend_error(self):
        self.state.ids = {"elections": {"main": "e"}, "contests": {"A": "c"}}
        self.hasura(
            TALLY_SESSIONS=[{"id": "t", "execution_status": "SUCCESS"}],
            TALLY_EXECUTION=[{"id": "x", "results_event_id": "r"}],
        )
        publications = iter(
            [
                [
                    {
                        "id": "p",
                        "publication_status": "Publishing",
                        "error_message": None,
                    }
                ],
                [
                    {
                        "id": "p",
                        "publication_status": "Failed",
                        "error_message": "upload refused",
                    }
                ],
            ]
        )
        self.backend.admin.answers[backend_module.RESULTS_PUBLICATION] = lambda _: {
            "sequent_backend_tally_results_publication": next(publications)
        }
        with mock.patch.object(self.backend, "_step") as step:
            with self.assertRaisesRegex(
                ScenarioError, "publication p failed: upload refused"
            ):
                self.backend.run(Stage.RESULTS, RESULTS, self.state)
        step.assert_not_called()

    def test_deletion_waits_for_both_the_event_row_and_its_realm(self):
        self.hasura(EVENT=[])
        self.backend.timeout = None
        self.backend.keycloak = mock.Mock()
        responses = [
            journey_client.Response(status, {}, b"", "") for status in (200, 404)
        ]
        with (
            mock.patch.object(self.backend, "_step") as step,
            mock.patch.object(backend_module, "Http") as http,
        ):
            http.return_value.get.side_effect = responses
            self.backend.delete_event("event")
        step.assert_called_once_with(
            "delete-election-event", "--election-event-id", "event"
        )
        self.assertEqual(http.return_value.get.call_count, 2)
        self.assertTrue(
            all(
                call.args[0].endswith(f"/admin/realms/tenant-{TENANT}-event-event")
                for call in http.return_value.get.call_args_list
            )
        )

    def test_portals_already_accepted_are_not_changed(self):
        client = {
            "rootUrl": "http://localhost:3000",
            "redirectUris": ["/*", "http://localhost:3001/*"],
            "webOrigins": ["*"],
        }
        self.backend.keycloak = FakeKeycloak({"tenant-": [client]})
        with mock.patch.object(backend_module, "allow_portals") as allow:
            self.backend.point_portals(self.state)
        allow.assert_not_called()

    def test_portals_elsewhere_are_allowed_on_the_event_realm_only(self):
        client = {"rootUrl": "http://localhost:3100", "redirectUris": ["/*"]}
        self.backend.keycloak = FakeKeycloak({"tenant-": [client]})
        with mock.patch.object(backend_module, "allow_portals") as allow:
            self.backend.point_portals(self.state)
        realm = f"tenant-{TENANT}-event-event"
        self.assertEqual(
            allow.call_args_list,
            [
                mock.call(
                    self.backend.keycloak,
                    realm,
                    "voting-portal",
                    ["http://localhost:3000", "http://localhost:3001"],
                ),
                mock.call(
                    self.backend.keycloak,
                    realm,
                    "voting-portal-kiosk",
                    ["http://localhost:3000"],
                ),
            ],
        )

    def test_an_interrupted_enrollment_restores_the_otp_test_mode(self):
        config = {"id": "c", "config": {"test-mode": "true", "test-mode-code": "1"}}
        self.backend.keycloak = FakeKeycloak({"tenant-": config})
        self.store.directory.mkdir(parents=True)
        (self.store.directory / backend_module.OTP_MARKER).write_text(
            json.dumps({"configId": "c", "testMode": "false"})
        )
        self.backend._restore_otp_test_mode()
        (method, path, body) = self.backend.keycloak.requests[-1]
        self.assertEqual(method, "PUT")
        self.assertEqual(path, f"tenant-{TENANT}/authentication/config/c")
        self.assertEqual(body["config"]["test-mode"], "false")
        self.assertFalse((self.store.directory / backend_module.OTP_MARKER).exists())

    def test_enrollment_restores_the_test_mode_even_when_it_fails(self):
        configs = [
            {"id": "c", "config": {"test-mode": "false"}},
            {"id": "c", "config": {"test-mode": "true"}},
        ]
        keycloak = FakeKeycloak()
        keycloak.admin = mock.Mock(side_effect=[configs[1], None])
        self.backend.keycloak = keycloak
        with (
            mock.patch.object(
                backend_module.bootstrap, "email_otp_config", return_value=configs[0]
            ),
            mock.patch.object(
                backend_module.bootstrap,
                "enroll_admin_mfa",
                side_effect=AssertionError("login failed"),
            ),
        ):
            with self.assertRaisesRegex(AssertionError, "login failed"):
                self.backend._enroll_administrator()
        put = keycloak.admin.call_args_list[-1]
        self.assertEqual(put.args[0], "PUT")
        self.assertEqual(put.args[2]["config"]["test-mode"], "false")
        self.assertFalse((self.store.directory / backend_module.OTP_MARKER).exists())

    def organization_backend(self, tenant_rows):
        from scripts.dev.scenario import tenants

        backend = backend_module.StackBackend(
            Checkout(self.root, {"COMPOSE_PROJECT_NAME": PROJECT}),
            load_manifest(REPOSITORY_ROOT),
            Portals("http://localhost:3000", "http://localhost:3001", "x"),
            self.store.directory,
            self.say,
            timeout=0,
            organization=tenants.STAFF_ASSOCIATION,
        )
        backend.admin = FakeHasura(
            {tenants.TENANT_BY_SLUG: lambda _: {"sequent_backend_tenant": tenant_rows}}
        )
        return backend

    def test_an_organization_scenario_runs_in_its_own_tenant(self):
        from scripts.dev.scenario import tenants

        backend = self.organization_backend([{"id": "org"}])
        self.assertEqual(backend.tenant_id, "org")
        self.assertEqual(backend.realm, "tenant-org")
        self.assertEqual(backend._event_realm("e"), "tenant-org-event-e")
        backend.keycloak = FakeKeycloak(
            {
                "tenant-org/clients?": [{"id": "client"}],
                "tenant-org/clients/client/client-secret": {"value": "secret"},
            }
        )
        backend.keycloak.password_grant = mock.Mock(
            return_value={"access_token": "token"}
        )
        backend.cli = mock.Mock()
        backend._configure_cli()
        args = backend.cli.step.call_args.args
        self.assertEqual(args[args.index("--tenant-id") + 1], "org")
        self.assertEqual(args[args.index("--keycloak-client-secret") + 1], "secret")
        self.assertEqual(
            backend.keycloak.password_grant.call_args.args[:2],
            ("tenant-org", tenants.API_KEY_CLIENT),
        )

    def test_a_missing_organization_tenant_is_named(self):
        backend = self.organization_backend([])
        with self.assertRaisesRegex(ScenarioError, "staff-association does not exist"):
            backend.tenant_id

    def test_the_test_mode_is_restored_in_the_recorded_realm(self):
        config = {"id": "c", "config": {"test-mode": "true"}}
        self.backend.keycloak = FakeKeycloak({"tenant-org/": config})
        self.store.directory.mkdir(parents=True)
        (self.store.directory / backend_module.OTP_MARKER).write_text(
            json.dumps({"realm": "tenant-org", "configId": "c", "testMode": "false"})
        )
        self.backend._restore_otp_test_mode()
        (method, path, body) = self.backend.keycloak.requests[-1]
        self.assertEqual((method, path), ("PUT", "tenant-org/authentication/config/c"))
        self.assertEqual(body["config"]["test-mode"], "false")

    def test_required_services_belong_to_the_backend_mode(self):
        manifest = load_manifest(REPOSITORY_ROOT)
        backend_services = set(manifest.mode(backend_module.BACKEND_MODE).services)
        self.assertLessEqual(set(backend_module.REQUIRED_SERVICES), backend_services)
        self.assertLessEqual(set(backend_module.TRUSTEE_SERVICES), backend_services)


if __name__ == "__main__":
    unittest.main()
