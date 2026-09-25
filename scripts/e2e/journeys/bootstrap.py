# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Wait for a fresh stack's super tenant, then prepare its administrator and trustees.

Exit status 0 means ready, 3 means the JWKS is published but Hasura still
rejects tenant tokens (scripts/e2e/run.sh then restarts graphql-engine once).
"""

import json
import time
import tomllib

from .client import ENV, OUTPUT, ROOT, S3_URL, TENANT_ID, TENANT_REALM, Hasura, Keycloak, StepCli, http_get, wait_until

TRUSTEES = ("trustee1", "trustee2")
HASURA_RESTART_NEEDED = 3


def realm_signing_kids():
    """Key IDs Keycloak signs the tenant realm's tokens with."""
    certs = http_get(f"{ENV['KEYCLOAK_URL']}/realms/{TENANT_REALM}/protocol/openid-connect/certs").json()
    return {key["kid"] for key in certs["keys"] if key.get("use") == "sig"}


def published_kids():
    response = http_get(f"{S3_URL}/{ENV['AWS_S3_PUBLIC_BUCKET']}/{ENV['AWS_S3_JWKS_CERTS_PATH']}")
    return {key["kid"] for key in response.json()["keys"]} if response.status == 200 else set()


def tenant_row():
    rows = Hasura.admin().query(
        "query($id: uuid!) { sequent_backend_tenant(where: {id: {_eq: $id}}) { id slug } }", {"id": TENANT_ID}
    )["sequent_backend_tenant"]
    return rows[0] if rows else None


def email_otp_config(keycloak):
    """The authenticator config of the tenant browser flow's email OTP step."""
    flow = keycloak.admin("GET", TENANT_REALM)["browserFlow"]
    executions = keycloak.admin("GET", f"{TENANT_REALM}/authentication/flows/{flow.replace(' ', '%20')}/executions")
    in_email_subflow = False
    for execution in executions:
        if execution.get("authenticationFlow"):
            in_email_subflow = execution["displayName"] == "Email Message OTP Subflow"
        elif in_email_subflow and execution.get("providerId") == "message-otp-authenticator":
            return keycloak.admin("GET", f"{TENANT_REALM}/authentication/config/{execution['authenticationConfig']}")
    raise AssertionError(f"No email OTP step in the {flow} flow")


def enroll_admin_mfa(keycloak):
    """Complete the seeded administrator's first login, as an operator would.

    Tenants require two-factor enrollment, and a password grant fails with
    "Account is not fully set up" until it is done. The administrator gets an
    email address, and one browser login passes the email OTP step using the
    authenticator's own test mode, which then records the message-otp
    credential. The test mode is switched back off afterwards.
    """
    admin = keycloak.user(TENANT_REALM, ENV["ADMIN_USERNAME"])
    path = f"{TENANT_REALM}/users/{admin['id']}"
    if any(c["type"] == "message-otp" for c in keycloak.admin("GET", f"{path}/credentials")):
        return False
    if not admin.get("email"):
        profile = {key: value for key, value in admin.items() if key != "userProfileMetadata"}
        profile.update(email="admin@example.invalid", emailVerified=True)
        keycloak.admin("PUT", path, profile, expect=(204,))
    config = email_otp_config(keycloak)
    original = config["config"].get("test-mode", "false")
    config["config"]["test-mode"] = "true"
    keycloak.admin("PUT", f"{TENANT_REALM}/authentication/config/{config['id']}", config, expect=(204,))
    try:
        keycloak.browser_login(
            TENANT_REALM,
            "admin-portal",
            "http://127.0.0.1:3002/",
            ENV["ADMIN_USERNAME"],
            ENV["ADMIN_PASSWORD"],
            otp=config["config"].get("test-mode-code", "123456"),
        )
    finally:
        config["config"]["test-mode"] = original
        keycloak.admin("PUT", f"{TENANT_REALM}/authentication/config/{config['id']}", config, expect=(204,))
    credentials = keycloak.admin("GET", f"{path}/credentials")
    if not any(c["type"] == "message-otp" for c in credentials):
        raise AssertionError(f"Admin login did not enroll a message-otp credential: {credentials}")
    return True


def admin_token(keycloak):
    """An administrator token from the API key client, the one step-cli uses."""
    return keycloak.password_grant(
        TENANT_REALM, "api-key-client", ENV["ADMIN_USERNAME"], ENV["ADMIN_PASSWORD"], ENV["API_KEY_CLIENT_SECRET"]
    )["access_token"]


def hasura_accepts_admin_token(keycloak):
    result = Hasura(token=admin_token(keycloak)).execute("{ sequent_backend_tenant { id } }")
    if result.get("errors"):
        return None
    return [row["id"] for row in result["data"]["sequent_backend_tenant"]] == [TENANT_ID]


def configure_step_cli(cli):
    cli.step(
        "config",
        "--tenant-id", TENANT_ID,
        "--endpoint-url", ENV["HASURA_ENDPOINT"],
        "--keycloak-url", ENV["KEYCLOAK_URL"],
        "--keycloak-user", ENV["ADMIN_USERNAME"],
        "--keycloak-password", ENV["ADMIN_PASSWORD"],
        "--keycloak-client-id", "api-key-client",
        "--keycloak-client-secret", ENV["API_KEY_CLIENT_SECRET"],
    )  # fmt: skip


def trustee_public_key(name):
    config = ROOT / f".devcontainer/trustees-data/{name}/{name}.toml"
    return tomllib.loads(config.read_text())["signing_key_pk"]


def seed_trustees(cli):
    """Register the compose trustees by the signing keys their services use."""
    listed = cli.step("list-trustees")
    for name in TRUSTEES:
        if f"name={name} " not in listed:
            cli.step("create-trustee", "--name", name, "--public-key", trustee_public_key(name))


def main():
    OUTPUT.mkdir(parents=True, exist_ok=True)
    started = time.monotonic()
    timings = {}

    def mark(step):
        timings[step] = round(time.monotonic() - started, 1)
        print(f"bootstrap: {step} after {timings[step]}s", flush=True)

    keycloak = Keycloak()
    wait_until("the super tenant row", tenant_row, timeout=600, interval=3)
    mark("tenant row")
    kids = wait_until("the tenant realm signing keys", realm_signing_kids, timeout=300)
    wait_until("the tenant realm keys in the published JWKS", lambda: kids <= published_kids(), timeout=300)
    mark("JWKS published")
    enrolled = enroll_admin_mfa(keycloak)
    mark("administrator enrolled" if enrolled else "administrator already enrolled")
    try:
        wait_until("Hasura to accept an administrator token", lambda: hasura_accepts_admin_token(keycloak), timeout=90)
    except TimeoutError as error:
        print(f"bootstrap: {error}", flush=True)
        return HASURA_RESTART_NEEDED
    mark("Hasura accepts administrator tokens")
    cli = StepCli()
    configure_step_cli(cli)
    seed_trustees(cli)
    mark("trustees registered")
    (OUTPUT / "bootstrap.json").write_text(json.dumps({"seconds": timings, "admin_enrolled": enrolled}, indent=2))
    return 0
