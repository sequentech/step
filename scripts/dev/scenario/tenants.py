# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Organizations with a tenant of their own on the development stack.

Most scenarios run in the stack's super tenant. A scenario that names an
organization runs in that organization's tenant instead, created the way an
operator creates one (Tenants > Create, the ``insertTenant`` action) by the
super tenant's administrator. The new tenant's realm is the default tenant
realm, so it has the same administrator user and password; its API key client
gets a new secret, read here from Keycloak.

An organization also shapes its event: the journeys' fixture's elections get
the organization's names, more elections can be added, and the event uses the
organization's languages and timezones.
"""

from __future__ import annotations

import copy
import uuid
from dataclasses import dataclass, field
from typing import Any

from scripts.e2e.journeys import fixtures
from scripts.e2e.journeys.client import ENV, KEYCLOAK_URL, Hasura, http_get

# The client step-cli and the scenario's administrator tokens use.
API_KEY_CLIENT = "api-key-client"
ADMIN_ROLE = "admin-user"
EMAIL_OTP_SUBFLOW = "Email Message OTP Subflow"
EMAIL_OTP_AUTHENTICATOR = "message-otp-authenticator"
ADMIN_PORTAL_CLIENT = "admin-portal"
ADMIN_PORTAL_REDIRECT = "http://127.0.0.1:3002/"
ADMIN_EMAIL = "admin@example.invalid"
DEFAULT_TEST_CODE = "123456"

TENANT_BY_SLUG = """query ($slug: String!) {
  sequent_backend_tenant(where: {slug: {_eq: $slug}}) { id }
}"""
TASK = """query ($id: uuid!) {
  sequent_backend_tasks_execution(where: {id: {_eq: $id}}) {
    execution_status logs
  }
}"""
INSERT_TENANT = """mutation ($slug: String!) {
  insertTenant(slug: $slug) { id slug error_msg task_execution { id } }
}"""
APPEND_SETTINGS = """mutation ($id: uuid!, $settings: jsonb!) {
  update_sequent_backend_tenant_by_pk(
    pk_columns: {id: $id}, _append: {settings: $settings}
  ) { id }
}"""
TENANT_SETTINGS = """query ($id: uuid!) {
  sequent_backend_tenant_by_pk(id: $id) { settings }
}"""
OWN_TENANTS = "{ sequent_backend_tenant { id } }"


class TenantError(RuntimeError):
    """The organization's tenant could not be created or used; the message says why."""


@dataclass(frozen=True)
class Organization:
    """An organization with its own tenant, and how its events look."""

    slug: str
    display_name: str
    # The first language is the default one.
    languages: tuple[str, ...]
    # IANA names; the first is the event's primary timezone.
    timezones: tuple[str, ...]
    # The journeys' fixture's elections ("main", "grace") -> this organization's names.
    election_names: dict[str, str] = field(default_factory=dict)
    # More elections, each with one contest for the fixture's first area.
    extra_elections: tuple[str, ...] = ()

    @property
    def elections(self) -> tuple[str, ...]:
        return (*self.election_names.values(), *self.extra_elections)


STAFF_ASSOCIATION = Organization(
    slug="staff-association",
    display_name="Staff Association",
    languages=("en", "es"),
    timezones=("Europe/Madrid", "Atlantic/Canary"),
    election_names={
        "main": "Council: Central office",
        "grace": "Council: Canary Islands office",
    },
    extra_elections=("Pension board",),
)
ORGANIZATIONS = {
    organization.slug: organization for organization in (STAFF_ASSOCIATION,)
}


def find(slug: str | None) -> Organization | None:
    return ORGANIZATIONS[slug] if slug else None


def tenant_realm(tenant_id: str) -> str:
    return f"tenant-{tenant_id}"


def event_realm(tenant_id: str, event_id: str) -> str:
    return f"tenant-{tenant_id}-event-{event_id}"


def tenant_settings(organization: Organization) -> dict[str, Any]:
    """The keys merged into the tenant's ``settings``."""
    return {
        "display_name": organization.display_name,
        "language_conf": {
            "enabled_language_codes": list(organization.languages),
            "default_language_code": organization.languages[0],
        },
    }


def _language_conf(organization: Organization, existing: dict | None) -> dict:
    conf = dict(existing or {})
    conf.update(
        enabled_language_codes=list(organization.languages),
        default_language_code=organization.languages[0],
    )
    return conf


