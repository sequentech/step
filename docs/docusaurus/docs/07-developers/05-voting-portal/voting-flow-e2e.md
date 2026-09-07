---
id: voting-flow-e2e
title: Voting journey tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Chromium proves the complete voter experience. k6 replays the HTTP workload derived from that same test, using prepared census credentials and encrypted ballots. Both must produce receipts that match PostgreSQL.

## One journey, two runners

| Step | Chromium with Playwright | k6 |
|---|---|---|
| Open portal | Loads HTML, JavaScript, WASM and configuration | Fetches the resources observed in the Chromium capture |
| Log in | Fills the event's Keycloak form | Uses a fresh cookie jar, PKCE challenge and the prepared voter's credentials |
| List elections | Renders the eligible election list | Executes the captured status query and validates the returned publication |
| Open ballot | Downloads the selected style and renders contests | Downloads the corresponding objects using fresh signed URLs |
| Prepare choice | Selects candidates and encrypts in browser WASM | Uses the assigned ciphertext prepared before load |
| Cast | Clicks Cast and reads the confirmation receipt | Submits the assigned mutation and validates the receipt |
| Verify | Matches UI receipt, API response and database row | Matches API response, assigned ballot and database row |

The shared UI test is `packages/voting-portal/test/load/flow.ts`. It waits for the actual login form, fills configured census fields, chooses valid candidates, accepts a required declaration, follows review/confirmation and rejects demo voting. Each journey starts in a fresh browser context; casts are never retried.

## What crosses the network

1. **Portal → portal host:** GET the login route, application bundle, WASM, icons and `global-settings.json`. Public event configuration is fetched from object storage.
2. **Portal → Keycloak:** GET `/realms/tenant-{tenant}-event-{event}/protocol/openid-connect/auth`; fetch the login theme resources; POST the current form action under `/login-actions/authenticate`. Keycloak redirects back with a fresh authorization code.
3. **Portal → Keycloak:** POST `/protocol/openid-connect/token` with that code and PKCE verifier; GET `/account` with the access token. k6 generates state, nonce and verifier per journey and validates the callback and ID-token nonce. It does not replay a captured token or login form action.
4. **Portal → Hasura:** POST `/v1/graphql`, operation `GetVoterStatus`. It combines live cast status with `get_ballot_files_urls`. Hasura calls the authenticated Harvest action `/get-ballot-files-urls`; Harvest authorizes scope and signs publication references. This action does not download the objects.
5. **Portal → private S3:** GET `event.json`, `election-{id}.json` and `summary-{id}.json` for the list; GET `style-{id}.json` when opening the ballot. The event object is reused. k6 binds these requests to its own status response and checks event, election, style and publication version.
6. **Portal → Hasura:** POST `/v1/graphql`, operation `InsertCastVote`, with the encrypted ballot. The writer remains authoritative for eligibility, voting windows and revote policy. The harness verifies the accepted receipt against PostgreSQL.

There is no separate `GetElections` request in this path. The list comes from published objects plus live status. Browser HAR describes browser-facing calls; SQL interval logs include background work and are not a per-request trace of every internal service call.

## Capture and derive the profile

Run in the devcontainer's `devenv shell`, from the repository containing the E2E harness. Provision an active, published event and an unused synthetic voter first. The target JSON supplies `engine: "chromium"`, `expected_path: "s3"`, the portal login URL, credentials, tenant/event IDs, explicit allowed origins, and observer database configuration. DSNs are supplied through the environment variables named by `databases.*.dsn_env`.

`databases.backend` and `databases.keycloak` each require `dsn_env` and `jsonlog_glob`. The observer needs readable PostgreSQL JSON logs with statement logging enabled. `sql_clients` optionally maps client IPs to service names. Keep the target inside an ignored directory; `credentials` contains the selected census row, including its password and configured match fields.

```sh
python3 scripts/voting_e2e/capture.py PRIVATE_TARGET PRIVATE_CAPTURE
```

A successful capture automatically writes **`profile.json`**. No hand-maintained list of assets or GraphQL requests is needed. The compiler preserves request order, duplicates and start offsets, replaces login/cast steps with protocol adapters and replaces signed URLs with publication bindings. New unsupported POSTs, missing downloads and unverified journeys fail profile generation.

The profile currently supports one eligible election per voter and the standard Keycloak form/code flow, including the configured census fields. Additional authentication factors need explicit adapters. Refresh the capture after changing the portal, theme, assets or election configuration.

k6 is an HTTP simulation: it downloads the observed assets but does not render them or execute JavaScript/WASM. It issues requests in recorded order, with recorded start offsets as the default minimum pacing; it does not reproduce browser subresource concurrency, browser caching or CORS preflights. Use Chromium for UI correctness and k6 for [repeatable distributed load](./prepared-vote-load.md).

HARs, query bindings, census files, tokens and encrypted ballots remain in private ignored directories. The aggregate report and generated charts contain no credentials or signed URLs.
