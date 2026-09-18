# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Local fixture preparation, executed inside the isolated runner only."""
import json
import os
import subprocess
import time
import tomllib
from pathlib import Path
import requests
import yaml
from .process import ROOT, execute, save

TENANT = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5"
ARTIFACTS = Path(os.environ.get("E2E_ARTIFACTS", "/tmp/e2e"))
HEADERS = {"x-hasura-admin-secret": "admin"}


def wait_http(url, timeout=240):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            response = requests.get(url, timeout=5)
            if response.ok:
                return response
        except requests.RequestException:
            pass
        time.sleep(2)
    raise RuntimeError(f"Service readiness deadline exceeded: {url}")


def sql(statement=None, file=None):
    command = ["psql", "-h", "postgres", "-U", "postgres", "-d", "postgres", "-v", "ON_ERROR_STOP=1"]
    command += ["--single-transaction", "-f", str(file)] if file else ["-c", statement]
    execute(command, env={"PGPASSWORD": "postgrespassword"},
            log=ARTIFACTS / "private/bootstrap.log", timeout=120)


def migrate():
    # This entry point is only called against a freshly owned Compose database.
    for _ in range(60):
        if subprocess.run(["pg_isready", "-h", "postgres", "-U", "postgres"],
                          stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0:
            break
        time.sleep(2)
    else:
        raise RuntimeError("Postgres readiness deadline exceeded")
    for migration in sorted((ROOT / "hasura/migrations/backend-db").glob("*/up.sql")):
        sql(file=migration)
    sql(f"INSERT INTO sequent_backend.tenant (id,slug,settings) VALUES ('{TENANT}','e2e','{{}}');")
    # Only synthetic receipts are visible to the independent browser auditor.
    sql("CREATE ROLE e2e_audit LOGIN PASSWORD 'e2e-audit'; GRANT CONNECT ON DATABASE postgres TO e2e_audit; "
        "GRANT USAGE ON SCHEMA sequent_backend TO e2e_audit; "
        "GRANT SELECT ON sequent_backend.cast_vote TO e2e_audit; "
        "ALTER ROLE e2e_audit SET default_transaction_read_only=on;")


def metadata():
    wait_http("http://graphql-engine:8080/healthz")
    execute(["hasura", "metadata", "apply", "--project", "hasura", "--endpoint",
             "http://graphql-engine:8080", "--admin-secret", "admin", "--disallow-inconsistent-metadata"],
            log=ARTIFACTS / "private/bootstrap.log")
    response = requests.post("http://graphql-engine:8080/v1/metadata", headers=HEADERS,
                             json={"type": "get_inconsistent_metadata", "args": {}}, timeout=30)
    response.raise_for_status()
    if not response.json().get("is_consistent"):
        raise RuntimeError("Inconsistent Hasura metadata")


def jwks():
    response = wait_http(f"http://keycloak:8090/realms/tenant-{TENANT}/protocol/openid-connect/certs")
    save(ARTIFACTS / "jwks.json", response.json())
    client_origins(f"tenant-{TENANT}", "admin-portal", ["http://portals:3002"])


def client_origins(realm, client_id, origins):
    """Allow only the isolated fixture's real portal origins for browser OIDC."""
    token = requests.post("http://keycloak:8090/realms/master/protocol/openid-connect/token",
        data={"client_id": "admin-cli", "username": "admin", "password": "admin", "grant_type": "password"}, timeout=30)
    token.raise_for_status()
    headers = {"Authorization": "Bearer " + token.json()["access_token"]}
    url = f"http://keycloak:8090/admin/realms/{realm}/clients"
    clients = requests.get(url, params={"clientId": client_id}, headers=headers, timeout=30)
    clients.raise_for_status()
    client = clients.json()[0]
    client.update(rootUrl=origins[0], baseUrl=origins[0], webOrigins=origins,
                  redirectUris=[origin + "/*" for origin in origins])
    response = requests.put(url + "/" + client["id"], headers=headers, json=client, timeout=30)
    response.raise_for_status()


def prepare(engine="chromium", count=8):
    mode = "normal" if os.environ.get("E2E_COVERAGE", "none") == "none" else "coverage"
    cli = str(ROOT / ".e2e/bin" / mode / "step-cli")
    log = ARTIFACTS / "private/prepare.log"
    realm = json.loads((ROOT / f".devcontainer/keycloak/import/tenant-{TENANT}.json").read_text())
    client = next(c for c in realm["clients"] if c["clientId"] == "api-key-client")
    execute([cli, "step", "config", "--tenant-id", TENANT, "--endpoint-url",
             "http://graphql-engine:8080/v1/graphql", "--keycloak-url", "http://keycloak:8090",
             "--keycloak-user", "admin", "--keycloak-password", "admin", "--keycloak-client-id",
             "api-key-client", "--keycloak-client-secret", client["secret"]], log=log)
    if not list(Path(os.environ["STEP_CLI_CONFIG_DIR"]).glob("*")):
        raise RuntimeError("CLI authentication did not create an isolated session")
    for name in ("trustee1", "trustee2"):
        config = tomllib.loads((ROOT / f".devcontainer/trustees-data/{name}/{name}.toml").read_text())
        execute([cli, "step", "create-trustee", "--name", name, "--public-key", config["signing_key_pk"]], log=log)
    config_path = ARTIFACTS / "private/workload.yaml"
    execute([cli, "load", "init", "--output", str(config_path)], log=log)
    config = yaml.safe_load(config_path.read_text())
    config["target"].update(portal_url="http://portals:3000", storage_origins=["http://minio-proxy:9002"], upload_mode="direct")
    config["workload"].update(engine=engine, count=count, concurrency=1, shard_size=count,
                              username_prefix="e2e-", max_duration="5m", journey_timeout_ms=120000)
    config["execution"].update(workers=1, executor="local")
    config["runtime"].update(playwright_dir=str(ROOT / "packages/voting-portal"), chromium=None)
    config_path.write_text(yaml.safe_dump(config))
    execute([cli, "load", "prepare", str(config_path), "--output", str(ARTIFACTS / "private/prepared")],
            log=log, env={"LOAD_PASSWORD": "E2e-synthetic-2026!"}, timeout=900)
    prepared = json.loads((ARTIFACTS / "private/prepared/inputs/config.json").read_text())
    client_origins(prepared["realm"], "voting-portal", ["http://portals:3000", "http://portals:3001"])
    save(ARTIFACTS / "private/fixture.json", {
        "tenantId": TENANT, "eventId": prepared["election_event_id"],
        "electionId": prepared["election_id"], "loginUrl": prepared["login_url"],
        "adminUrl": "http://portals:3002",
        "verifierUrl": f"http://portals:3001/tenant/{TENANT}/event/{prepared['election_event_id']}/login", "resultsUrl": "http://portals:3004",
        "usernamePrefix": "e2e-", "password": "E2e-synthetic-2026!",
        "adminUsername": "admin", "adminPassword": "admin",
        "auditDsn": "postgres://e2e_audit:e2e-audit@postgres:5432/postgres"})


if __name__ == "__main__":
    import sys
    {"migrate": migrate, "metadata": metadata, "jwks": jwks, "prepare": prepare}[sys.argv[1]]()
