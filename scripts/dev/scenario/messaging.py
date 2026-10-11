# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Messaging on the development stack's fake provider routes, as data.

Each profile is one organization's messaging: its sending accounts (with
synthetic credentials, sent only to harvest's fake provider routes, which the
development ``.env`` points the provider endpoints at) and its event's
messaging configuration. The ``messaging`` scenario uses the first
organization's profile, in the stack's super tenant;
``messaging-second-organization`` another organization's, in its own tenant
(``tenants.STAFF_ASSOCIATION``), with accounts on all five channels whose
senders differ from the first organization's (the fake provider reports a
message only when exactly one account uses its sender), other purposes and
labels, and channels restricted per election.

The scenario's event realm is the development event realm, which has the
single-channel sign-in step (``message channel OTP``, conditional on the
voter's ``message-channel``), instead of the journeys' password-only one.

Print a profile's accounts and event configuration with::

    python3 -m scripts.dev.scenario.messaging messaging
"""

from __future__ import annotations

import argparse
import copy
import json
import sys
import uuid
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[3]
DEV_REALM = (
    ROOT
    / ".devcontainer/keycloak/import"
    / (
        "tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-"
        "33f18502-a67c-4853-8333-a58630663559.json"
    )
)

# The template names the fake provider reports as approved, in every
# language (harvest routes/fake_provider.rs FAKE_TEMPLATES).
CODE_TEMPLATE = "step_code"
NOTICE_TEMPLATE = "step_notice"
NOTICE_ALIAS = "Demo notification"
NOTICE_TEMPLATES = """query($tenant: uuid!, $alias: String!) {
  sequent_backend_template(
    where: {tenant_id: {_eq: $tenant}, alias: {_eq: $alias}}) { id }
}"""
INSERT_NOTICE_TEMPLATE = """mutation($object: sequent_backend_template_insert_input!) {
  insert_sequent_backend_template_one(object: $object,
    on_conflict: {constraint: communication_template_pkey, update_columns: []}) { id }
}"""
LANGUAGES = ("en", "es", "ca", "eu", "fr", "gl", "nl", "tl")
OTP = "OTP"
NOTICE = "NOTICE"
CONFIG_VERSION = 1
GRAPH_API_VERSION = "v23.0"
# Synthetic values; only the fake provider routes ever receive them.
META_CREDENTIALS = {
    "ACCESS_TOKEN": "fake-provider-access-token",
    "APP_SECRET": "fake-provider-app-secret",
}
INFOBIP_CREDENTIALS = {"API_KEY": "fake-provider-api-key"}
AWS_CREDENTIALS = {
    "AWS_ACCESS_KEY_ID": "AKIAFAKEPROVIDER0001",
    "AWS_SECRET_ACCESS_KEY": "fake-provider-secret-access-key",
}

ACCOUNT_ROLE = "messaging-account-write"
CONFIG_ROLE = "messaging-config-write"

UPSERT = """mutation ($name: String!, $sender: jsonb!, $is_default: Boolean) {
  upsert_messaging_account(name: $name, sender: $sender, is_default: $is_default) {
    id callback_url callback_reason
  }
}"""
CREDENTIALS = """mutation ($id: uuid!, $credentials: jsonb!, $verify: Boolean,
    $report: Boolean) {
  replace_messaging_account_credentials(id: $id, credentials: $credentials,
      generate_verify_token: $verify, generate_report_credentials: $report) {
    replaced
  }
}"""
CHECK = """mutation ($id: uuid!) {
  check_messaging_account(id: $id) { status callback_reason }
}"""
SAVE_CONFIG = """mutation ($event: uuid!, $config: jsonb!) {
  update_event_messaging_config(election_event_id: $event, config: $config) {
    errors
  }
}"""
ACCOUNTS = """query ($tenant: uuid!, $names: [String!]!) {
  sequent_backend_messaging_account(
    where: {tenant_id: {_eq: $tenant}, name: {_in: $names}}
  ) { id name sender credentials }
}"""
RENAME_ACCOUNT = """mutation (
  $tenant: uuid!, $id: uuid!, $name: String!, $sender: jsonb!
) {
  update_sequent_backend_messaging_account(
    where: {tenant_id: {_eq: $tenant}, id: {_eq: $id}},
    _set: {name: $name, sender: $sender}
  ) { affected_rows }
}"""
DEFAULT_ACCOUNTS = """query ($tenant: uuid!) {
  sequent_backend_messaging_account(
    where: {tenant_id: {_eq: $tenant}, is_default: {_eq: true}}
  ) { name channel }
}"""
# The account checks' reasons (messaging/src/providers/mod.rs) that mean the
# provider wasn't reached as expected, and the start of a missing one.
CONNECTION_REASONS = (
    "The provider couldn't be reached",
    "The provider answered with status ",
    "The provider's check failed: ",
)
MISSING_CREDENTIAL = "Missing credential: "
ELECTIONS = """query ($event: uuid!) {
  sequent_backend_election(where: {election_event_id: {_eq: $event}}) {
    id presentation
  }
}"""
EVENT_CONFIG = """query ($event: uuid!) {
  sequent_backend_messaging_event_config(
    where: {election_event_id: {_eq: $event}}
  ) { config }
}"""


@dataclass(frozen=True)
class Account:
    """A sending account, found again by its name."""

    key: str
    name: str
    channel: str
    sender: dict[str, Any]
    credentials: dict[str, str]
    is_default: bool = False
    legacy_name: str | None = None
    # Meta: the subscription's verify token; Infobip: the report Basic auth.
    generate_verify_token: bool = False
    generate_report_credentials: bool = False


@dataclass(frozen=True)
class Channel:
    """One channel of the event: its account, purposes and templates."""

    account: str
    purposes: tuple[str, ...]
    # Purpose -> template name, bound in every language.
    templates: dict[str, str] = field(default_factory=dict)


@dataclass(frozen=True)
class Profile:
    accounts: tuple[Account, ...]
    channels: tuple[Channel, ...]
    # Election name (English) -> the only channels its voters can use.
    election_channels: dict[str, tuple[str, ...]] = field(default_factory=dict)
    # The languages templates are bound in.
    languages: tuple[str, ...] = LANGUAGES

    def account(self, key: str) -> Account:
        return next(account for account in self.accounts if account.key == key)


def _whatsapp(
    key: str, name: str, number: str, display: str, default: bool, sender_name: str
) -> Account:
    return Account(
        key=key,
        name=name,
        legacy_name="Fake WhatsApp" if name == "Demo WhatsApp" else None,
        channel="WHATSAPP",
        sender={
            "provider": "WHATSAPP_CLOUD_API",
            "business_account_id": f"demo-waba-{number}",
            "phone_number_id": f"demo-wa-number-{number}",
            "display_phone_number": display,
            "display_name": sender_name,
            "api_version": GRAPH_API_VERSION,
        },
        credentials=META_CREDENTIALS,
        is_default=default,
        generate_verify_token=True,
    )


def _templates(
    by_purpose: dict[str, str], languages: tuple[str, ...] = LANGUAGES
) -> dict[str, dict[str, str]]:
    return {
        purpose: {language: name for language in languages}
        for purpose, name in by_purpose.items()
    }


FIRST_ORGANIZATION = Profile(
    accounts=(
        _whatsapp(
            "whatsapp", "Demo WhatsApp", "1", "+1 555 010 0001", True, "Demo WhatsApp"
        ),
        Account(
            key="viber",
            name="Demo Viber",
            legacy_name="Fake Viber",
            channel="VIBER",
            sender={
                "provider": "VIBER_INFOBIP",
                # Replaced by MESSAGING_INFOBIP_URL (the fake routes).
                "base_url": "https://demo-infobip.example.org",
                "sender": "DemoElections",
                "approved_templates": _templates(
                    {OTP: CODE_TEMPLATE, NOTICE: NOTICE_TEMPLATE}
                ),
            },
            credentials=INFOBIP_CREDENTIALS,
            is_default=True,
            generate_report_credentials=True,
        ),
        Account(
            key="messenger",
            name="Demo Messenger",
            legacy_name="Fake Messenger",
            channel="MESSENGER",
            sender={
                "provider": "MESSENGER_SEND_API",
                "page_id": "100000000000001",
                "page_name": "Elections",
                "page_username": "demo.elections",
                "api_version": GRAPH_API_VERSION,
            },
            credentials=META_CREDENTIALS,
            is_default=True,
            generate_verify_token=True,
        ),
        Account(
            key="sms",
            name="Demo SMS",
            legacy_name="Fake SMS",
            channel="SMS",
            sender={
                "provider": "AWS_SNS",
                "sender_id": "Elections",
                "region": "us-east-1",
            },
            credentials=AWS_CREDENTIALS,
            is_default=True,
        ),
        Account(
            key="email",
            name="Demo Email",
            legacy_name="Fake email",
            channel="EMAIL",
            sender={
                "provider": "AWS_SES",
                "from_address": "elections@demo-provider.example.org",
                "from_name": "Elections",
                "region": "us-east-1",
            },
            credentials=AWS_CREDENTIALS,
            is_default=True,
        ),
    ),
    channels=(
        Channel(
            "whatsapp", (OTP, NOTICE), {OTP: CODE_TEMPLATE, NOTICE: NOTICE_TEMPLATE}
        ),
        Channel("viber", (OTP, NOTICE), {OTP: CODE_TEMPLATE, NOTICE: NOTICE_TEMPLATE}),
        # Notices outside the 24-hour window go as the utility template.
        Channel("messenger", (OTP, NOTICE), {NOTICE: NOTICE_TEMPLATE}),
        Channel("sms", (OTP, NOTICE)),
        Channel("email", (OTP, NOTICE)),
    ),
)

# The second organization, in its own tenant: accounts on all five channels
# (each its tenant's default) whose senders differ from the first
# organization's, other purposes and labels, and channels restricted per
# election (tenants.STAFF_ASSOCIATION names the elections). Email goes through
# SES: the development stack has no SMTP server.
SECOND_LANGUAGES = ("en", "es")
SECOND_ORGANIZATION = Profile(
    accounts=(
        _whatsapp(
            "whatsapp",
            "Staff Association WhatsApp",
            "2",
            "+34 910 000 002",
            True,
            "Staff Association",
        ),
        Account(
            key="viber",
            name="Staff Association Viber",
            channel="VIBER",
            sender={
                "provider": "VIBER_INFOBIP",
                # Replaced by MESSAGING_INFOBIP_URL (the fake routes).
                "base_url": "https://demo-infobip.example.org",
                "sender": "StaffAssociation",
                "approved_templates": _templates(
                    {NOTICE: NOTICE_TEMPLATE}, SECOND_LANGUAGES
                ),
            },
            credentials=INFOBIP_CREDENTIALS,
            is_default=True,
            generate_report_credentials=True,
        ),
        Account(
            key="messenger",
            name="Staff Association Messenger",
            channel="MESSENGER",
            sender={
                "provider": "MESSENGER_SEND_API",
                "page_id": "100000000000002",
                "page_name": "Staff Association",
                "page_username": "demo.staff.association",
                "api_version": GRAPH_API_VERSION,
            },
            credentials=META_CREDENTIALS,
            is_default=True,
            generate_verify_token=True,
        ),
        Account(
            key="sms",
            name="Staff Association SMS",
            channel="SMS",
            sender={
                "provider": "AWS_SNS",
                "sender_id": "StaffAssoc",
                "region": "eu-west-1",
            },
            credentials=AWS_CREDENTIALS,
            is_default=True,
        ),
        Account(
            key="email",
            name="Staff Association email",
            channel="EMAIL",
            sender={
                "provider": "AWS_SES",
                "from_address": "votes@staff-association.demo-provider.example.org",
                "from_name": "Staff Association",
                "region": "eu-west-1",
            },
            credentials=AWS_CREDENTIALS,
            is_default=True,
        ),
    ),
    channels=(
        Channel(
            "whatsapp", (OTP, NOTICE), {OTP: CODE_TEMPLATE, NOTICE: NOTICE_TEMPLATE}
        ),
        Channel("viber", (NOTICE,), {NOTICE: NOTICE_TEMPLATE}),
        Channel("messenger", (OTP,)),
        Channel("sms", (OTP,)),
        Channel("email", (OTP, NOTICE)),
    ),
    election_channels={
        "Council: Canary Islands office": ("WHATSAPP", "SMS", "EMAIL"),
        "Pension board": ("EMAIL",),
    },
    languages=SECOND_LANGUAGES,
)

PROFILES = {
    "messaging": FIRST_ORGANIZATION,
    "messaging-second-organization": SECOND_ORGANIZATION,
}


class MessagingError(RuntimeError):
    """The backend refused part of the messaging setup; the message says what."""


def event_config(
    profile: Profile, account_ids: dict[str, str], election_ids: dict[str, str]
) -> dict[str, Any]:
    """The event's messaging configuration, with the stored account IDs.

    ``election_ids`` maps each election's English name to its ID.
    """
    missing = sorted(set(profile.election_channels) - set(election_ids))
    if missing:
        raise MessagingError(
            f"the event has no election named {', '.join(missing)}; it has "
            f"{', '.join(sorted(election_ids)) or 'none'}"
        )
    channels = []
    for channel in profile.channels:
        account = profile.account(channel.account)
        channels.append(
            {
                "channel": account.channel,
                "account_id": account_ids[account.key],
                "purposes": list(channel.purposes),
                "templates": [
                    {
                        "purpose": purpose,
                        "language": language,
                        "provider_template": name,
                    }
                    for purpose, name in channel.templates.items()
                    for language in profile.languages
                ],
            }
        )
    return {
        "version": CONFIG_VERSION,
        "channels": channels,
        "election_channels": {
            election_ids[election]: list(allowed)
            for election, allowed in profile.election_channels.items()
        },
    }


def dev_event_realm(fixture_realm: dict[str, Any]) -> dict[str, Any]:
    """The development event realm in place of the fixture's, without users.

    Keeps the fixture realm's name and the adjustments the journeys' fixture
    made to it (portal client URLs, the password field policy), so the
    import and the portals work as they do for the other scenarios.
    """
    realm = json.loads(DEV_REALM.read_text(encoding="utf-8"))
    realm.pop("users", None)
    for name in ("id", "realm", "displayName"):
        if name in fixture_realm:
            realm[name] = fixture_realm[name]
    realm.setdefault("attributes", {}).update(
        {
            key: value
            for key, value in (fixture_realm.get("attributes") or {}).items()
            if key.startswith("credential-input")
        }
    )
    for config in realm.get("authenticatorConfig") or []:
        if "matchAttributes" in config["config"]:
            config["config"]["matchAttributes"] = "username"
    for messages in (realm.get("localizationTexts") or {}).values():
        messages["loginCustomCss"] = ""
    fixture_clients = {
        client["clientId"]: client for client in fixture_realm.get("clients") or []
    }
    existing_clients = {client["clientId"] for client in realm.get("clients") or []}
    for client_id, client in fixture_clients.items():
        if client_id not in existing_clients:
            added = copy.deepcopy(client)
            # Fixture flow IDs belong to a different realm. New portal clients
            # inherit this realm's single-channel browser policy.
            added.pop("authenticationFlowBindingOverrides", None)
            realm.setdefault("clients", []).append(added)
    for client in realm.get("clients") or []:
        source = fixture_clients.get(client["clientId"])
        if source is None:
            continue
        for key in ("rootUrl", "baseUrl", "redirectUris", "webOrigins"):
            if key in source:
                client[key] = copy.deepcopy(source[key])
    return realm


def has_single_channel_step(realm: dict[str, Any]) -> bool:
    """Whether the realm's sign-in asks for a code on the voter's channel."""
    return any(
        execution.get("authenticator") == "message-otp-authenticator"
        for flow in realm.get("authenticationFlows") or []
        for execution in flow.get("authenticationExecutions") or []
    )


class Graphql:
    """Hasura actions as the tenant administrator, in one role each."""

    def __init__(self, http: Any, url: str, token: str) -> None:
        self.http = http
        self.url = url
        self.token = token

    def run(self, role: str, query: str, variables: dict[str, Any]) -> dict[str, Any]:
        response = self.http.post(
            self.url,
            json_body={"query": query, "variables": variables},
            headers={
                "Authorization": f"Bearer {self.token}",
                "x-hasura-role": role,
            },
        )
        if response.status != 200:
            raise MessagingError(
                f"Hasura answered HTTP {response.status}: {response.text[:300]}"
            )
        result = response.json()
        if result.get("errors"):
            messages = "; ".join(error.get("message", "") for error in result["errors"])
            raise MessagingError(messages)
        return result["data"]


def demo_label(value: str) -> str:
    """The old synthetic sender labels, retaining real provider credentials."""
    return (
        value.replace("Fake", "Demo")
        .replace("fake-", "demo-")
        .replace("fake.", "demo.")
    )


def rename_demo_account(
    admin: Any, tenant_id: str, account: Account, current: dict[str, Any]
) -> bool:
    """Update this fixture's display metadata without replacing its shared account."""
    sender = {
        key: demo_label(value) if isinstance(value, str) else value
        for key, value in (current.get("sender") or {}).items()
    }
    if current["name"] != account.legacy_name and sender == current.get("sender", {}):
        return False
    renamed = admin.query(
        RENAME_ACCOUNT,
        {
            "tenant": tenant_id,
            "id": current["id"],
            "name": account.name,
            "sender": sender,
        },
    )["update_sequent_backend_messaging_account"]["affected_rows"]
    if renamed != 1:
        raise MessagingError(f"could not rename demo account {account.name}")
    return True


def stored_rows(admin: Any, tenant_id: str, profile: Profile) -> dict[str, dict]:
    """The profile's accounts the tenant already has, by key."""
    names = {
        name: account.key
        for account in profile.accounts
        for name in (account.name, account.legacy_name)
        if name is not None
    }
    rows = admin.query(ACCOUNTS, {"tenant": tenant_id, "names": list(names)})[
        "sequent_backend_messaging_account"
    ]
    return {names[row["name"]]: row for row in rows}


def stored_accounts(admin: Any, tenant_id: str, profile: Profile) -> dict[str, str]:
    """The IDs of the profile's accounts the tenant already has, by key."""
    return {
        key: row["id"] for key, row in stored_rows(admin, tenant_id, profile).items()
    }


def expected_credentials(account: Account) -> set[str]:
    """The credential names a ready account has: the entered ones and the
    ones Step generates for it."""
    names = set(account.credentials)
    if account.generate_verify_token:
        names.add("VERIFY_TOKEN")
    if account.generate_report_credentials:
        names |= {"REPORT_USERNAME", "REPORT_PASSWORD"}
    return names


def tenant_defaults(admin: Any, tenant_id: str) -> dict[str, str]:
    """The tenant's default account names, by channel."""
    rows = admin.query(DEFAULT_ACCOUNTS, {"tenant": tenant_id})[
        "sequent_backend_messaging_account"
    ]
    return {row["channel"]: row["name"] for row in rows}


def election_ids(admin: Any, event_id: str) -> dict[str, str]:
    """The event's election IDs by English name."""
    rows = admin.query(ELECTIONS, {"event": event_id})["sequent_backend_election"]
    return {
        ((row.get("presentation") or {}).get("i18n") or {})
        .get("en", {})
        .get("name"): row["id"]
        for row in rows
    }


def ensure_notice_template(admin: Any, tenant_id: str) -> None:
    """Seed an editable demo notification without replacing an existing template."""
    if admin.query(NOTICE_TEMPLATES, {"tenant": tenant_id, "alias": NOTICE_ALIAS})[
        "sequent_backend_template"
    ]:
        return
    message = "Hello {{user.first_name}}, voting is open. Visit {{vote_url}} to vote."
    instant = {
        "message": message,
        "parameters": ["{{user.first_name}}", "{{vote_url}}"],
        "provider_template": NOTICE_TEMPLATE,
        "provider_language": "en",
    }
    content = {
        "alias": NOTICE_ALIAS,
        "selected_methods": dict.fromkeys(
            ("EMAIL", "SMS", "WHATSAPP", "VIBER", "MESSENGER"), True
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
    admin.query(
        INSERT_NOTICE_TEMPLATE,
        {
            "object": {
                "id": str(
                    uuid.uuid5(uuid.NAMESPACE_URL, "step-demo-notice:" + tenant_id)
                ),
                "tenant_id": tenant_id,
                "alias": NOTICE_ALIAS,
                "communication_method": "EMAIL",
                "type": "CREDENTIALS",
                "created_by": "demo-setup",
                "template": content,
            }
        },
    )


def configured(admin: Any, tenant_id: str, event_id: str, profile: Profile) -> bool:
    """Whether the accounts exist and the event has a messaging configuration."""
    rows = stored_rows(admin, tenant_id, profile)
    if len(rows) < len(profile.accounts) or any(
        rows[account.key]["name"] != account.name
        or any(
            isinstance(value, str) and demo_label(value) != value
            for value in (rows[account.key].get("sender") or {}).values()
        )
        for account in profile.accounts
    ):
        return False
    return bool(
        admin.query(EVENT_CONFIG, {"event": event_id})[
            "sequent_backend_messaging_event_config"
        ]
    ) and bool(
        admin.query(NOTICE_TEMPLATES, {"tenant": tenant_id, "alias": NOTICE_ALIAS})[
            "sequent_backend_template"
        ]
    )


def _set_credentials(graphql: Graphql, account: Account, account_id: str) -> None:
    try:
        graphql.run(
            ACCOUNT_ROLE,
            CREDENTIALS,
            {
                "id": account_id,
                "credentials": account.credentials,
                "verify": account.generate_verify_token,
                "report": account.generate_report_credentials,
            },
        )
    except MessagingError as error:
        raise MessagingError(
            f"{account.name}: its credentials couldn't be set ({error}); the "
            "next run sets them again"
        ) from error


def _not_ready(account: Account, status: dict[str, Any]) -> MessagingError:
    reason = status.get("reason") or json.dumps(status)
    message = f"{account.name} is not ready: {reason}"
    if any(reason.startswith(start) for start in CONNECTION_REASONS):
        message += (
            "; is MESSAGING_FAKE_PROVIDER_ROUTES=ENABLED and do the provider "
            "endpoint variables point at harvest's /fake-provider routes?"
        )
    return MessagingError(message)


def configure(
    graphql: Graphql,
    admin: Any,
    tenant_id: str,
    event_id: str,
    profile: Profile,
    say: Any,
) -> dict[str, str]:
    """Creates the missing accounts, checks every one, saves the event config.

    Existing accounts keep their credentials (a new verify token or report
    password would stop matching what was set up), unless a run stopped
    before setting them: then they are set again, and said so. A new account
    is its channel's default only when the tenant has none; the tenant's own
    default stays, and is named. Returns the account IDs.
    """
    rows = stored_rows(admin, tenant_id, profile)
    ids = {key: row["id"] for key, row in rows.items()}
    defaults = tenant_defaults(admin, tenant_id)
    for account in profile.accounts:
        if account.key not in ids:
            other = defaults.get(account.channel)
            is_default = account.is_default and other is None
            created = graphql.run(
                ACCOUNT_ROLE,
                UPSERT,
                {
                    "name": account.name,
                    "sender": account.sender,
                    "is_default": is_default,
                },
            )["upsert_messaging_account"]
            ids[account.key] = created["id"]
            _set_credentials(graphql, account, created["id"])
            say(f"    created account {account.name}")
            if is_default:
                defaults[account.channel] = account.name
                say(
                    f"    {account.name} is the tenant's default "
                    f"{account.channel} account"
                )
            elif account.is_default:
                say(
                    f"    {account.name} is not the default {account.channel} "
                    f"account: the tenant's own default, {other}, stays"
                )
        else:
            if rename_demo_account(admin, tenant_id, account, rows[account.key]):
                say(f"    renamed account {account.name}")
            missing = expected_credentials(account) - set(
                rows[account.key].get("credentials") or {}
            )
            if missing:
                _set_credentials(graphql, account, ids[account.key])
                say(
                    f"    set the credentials of {account.name} again "
                    f"(missing: {', '.join(sorted(missing))})"
                )
        checked = graphql.run(ACCOUNT_ROLE, CHECK, {"id": ids[account.key]})[
            "check_messaging_account"
        ]
        status = checked["status"] or {}
        if (status.get("reason") or "").startswith(MISSING_CREDENTIAL):
            _set_credentials(graphql, account, ids[account.key])
            say(f"    set the credentials of {account.name} again ({status['reason']})")
            checked = graphql.run(ACCOUNT_ROLE, CHECK, {"id": ids[account.key]})[
                "check_messaging_account"
            ]
            status = checked["status"] or {}
        if not (status.get("connected") and status.get("production_access")):
            raise _not_ready(account, status)
        if checked.get("callback_reason"):
            say(f"    {account.name}: {checked['callback_reason']}")
    config = event_config(profile, ids, election_ids(admin, event_id))
    errors = graphql.run(
        CONFIG_ROLE, SAVE_CONFIG, {"event": event_id, "config": config}
    )["update_event_messaging_config"]["errors"]
    if errors:
        raise MessagingError(
            f"the event configuration was refused: {json.dumps(errors)}"
        )
    ensure_notice_template(admin, tenant_id)
    return ids


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("profile", choices=sorted(PROFILES))
    args = parser.parse_args(argv)
    profile = PROFILES[args.profile]
    ids = {account.key: f"<{account.name}>" for account in profile.accounts}
    elections = {name: f"<{name}>" for name in profile.election_channels}
    print(
        json.dumps(
            {
                "accounts": [
                    {"name": a.name, "channel": a.channel, "sender": a.sender}
                    for a in profile.accounts
                ],
                "event_config": event_config(profile, ids, elections),
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