def organization_event(document: dict[str, Any], organization: Organization) -> dict:
    """The journeys' import bundle with the organization's elections, languages
    and timezones. The fixture's areas, contests and voters stay as they are, so
    the other stages work unchanged; each extra election gets one contest (named
    after it) for the fixture's first area.
    """
    event = copy.deepcopy(document)
    by_key = dict(zip(("main", "grace"), event["elections"], strict=False))
    for key, name in organization.election_names.items():
        fixtures._name(by_key[key], name)
    main = by_key["main"]
    first_area = event["areas"][0]
    template_contest = next(
        contest for contest in event["contests"] if contest["election_id"] == main["id"]
    )
    for name in organization.extra_elections:
        election = copy.deepcopy(main)
        election.update(id=str(uuid.uuid4()), external_id=None)
        fixtures._name(election, name)
        contest = copy.deepcopy(template_contest)
        contest.update(id=str(uuid.uuid4()), election_id=election["id"])
        fixtures._name(contest, name)
        candidates = [
            {
                **copy.deepcopy(candidate),
                "id": str(uuid.uuid4()),
                "contest_id": contest["id"],
            }
            for candidate in event["candidates"]
            if candidate["contest_id"] == template_contest["id"]
        ]
        event["elections"].append(election)
        event["contests"].append(contest)
        event["candidates"].extend(candidates)
        event["area_contests"].append(
            {
                "id": str(uuid.uuid4()),
                "area_id": first_area["id"],
                "contest_id": contest["id"],
            }
        )
    presentation = event["election_event"].setdefault("presentation", {})
    presentation["language_conf"] = _language_conf(
        organization, presentation.get("language_conf")
    )
    presentation["timezones"] = {
        "configured": list(organization.timezones),
        "primary": organization.timezones[0],
    }
    for election in event["elections"]:
        election_presentation = election.setdefault("presentation", {})
        election_presentation["language_conf"] = _language_conf(
            organization, election_presentation.get("language_conf")
        )
    return event


def find_tenant_id(admin: Hasura, slug: str) -> str | None:
    rows = admin.query(TENANT_BY_SLUG, {"slug": slug})["sequent_backend_tenant"]
    return rows[0]["id"] if rows else None


def create_tenant(graphql: Any, slug: str) -> tuple[str, str | None]:
    """Asks for the tenant as the super tenant's administrator.

    Returns the new tenant's ID and its task's ID; the tenant exists once the
    task created its realm and row.
    """
    created = graphql.run(ADMIN_ROLE, INSERT_TENANT, {"slug": slug})["insertTenant"]
    if created.get("error_msg"):
        raise TenantError(f"insertTenant {slug}: {created['error_msg']}")
    task = created.get("task_execution") or {}
    return created["id"], task.get("id")


def task_status(admin: Hasura, task_id: str) -> tuple[str, str]:
    """The task's execution status and its last log line."""
    rows = admin.query(TASK, {"id": task_id})["sequent_backend_tasks_execution"]
    if not rows:
        return "absent", ""
    logs = rows[0].get("logs") or []
    last = logs[-1] if isinstance(logs, list) and logs else ""
    return rows[0]["execution_status"], str(last)[:300]


def api_key_client_secret(keycloak: Any, realm: str) -> str:
    """The API key client's secret in the realm (each new tenant gets its own)."""
    (client,) = keycloak.admin("GET", f"{realm}/clients?clientId={API_KEY_CLIENT}")
    return keycloak.admin("GET", f"{realm}/clients/{client['id']}/client-secret")[
        "value"
    ]


def admin_token(keycloak: Any, realm: str, secret: str) -> str:
    """The realm administrator's token from the API key client, as step-cli gets it."""
    return keycloak.password_grant(
        realm,
        API_KEY_CLIENT,
        ENV["ADMIN_USERNAME"],
        ENV["ADMIN_PASSWORD"],
        secret,
    )["access_token"]


def _messaging_service_user(keycloak: Any, tenant_id: str) -> dict[str, Any]:
    realm = tenant_realm(tenant_id)
    (client,) = keycloak.admin("GET", f"{realm}/clients?clientId=service-account")
    return keycloak.admin("GET", f"{realm}/clients/{client['id']}/service-account-user")


def messaging_service_account_configured(keycloak: Any, tenant_id: str) -> bool:
    """The messaging service token's tenant mapper needs this user attribute."""
    user = _messaging_service_user(keycloak, tenant_id)
    return (user.get("attributes") or {}).get("tenant-id") == [tenant_id]


def configure_messaging_service_account(keycloak: Any, tenant_id: str) -> bool:
    """Repair older local realms without replacing unrelated service attributes."""
    user = _messaging_service_user(keycloak, tenant_id)
    attributes = dict(user.get("attributes") or {})
    if attributes.get("tenant-id") == [tenant_id]:
        return False
    attributes["tenant-id"] = [tenant_id]
    keycloak.admin(
        "PUT",
        f"{tenant_realm(tenant_id)}/users/{user['id']}",
        json_body={"attributes": attributes},
        expect=(204,),
    )
    return True


