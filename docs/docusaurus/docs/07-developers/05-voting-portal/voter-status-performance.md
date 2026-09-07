---
id: voter-status-performance
title: GetVoterStatus and S3 E2E measurements
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

## Run the new path

Use a main portal and main Harvest with prepared S3 publication files and the `get_ballot_files_urls` Hasura query action. This release/10 E2E branch is the test driver; its own portal source does not implement the new main-only application path. The measured portal/backend source was main commit `5131ff46588bd46111628122ed2dc09b1a3f6010`, portal bundle SHA256 `7d838c9eee50fa7b59cafff91ad15e3367486d04117a41e63f4a43eeeb94d794`; existing cast services were release/10 B3.

Run inside the devcontainer, using `devenv shell`. Copy `scripts/voting_e2e/target.example.json` to a private ignored path; supply voter credentials, exact allowed origins, tenant/event scope, readable database JSON logs and environment-held DSNs. Set `expected_path: "s3"` to reject old GraphQL operations or missing event/election/summary/style downloads. Use fresh voters for casts; casts are never retried.

```sh
yarn --cwd packages/voting-portal typecheck:e2e
python3 -m unittest discover -s scripts/voting_e2e -p "test_*.py"
python3 scripts/voting_e2e/capture.py PRIVATE_TARGET PRIVATE_NEW_OUTPUT
python3 scripts/voting_e2e/cohort.py PRIVATE_TARGET PRIVATE_VOTERS_CSV PRIVATE_NEW_COHORT --count 5
```

For the read-only API benchmark set `mode: "status"`, `engine: "chromium"` and `performance: {"samples": 100, "warmup": 10, "concurrency": 4}`. An optional `max_p95_ms` fails the run when its local latency budget is exceeded. Authentication follows the real portal login; the benchmark reuses only the observed `GetVoterStatus` query and bearer token in memory. HTTP/GraphQL failures, missing scoped file references or inline EML fail validation. This mode never casts. Its requests use no automatic retries or redirects.

## Instrumentation

Browser captures record each request, GraphQL operation, timing, response size and status in private `capture.json`; `resource-profile.json.observed_traffic` preserves operation names even when HAR bodies are omitted. `journey.har` records authentication and resources. `backend-sql.json` and `keycloak-sql.json` retain actual PostgreSQL statements privately; `sql_summary` separates submitted SQL, transactions and nested plans. These SQL records cover the whole capture interval and log-drain wait, including authentication and possible background work, rather than asserting per-request attribution.

To observe the Hasura action hop, run the local diagnostic proxy and temporarily point **only** the local `get_ballot_files_urls` action handler to it:

```sh
python3 scripts/voting_e2e/proxy.py --upstream http://127.0.0.1:3031 --port 3032 --log PRIVATE_LOG
```

Set `action_proxy_log` in the target to that absolute log path. The runner copies newly observed calls into `action-http.json`. The proxy records method, fixed endpoint, status, elapsed time and response size; it never logs headers, tokens or bodies. Restore the action handler after capture. Proxy latency and full SQL logging are included in these diagnostic measurements. The proxy observes this action hop only, not every service-to-service call. Browser traffic captures Hasura, Keycloak, portal and S3 endpoints.

Raw targets, HAR, SQL and signed URLs remain private. Publish only reviewed aggregate results.

The Python regression suite (11 tests), dedicated E2E TypeScript check and five real S3 journeys pass. A deliberate zero-millisecond p95 budget correctly fails the API benchmark after three valid requests; it records no casts.

## Local results

Measured 2026-09-07 UTC in the existing aarch64 devcontainer with Chromium 144.0.7559.132, warm application services, one event/election/contest/area. No old SQL benchmark was rerun.

API samples exclude authentication, ten sequential warm-ups and S3 downloads. Each concurrency run uses one authenticated voter and closed-loop workers; it does not establish multi-voter production capacity.

| Concurrency | Samples | Errors | p50 ms | p95 ms | p99 ms | Requests/s |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 100 | 0 | 19.63 | 20.65 | 21.09 | 49.58 |
| 4 | 100 | 0 | 25.76 | 36.40 | 58.20 | 140.89 |

