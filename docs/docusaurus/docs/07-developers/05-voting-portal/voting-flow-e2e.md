---
id: voting-flow-e2e
title: Voting journey tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Chromium verifies the complete voter experience and can capture its browser requests alongside database activity. The [scalable voting runner](./voter-status-performance.md) works independently with either Chromium or k6; a capture is optional diagnostic evidence.

The shared UI test is `packages/voting-portal/test/load/flow.ts`. It fills configured login fields, chooses valid candidates, follows review and confirmation, and rejects demo voting. Each voter gets a fresh context; casts are never retried.

## What crosses the network

1. **Portal → portal host:** GET the login route, application bundle, WASM, icons and `global-settings.json`. Public event configuration is fetched from object storage.
2. **Portal → Keycloak:** GET `/realms/tenant-{tenant}-event-{event}/protocol/openid-connect/auth`; fetch the login theme resources; POST the current form action under `/login-actions/authenticate`. Keycloak redirects back with a fresh authorization code.
3. **Portal → Keycloak:** POST `/protocol/openid-connect/token` with that code and PKCE verifier; GET `/account` with the access token. The k6 protocol adapter generates state, nonce and verifier per journey and validates the callback and ID-token nonce. It does not replay a captured token or login form action.
4. **Portal → Hasura:** POST `/v1/graphql`, operation `GetVoterStatus`. It combines live cast status with `get_ballot_files_urls`. Hasura calls the authenticated Harvest action `/get-ballot-files-urls`; Harvest authorizes scope and signs publication references. This action does not download the objects.
5. **Portal → private S3:** GET `event.json`, `election-{id}.json` and `summary-{id}.json` for the list; GET `style-{id}.json` when opening the ballot. The event object is reused. k6 binds these requests to its own status response and checks event, election, style and publication version.
6. **Portal → Hasura:** POST `/v1/graphql`, operation `InsertCastVote`, with the encrypted ballot. The writer remains authoritative for eligibility, voting windows and revote policy. Diagnostic capture verifies the accepted receipt against PostgreSQL.

There is no separate `GetElections` request in this path. The list comes from published objects plus live status. Browser HAR describes browser-facing calls; SQL interval logs include background work and are not a per-request trace of every internal service call.

## Capture a diagnostic journey

Run in the devcontainer's `devenv shell`. First complete [election setup](./voter-status-performance.md#prepare-an-election-from-the-command-line), but do not run the load: the diagnostic capture casts as the first voter in that census. Do not subsequently use that same census range for a voting load.

The observer requires read access to both databases and their PostgreSQL JSON statement logs. The database servers must already have `log_destination = 'jsonlog'` and either `log_statement = 'all'` or `log_min_duration_statement = 0`; capture checks these settings but does not change them. Enter log paths visible **inside the devcontainer**, not paths that exist only inside the database container. This SQL diagnostic is optional; the regular runner does not require database access.

Create the private capture target from the setup configuration:

```bash
umask 077
read -rs -p 'Backend observer PostgreSQL DSN: ' VOTING_E2E_BACKEND_DSN
read -rs -p 'Keycloak observer PostgreSQL DSN: ' VOTING_E2E_KEYCLOAK_DSN
read -r -p 'Backend JSON log glob (for example /logs/backend/*.json): ' LOAD_BACKEND_LOGS
read -r -p 'Keycloak JSON log glob (for example /logs/keycloak/*.json): ' LOAD_KEYCLOAK_LOGS
export VOTING_E2E_BACKEND_DSN VOTING_E2E_KEYCLOAK_DSN
export LOAD_BACKEND_LOGS LOAD_KEYCLOAK_LOGS

python3 - <<'PYTHON'
import json
import os
from pathlib import Path

config = json.loads(Path(".cache/voting-scale/setup/config.json").read_text())
target = {
    "engine": "chromium",
    "expected_path": "s3",
    "mode": "journey",
    **{key: config[key] for key in (
        "login_url", "tenant_id", "election_event_id", "allowed_origins"
    )},
    "credentials": {
        "username": config["username_prefix"] + str(config["start"]),
        "password": os.environ[config["password_env"]],
    },
    "databases": {
        "backend": {
            "dsn_env": "VOTING_E2E_BACKEND_DSN",
            "jsonlog_glob": os.environ["LOAD_BACKEND_LOGS"],
        },
        "keycloak": {
            "dsn_env": "VOTING_E2E_KEYCLOAK_DSN",
            "jsonlog_glob": os.environ["LOAD_KEYCLOAK_LOGS"],
        },
    },
}
Path(".cache/voting-scale/capture-target.json").write_text(json.dumps(target, indent=2) + "\n")
PYTHON

python3 scripts/voting_e2e/capture.py \
  .cache/voting-scale/capture-target.json \
  .cache/voting-scale/capture
```

For a different census schema, the `credentials` object must contain that voter's actual login fields. `sql_clients` optionally maps database client IPs to service names; without it the report retains the IP addresses.

A successful capture automatically writes **`profile.json`**. No hand-maintained list of assets or GraphQL requests is needed. The compiler preserves request order, duplicates and start offsets, replaces login/cast steps with protocol adapters and replaces signed URLs with publication bindings. New unsupported POSTs, missing downloads and unverified journeys fail profile generation.

The profile currently supports one eligible election per voter and the standard Keycloak form/code flow, including the configured census fields. Additional authentication factors need explicit adapters. Refresh the capture after changing the portal, theme, assets or election configuration.

k6 is an HTTP simulation: it downloads the observed assets but does not render them or execute JavaScript/WASM. It issues requests in recorded order, with recorded start offsets as the default minimum pacing; it does not reproduce browser subresource concurrency, browser caching or CORS preflights. This optional capture replay is useful for comparing resource traffic. For browser-free preparation and bounded worker inputs, use the [scalable runner](./voter-status-performance.md).

HARs, query bindings, census files, tokens and encrypted ballots remain in private ignored directories. The aggregate report and generated charts contain no credentials or signed URLs.