def realm_signing_kids(realm: str) -> set[str]:
    certs = http_get(
        f"{KEYCLOAK_URL}/realms/{realm}/protocol/openid-connect/certs"
    ).json()
    return {key["kid"] for key in certs["keys"] if key.get("use") == "sig"}


def hasura_accepts(token: str, tenant_id: str) -> bool:
    """Whether Hasura takes the token and scopes it to the tenant."""
    result = Hasura(token=token).execute(OWN_TENANTS)
    if result.get("errors"):
        return False
    return [row["id"] for row in result["data"]["sequent_backend_tenant"]] == [
        tenant_id
    ]


def email_otp_config(keycloak: Any, realm: str) -> dict[str, Any]:
    """The authenticator config of the realm browser flow's email code step."""
    flow = keycloak.admin("GET", realm)["browserFlow"]
    executions = keycloak.admin(
        "GET",
        f"{realm}/authentication/flows/{flow.replace(' ', '%20')}/executions",
    )
    in_email_subflow = False
    for execution in executions:
        if execution.get("authenticationFlow"):
            in_email_subflow = execution["displayName"] == EMAIL_OTP_SUBFLOW
        elif (
            in_email_subflow and execution.get("providerId") == EMAIL_OTP_AUTHENTICATOR
        ):
            return keycloak.admin(
                "GET",
                f"{realm}/authentication/config/{execution['authenticationConfig']}",
            )
    raise TenantError(f"no email code step in the {flow} flow of {realm}")


def enroll_admin_mfa(keycloak: Any, realm: str, config: dict[str, Any]) -> bool:
    """The administrator's first login in the realm, with the code test mode on.

    The same steps as the journeys' bootstrap for the super tenant: tenants
    require two-factor enrollment, and a password grant fails with "Account is
    not fully set up" until one browser login enrolled the email code. The
    caller switches the test mode on before and back off after.
    """
    admin = keycloak.user(realm, ENV["ADMIN_USERNAME"])
    path = f"{realm}/users/{admin['id']}"
    if any(
        c["type"] == "message-otp" for c in keycloak.admin("GET", f"{path}/credentials")
    ):
        return False
    if not admin.get("email"):
        profile = {
            key: value for key, value in admin.items() if key != "userProfileMetadata"
        }
        profile.update(email=ADMIN_EMAIL, emailVerified=True)
        keycloak.admin("PUT", path, profile, expect=(204,))
    keycloak.browser_login(
        realm,
        ADMIN_PORTAL_CLIENT,
        ADMIN_PORTAL_REDIRECT,
        ENV["ADMIN_USERNAME"],
        ENV["ADMIN_PASSWORD"],
        otp=config["config"].get("test-mode-code", DEFAULT_TEST_CODE),
    )
    credentials = keycloak.admin("GET", f"{path}/credentials")
    if not any(c["type"] == "message-otp" for c in credentials):
        raise TenantError(
            f"the administrator's login in {realm} did not enroll an email code"
        )
    return True


def configure_step_cli(cli: Any, tenant_id: str, secret: str) -> None:
    cli.step(
        "config",
        "--tenant-id", tenant_id,
        "--endpoint-url", ENV["HASURA_ENDPOINT"],
        "--keycloak-url", ENV["KEYCLOAK_URL"],
        "--keycloak-user", ENV["ADMIN_USERNAME"],
        "--keycloak-password", ENV["ADMIN_PASSWORD"],
        "--keycloak-client-id", API_KEY_CLIENT,
        "--keycloak-client-secret", secret,
    )  # fmt: skip


def reconcile_settings(
    graphql: Any, admin: Hasura, tenant_id: str, organization: Organization
) -> list[str]:
    """Writes the organization's settings the tenant doesn't have yet (all of
    them for a new tenant, or one a stopped run left without them). Settings
    the tenant has with other values are its own: they stay, and their names
    are returned so the caller says so."""
    row = admin.query(TENANT_SETTINGS, {"id": tenant_id})[
        "sequent_backend_tenant_by_pk"
    ]
    current = (row or {}).get("settings") or {}
    wanted = tenant_settings(organization)
    missing = {key: value for key, value in wanted.items() if key not in current}
    if missing:
        graphql.run(ADMIN_ROLE, APPEND_SETTINGS, {"id": tenant_id, "settings": missing})
    return sorted(
        key for key, value in wanted.items() if key in current and current[key] != value
    )