An exploratory concurrency-4 run overlapped synthetic-voter import (p95 46.25 ms, p99 152.13 ms); the table uses the subsequent isolated run. This disclosure avoids treating that contaminated run as comparable.

Each API run produced **111 successful action proxy calls**: one initial portal query, ten warm-ups and 100 measured requests. The matching main-Harvest database connection submitted **222 SELECTs**, two per action: (1) scoped active ballot-style/publication references joined to live election status/policy, and (2) event status. Hasura separately reads the scoped cast-vote status. URL signing does not download S3 ballot content.

Full journeys: **5/5 passed**, each matched the UI/API ballot receipt to its stored database record. p50 2511.0 ms; p95 2908.0 ms; p99 2986.4 ms. Each had one `GetVoterStatus`, one `InsertCastVote`, four private publication GETs and 54 total browser requests. The proxy recorded five successful `/get-ballot-files-urls` calls. At n=5, tail percentiles are descriptive interpolations.

The exact portal query is:

```graphql
query GetVoterStatus($electionEventId: String!) {
  get_ballot_files_urls(election_event_id: $electionEventId)
  sequent_backend_cast_vote { id tenant_id election_id election_event_id status }
}
```

### All observed browser endpoints (five journeys)

Dynamic UUIDs are replaced with `ID`; query strings and headers are omitted. Repeated requests remain counted.

| Host | Method | Path / operation | Count | Status counts |
|---|---|---|---:|---|
| 127.0.0.1 | GET | `/tenant/ID/event/ID/login` | 10 | `{'200': 10}` |
| 127.0.0.1 | GET | `/index.js` | 10 | `{'200': 10}` |
| 127.0.0.1 | GET | `/7f4f8e25342001a49610.wasm` | 10 | `{'200': 10}` |
| 127.0.0.1 | GET | `/favicon.svg` | 45 | `{'200': 45}` |
| 127.0.0.1 | GET | `/favicon-96x96.png` | 40 | `{'200': 40}` |
| 127.0.0.1 | GET | `/global-settings.json` | 10 | `{'200': 10}` |
| minio-proxy | GET | `/public/tenant-ID/event-ID/election_event_config.json` | 30 | `{'200': 30}` |
| keycloak | GET | `/realms/tenant-ID-event-ID/protocol/openid-connect/auth` | 5 | `{'200': 5}` |
| keycloak | GET | `/resources/c3r9m/common/keycloak/vendor/patternfly-v4/patternfly.min.css` | 5 | `{'200': 5}` |
| keycloak | GET | `/resources/c3r9m/common/keycloak/vendor/patternfly-v3/css/patternfly.min.css` | 5 | `{'200': 5}` |
| keycloak | GET | `/resources/c3r9m/common/keycloak/vendor/patternfly-v3/css/patternfly-additions.min.css` | 5 | `{'200': 5}` |
| keycloak | GET | `/resources/c3r9m/common/keycloak/lib/pficon/pficon.css` | 5 | `{'200': 5}` |
| keycloak | GET | `/resources/c3r9m/login/sequent.voting-portal/css/custom.css` | 5 | `{'200': 5}` |
| keycloak | GET | `/resources/c3r9m/login/sequent.voting-portal/css/login.css` | 5 | `{'200': 5}` |
| keycloak | GET | `/resources/c3r9m/login/sequent.voting-portal/js/menu-button-links.js` | 5 | `{'200': 5}` |
| keycloak | GET | `/resources/c3r9m/login/sequent.voting-portal/js/structured-credential.js` | 5 | `{'200': 5}` |
| keycloak | GET | `/resources/c3r9m/login/sequent.voting-portal/js/authChecker.js` | 5 | `{'200': 5}` |
| keycloak | GET | `/resources/c3r9m/common/keycloak/vendor/patternfly-v3/fonts/OpenSans-Light-webfont.woff2` | 5 | `{'200': 5}` |
| keycloak | GET | `/resources/c3r9m/common/keycloak/vendor/patternfly-v3/fonts/OpenSans-Bold-webfont.woff2` | 5 | `{'200': 5}` |
| keycloak | GET | `/resources/c3r9m/common/keycloak/vendor/patternfly-v3/fonts/OpenSans-Regular-webfont.woff2` | 5 | `{'200': 5}` |
| keycloak | GET | `/resources/c3r9m/common/keycloak/vendor/patternfly-v3/fonts/fontawesome-webfont.woff2` | 5 | `{'200': 5}` |
| keycloak | POST | `/realms/tenant-ID-event-ID/login-actions/authenticate` | 5 | `{'302': 5}` |
| keycloak | POST | `/realms/tenant-ID-event-ID/protocol/openid-connect/token` | 5 | `{'200': 5}` |
| keycloak | GET | `/realms/tenant-ID-event-ID/account` | 5 | `{'200': 5}` |
| graphql-engine | POST | `GetVoterStatus` | 5 | `{'200': 5}` |
| minio-proxy | GET | `/election-event-documents/tenant-ID/event-ID/publication-ID/ID/event.json` | 5 | `{'200': 5}` |
| minio-proxy | GET | `/election-event-documents/tenant-ID/event-ID/publication-ID/ID/election-ID.json` | 5 | `{'200': 5}` |
| minio-proxy | GET | `/election-event-documents/tenant-ID/event-ID/publication-ID/ID/summary-ID.json` | 5 | `{'200': 5}` |
| minio-proxy | GET | `/election-event-documents/tenant-ID/event-ID/publication-ID/ID/style-ID.json` | 5 | `{'200': 5}` |
| graphql-engine | POST | `InsertCastVote` | 5 | `{'200': 5}` |

### Observed SQL during the five journey intervals

Counts include background SQL; client-IP service attribution is not request correlation. Main Harvest shares the devcontainer IP. Actual statements and nested plans are retained in each private database SQL artifact.

| Database | Client | Category | Verb | Count |
|---|---|---|---|---:|
| backend | harvest | empty protocol check | EMPTY | 11 |
| backend | harvest | nested SQL plan | PLAN | 30 |
| backend | harvest | submitted SQL | INSERT | 5 |
| backend | harvest | submitted SQL | SELECT | 21 |
| backend | harvest | transaction | COMMIT | 5 |
| backend | harvest | transaction | ROLLBACK | 1 |
| backend | harvest | transaction | START | 6 |
| backend | hasura | submitted SQL | SELECT | 29 |
| backend | hasura | submitted SQL | WITH | 6 |
| backend | hasura | transaction | BEGIN | 27 |
| backend | hasura | transaction | COMMIT | 27 |
| backend | main-harvest (shared devcontainer IP) | empty protocol check | EMPTY | 5 |
| backend | main-harvest (shared devcontainer IP) | submitted SQL | SELECT | 10 |
| backend | main-harvest (shared devcontainer IP) | transaction | COMMIT | 5 |
| backend | main-harvest (shared devcontainer IP) | transaction | START | 5 |
| backend | windmill | empty protocol check | EMPTY | 13 |
| backend | windmill | submitted SQL | SELECT | 40 |
| backend | windmill | transaction | COMMIT | 8 |
| backend | windmill | transaction | ROLLBACK | 5 |
| backend | windmill | transaction | START | 13 |
| keycloak | harvest | empty protocol check | EMPTY | 5 |
| keycloak | keycloak | submitted SQL | DELETE | 2 |
| keycloak | keycloak | submitted SQL | INSERT | 20 |
| keycloak | keycloak | submitted SQL | SELECT | 109 |
| keycloak | keycloak | submitted SQL | UPDATE | 15 |
| keycloak | keycloak | transaction | BEGIN | 54 |
| keycloak | keycloak | transaction | COMMIT | 54 |
| keycloak | windmill | empty protocol check | EMPTY | 5 |
| keycloak | windmill | submitted SQL | SELECT | 10 |
| keycloak | windmill | transaction | ROLLBACK | 5 |
| keycloak | windmill | transaction | START | 5 |
