---
id: voting-flow-e2e
title: Capturing the full voting journey
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

The diagnostic harness drives the real voting portal with Playwright connected to
Obscura or an explicitly selected Chromium control, records HTTP requests, and reconciles API-accepted ballots
against PostgreSQL. It complements the SQL-only measurements in
[Voting flow performance](./voting-flow-performance.md).

## Run inside devenv

Install the workspace's locked JavaScript dependencies, then install the pinned
normal-render browser. The installer verifies its archive checksum and supports
Linux aarch64 and x86_64.

```sh
devenv shell python3 scripts/voting_e2e/install_obscura.py
.cache/obscura/obscura serve --port 9222 --allow-private-network
```

Keep Obscura running in that terminal. Private-network access allows connections
to local application services. Stealth and resource blocking remain disabled.

For a browser-only compatibility check, set `CAPTURE_OUTPUT_DIR` to an absolute
ignored directory and `OBSCURA_CDP_URL=http://127.0.0.1:9222`, then run in the voting
portal workspace:

```sh
node ../node_modules/@playwright/test/cli.js test --config playwright.capture.config.ts obscura.spec.ts
```

The capture runner resolves the CLI from `@playwright/test`; the generic
`playwright` executable can resolve to Nightwatch's different version in this
workspace.

Obscura 0.2.2 also aliases several HTML element constructors to `Element`, making
`head instanceof HTMLIFrameElement` incorrectly true. A guarded init script
restores tag-specific type checks before application code runs; this fixes
style-loader startup without suppressing requests. The probe verifies the
regression. See [the upstream report](https://github.com/h4ckf0r0day/obscura/issues/817).

This tests selectors, basic WASM execution, WebCrypto availability, stylesheet
observation, cookie isolation and HAR output. It does not prove that production
ballot encryption, Keycloak login or a cast works.

## Capture an actual voter

Start the local portal, Keycloak, Hasura and application workers, and use step-cli
setup to create synthetic voters and a published, open ONLINE event with a real
key ceremony. Copy `scripts/voting_e2e/target.example.json` into the ignored
`.cache/voting-e2e/` directory and replace its event IDs and credentials. Configure
the two DSN environment variables named in that file.

The optional `.devcontainer/docker-compose-e2e.yml` overlay mounts both log
volumes into devenv and enables full statement logging plus nested `auto_explain`
plans. It preserves the existing shared preload configuration. Use it only for
local diagnostic runs; these settings add measurement overhead. From a configured
devenv with Docker access, use the overlay when starting the stack:

```sh
docker-compose --env-file .devcontainer/.env \
  -f .devcontainer/docker-compose.yml -f .devcontainer/docker-compose-e2e.yml \
  --profile base up -d
```

The existing devcontainer must be recreated with those read-only log mounts
before capture. Set the release branch's B3 environment variables as well as the
application/database variables; a main-branch B4 environment file is insufficient.

Both PostgreSQL instances need JSON logging with `log_statement=all` or
`log_min_duration_statement=0`. Mount their JSON log directories readably inside
devenv and set the target's two `jsonlog_glob` paths. The observer checks these
settings but does not change them on a deployed cluster. Its own queries use
`application_name=voting_e2e_observer` and are excluded from captured records.

```sh
devenv shell python3 scripts/voting_e2e/capture.py \
  .cache/voting-e2e/target.json .cache/voting-e2e/run-001
```

Each run needs a fresh output directory. `--preflight-only` checks readiness
without attempting login or voting. The runner records readiness, collects new
JSON logs from both databases, and extracts `resource-profile.json` from the
successful HAR. It verifies every returned ballot's ID and tenant/event/election
scope against the stored vote. A displayed receipt alone is insufficient; demo
voting is rejected.

Raw HAR, SQL logs and target files can contain credentials and voter information.
Keep them in private local directories; they are never Docusaurus assets. Resource
profiles remove headers, bodies and query values, but paths may still contain
dynamic identifiers: review and bind those before sharing or replaying them.

The collector observes a database interval, which includes background
traffic. Nested plans were recorded in real voter journeys; complete per-request
attribution and exhaustive trigger/function coverage are not established. Do not interpret its record count as physical reads,
connection checkouts or a complete SQL statement count.

## What the API and SQL observations mean

The browser loads portal HTML, `index.js`, WASM, settings and the public
`election_event_config.json`, then follows Keycloak's authorization-code/PKCE
login. It submits the configured PIN/date-of-birth form, follows the redirect,
exchanges the authorization code for tokens and requests account information.
The local fixture uses local assets; an explicit `allowed_origins` list rejects
unexpected hosts. Playwright routing disables the HTTP cache in this diagnostic
mode, so these results describe fresh contexts with uncached resource requests.

| Browser operation | Purpose | PostgreSQL work observed in this run |
|---|---|---|
| Keycloak authorization, form submission and token exchange | Authenticate a fresh voter and create a session | `USER_ENTITY`, `USER_ATTRIBUTE`, `CREDENTIAL`, role/group mappings, session persistence and `EVENT_ENTITY` in the Keycloak database |
| `GetElectionEvent` | Load presentation and event status | Hasura reads `election_event` in the backend database |
| `GetElections` | Load elections permitted by the current voter's session | Hasura reads `election`; repeated requests remain in the profile |
| `GetBallotStyles` | Load eligible ballot definitions and signatures | Hasura reads `ballot_style`; these definitions feed client-side encryption |
| `GetCastVotes` | Load the voter's existing voting status | Hasura reads `cast_vote` under voter permissions |
| `InsertCastVote` | Submit the encrypted ballot through the real API | Harvest reads eligibility, signing material and the authoritative voting window, then inserts `cast_vote`; PostgreSQL also executes nested function/trigger work |

The GraphQL operations all use POST `/v1/graphql`. The cast assertion requires a
successful GraphQL result, a matching UI ballot receipt and a stored ballot with
the same tenant, event and election. HTTP 200 alone does not establish success.

Backend logs also contain Hasura `hdb_metadata` polling and Windmill activity.
Keycloak logs include worker traffic as well as authentication. The generated
SQL table separates services, submitted statements, transaction commands, nested
plans and empty protocol checks. `BEGIN`/`COMMIT` counts are not connection
checkouts; a pooled connection can execute many transactions. One SQL result row
is not one physical row read. This full-journey observation must not be compared
directly with the cast-only SQL benchmark's operation counts.

## Browser-free replication plan with Grafana k6

This is the implementation plan; a working k6 cast runner is not claimed yet.
Use the local k6 HTTP engine (`k6/http`), with no browser process. Python remains
responsible for fixture preparation, extraction, orchestration and comparison.

1. **Establish the devenv reference.** Keep the verified Playwright cohort,
   normalized HTTP/API inventory, both PostgreSQL logs and Docusaurus results.
   Run the browser compatibility gate before accepting an engine's HAR as a
   protocol reference. Obscura 0.2.2 currently fails method fidelity and storage
   persistence; Chromium is the measured reference until those checks pass.
2. **Replicate one complete voter with k6 in devenv.** Generate a recipe from
   HAR, including methods, normalized paths, query/form field names, operation
   names, dependency ordering, repeated resources and timings. Treat HAR
   conversion as scaffolding. Start each voter with a fresh cookie jar and
   fresh PKCE verifier/challenge, state and nonce. Parse the live Keycloak form
   action and hidden fields, submit that voter's configured credentials, extract
   the redirect code and exchange it for fresh tokens. Bind event/election IDs
   and GraphQL variables from current responses. Never reuse captured cookies,
   authorization codes, CSRF values, bearer tokens or encrypted ballots.
   Grafana documents [dynamic-value correlation](https://grafana.com/docs/k6/latest/examples/correlation-and-dynamic-data/)
   and [per-VU cookie jars](https://grafana.com/docs/k6/latest/using-k6/cookies/).
3. **Add IVR-like step-cli login and optional resource load.** Implement a CLI
   authentication mode using the configured IVR-style flow, plus selectable
   `minimal`, `ballot-list` and `browser-profile` resource modes. Reuse the
   script-generated recipe with k6 orchestration so updates to Playwright refresh
   the asset/API inventory. This is a distinct authentication workload: do not
   call its results equivalent to the browser's authorization-code login.
4. **Generate valid ballots without a browser.** Reuse the application's Rust
   ballot encryption code through step-cli to prepare unique ballots against
   the current published election keys. Allocate each artifact to exactly one
   voter/event/iteration. k6 submits the real `InsertCastVote` mutation and checks
   GraphQL errors and returned IDs; the observer verifies persistence afterward.
   Measure ballot preparation separately and preserve the browser-observed delay
   between data loading and casting when testing the matching load profile.
   An empty payload or a recorded ballot is never an acceptable substitute.
5. **Prove equivalence before increasing load.** Compare one voter, then a small
   distinct-voter cohort: method/path/operation counts, repeated requests, status
   distribution, ordering, bytes and phase timing; also compare per-service SQL
   statement shapes, transactions and nested work in both databases. Record an
   idle interval to identify background work. Require accepted and persisted
   ballots with no unexplained missing requests or extra writes. Document
   expected differences: no rendering/WASM CPU in k6, HTTP cache policy,
   compression, TLS, connection reuse, resource parallelism and think time.
6. **Scale the validated recipe.** Batch only independent resource requests with
   [k6 HTTP batching](https://grafana.com/docs/k6/latest/javascript-api/k6-http/batch/).
   Use [arrival-rate scenarios](https://grafana.com/docs/k6/latest/using-k6/scenarios/executors/constant-arrival-rate/)
   to schedule voter journeys independently of response time. Report offered
   journeys/s, accepted votes/s, failures, dropped iterations and p50/p95/p99;
   do not equate iteration starts with successful votes. Allocate disjoint voter
   and ballot ranges per environment, tenant, event and worker. Exercise multiple
   events across environments sharing the cluster before deploying multiple GCE
   workers. Keep explicit target origins, bounded run duration, worker ownership
   and run-owned cleanup. Publish local equivalence results before remote load
   results, and measure generator CPU/memory before claiming cost savings.

## Recorded browser compatibility limitation

The Obscura 0.2.2 probe on aarch64 passed DOM type checks with the guarded shim,
basic WASM, SHA-256, stylesheet observation, cookie isolation and HAR flushing.
However, its local HTTP server received **POST** while CDP/HAR recorded **GET**;
localStorage and sessionStorage were both empty after navigation. Transferred
body sizes were also invalid. The full voter attempt looped during authentication.
These are failed checks, not skipped checks or successful Obscura casts.
See the upstream [storage issue](https://github.com/h4ckf0r0day/obscura/issues/678).
The Chromium control below uses the actual application without the Obscura shim.

## Refresh optional CLI load from the browser flow

Extraction is scripted, so changing the Playwright flow and capturing it again
updates the resource recipe without manually maintaining an asset list:

```sh
devenv shell python3 scripts/voting_e2e/resources.py \
  .cache/voting-e2e/run-001/journey.har \
  .cache/voting-e2e/run-001/resource-profile.json
```

The recipe includes a non-executable protocol inventory with form field names,
API operations and fresh-session/ballot requirements, without captured values.
Its optional-resource list retains resource ordering, offsets, sizes, document/JS/CSS/image/font
fetches and named GraphQL queries, including ballot-list data queries when
present. It excludes casts and other mutations. Negative or missing HAR body-transfer
sizes become unknown values, rather than fabricated zero-byte requests; decoded
body size remains separate. Authentication state, query
values, GraphQL variables and signed document URLs need fresh session bindings.

The planned CLI stage adds an IVR-like login mode and optional resource groups:
minimal login, ballot-list/data requests, or the extracted browser resource
profile. Those are separate workload modes: minimal login does not reproduce the
browser's load. CLI login and resource replay are not implemented by this initial
capture harness.

## Generate results documentation

```sh
devenv shell python3 scripts/voting_e2e/report.py .cache/voting-e2e/run-001
```

The capture command also regenerates this section automatically, including on a
readiness failure. This replaces only the generated section below. Readiness
failures and synthetic browser checks are explicitly separated from verified voter journeys. A single
capture provides diagnostic timing; p50/p99 and votes/second comparisons require
multiple verified journeys and a defined measurement interval.

<!-- generated-e2e-results -->

## Recorded local results

Verified journeys: **10/10**.

Measured 2026-09-07: chromium 144.0.7559.132, aarch64, 8 logical CPUs, concurrency 1.

One event, one election, one contest, one area and distinct synthetic voters. Every sample uses a fresh browser context, warm application services, full SQL/auto_explain logging and no human think time. This is a diagnostic cohort, not a capacity benchmark or a before/after comparison.

HTTP cache: disabled by origin-guard routing. All API receipts were matched to the UI receipt and the stored ballot scope.

| Interval (ms) | Min | p50 | p95 | p99 | Max |
|---|---:|---:|---:|---:|---:|
| Full journey | 2299.0 | 2547.5 | 2922.6 | 2937.3 | 2941.0 |
| Portal to login form | 561.0 | 578.0 | 793.4 | 796.3 | 797.0 |
| Authentication and initial data | 837.0 | 1063.0 | 1340.5 | 1358.5 | 1363.0 |
| Ballot selection through confirmation | 719.0 | 752.0 | 769.0 | 773.0 | 774.0 |
| Cast API HTTP time | 18.0 | 19.0 | 21.2 | 21.2 | 21.2 |

**10 accepted votes / 25.795 measured journey seconds = 0.388 votes/s** for serial browser work. Including runner startup, log-drain waits and verification: 10 / 48.937 = 0.204 votes/s. Neither is sustainable cluster throughput. With n=10, p99 is an interpolation near the slowest sample, not a reliable production tail estimate.

![Recorded journey intervals](/img/voting-flow-e2e-latency.svg)

### Observed HTTP APIs and resource paths

Counts preserve repeated requests. GraphQL operation names below all use POST `/v1/graphql`; other rows show normalized paths. IDs, query values, credentials and response bodies are excluded.

| Service | Method | Operation or path | Total | Mean/voter | Status counts |
|---|---|---|---:|---:|---|
| Hasura | POST | `GetBallotStyles` | 10 | 1.0 | 200: 10 |
| Hasura | POST | `GetCastVotes` | 10 | 1.0 | 200: 10 |
| Hasura | POST | `GetElectionEvent` | 10 | 1.0 | 200: 10 |
| Hasura | POST | `GetElections` | 20 | 2.0 | 200: 20 |
| Hasura | POST | `InsertCastVote` | 10 | 1.0 | 200: 10 |
| Keycloak | GET | `/realms/tenant-{id}vent-{id}/account` | 10 | 1.0 | 200: 10 |
| Keycloak | GET | `/realms/tenant-{id}vent-{id}/protocol/openid-connect/auth` | 10 | 1.0 | 200: 10 |
| Keycloak | GET | `/resources/{version}/common/keycloak/lib/pficon/pficon.css` | 10 | 1.0 | 200: 10 |
| Keycloak | GET | `/resources/{version}/common/keycloak/vendor/patternfly-v3/css/patternfly-additions.min.css` | 10 | 1.0 | 200: 10 |
| Keycloak | GET | `/resources/{version}/common/keycloak/vendor/patternfly-v3/css/patternfly.min.css` | 10 | 1.0 | 200: 10 |
| Keycloak | GET | `/resources/{version}/common/keycloak/vendor/patternfly-v3/fonts/OpenSans-Bold-webfont.woff2` | 10 | 1.0 | 200: 10 |
| Keycloak | GET | `/resources/{version}/common/keycloak/vendor/patternfly-v3/fonts/OpenSans-Light-webfont.woff2` | 10 | 1.0 | 200: 10 |
| Keycloak | GET | `/resources/{version}/common/keycloak/vendor/patternfly-v3/fonts/OpenSans-Regular-webfont.woff2` | 10 | 1.0 | 200: 10 |
| Keycloak | GET | `/resources/{version}/common/keycloak/vendor/patternfly-v3/fonts/fontawesome-webfont.woff2` | 10 | 1.0 | 200: 10 |
| Keycloak | GET | `/resources/{version}/common/keycloak/vendor/patternfly-v4/patternfly.min.css` | 10 | 1.0 | 200: 10 |
| Keycloak | GET | `/resources/{version}/login/sequent.voting-portal/css/custom.css` | 10 | 1.0 | 200: 10 |
| Keycloak | GET | `/resources/{version}/login/sequent.voting-portal/css/login.css` | 10 | 1.0 | 200: 10 |
| Keycloak | GET | `/resources/{version}/login/sequent.voting-portal/js/authChecker.js` | 10 | 1.0 | 200: 10 |
| Keycloak | GET | `/resources/{version}/login/sequent.voting-portal/js/menu-button-links.js` | 10 | 1.0 | 200: 10 |
| Keycloak | GET | `/resources/{version}/login/sequent.voting-portal/js/structured-credential.js` | 10 | 1.0 | 200: 10 |
| Keycloak | POST | `/realms/tenant-{id}vent-{id}/login-actions/authenticate` | 10 | 1.0 | 302: 10 |
| Keycloak | POST | `/realms/tenant-{id}vent-{id}/protocol/openid-connect/token` | 10 | 1.0 | 200: 10 |
| portal | GET | `/favicon-96x96.png` | 80 | 8.0 | 200: 80 |
| portal | GET | `/favicon.svg` | 90 | 9.0 | 200: 90 |
| portal | GET | `/global-settings.json` | 20 | 2.0 | 200: 20 |
| portal | GET | `/index.js` | 20 | 2.0 | 200: 20 |
| portal | GET | `/tenant/{id}/event/{id}/login` | 20 | 2.0 | 200: 20 |
| portal | GET | `/{bundle}.wasm` | 20 | 2.0 | 200: 20 |
| public bucket | GET | `/public/tenant-{id}/event-{id}/election_event_config.json` | 60 | 6.0 | 200: 60 |

Total HTTP requests: **540**, mean **54.0/voter**.

### Observed PostgreSQL work

Each database is a separate PostgreSQL instance. Counts cover the capture interval plus a one-second log-drain wait. Client IP identifies the submitting service, not the originating HTTP request. Background worker and Hasura metadata activity remain included. Observer verification queries are excluded.

Submitted statements, transaction commands, nested plans and empty protocol checks are separate categories. Nested plans describe work inside functions/triggers and must not be added as independent client requests. These counts do not measure physical rows, connection checkouts or pool occupancy.

| Database | Service | Category | SQL verb | Total | Mean/voter | Min–max/voter |
|---|---|---|---|---:|---:|---:|
| backend | harvest | empty protocol check | EMPTY | 19 | 1.9 | 1–3 |
| backend | harvest | nested SQL plan | PLAN | 60 | 6.0 | 6–6 |
| backend | harvest | submitted SQL | INSERT | 10 | 1.0 | 1–1 |
| backend | harvest | submitted SQL | SELECT | 41 | 4.1 | 4–5 |
| backend | harvest | transaction | COMMIT | 10 | 1.0 | 1–1 |
| backend | harvest | transaction | ROLLBACK | 1 | 0.1 | 0–1 |
| backend | harvest | transaction | START | 11 | 1.1 | 1–2 |
| backend | hasura | submitted SQL | SELECT | 100 | 10.0 | 9–11 |
| backend | hasura | submitted SQL | SET | 3 | 0.3 | 0–3 |
| backend | hasura | submitted SQL | WITH | 8 | 0.8 | 0–2 |
| backend | hasura | transaction | BEGIN | 53 | 5.3 | 5–6 |
| backend | hasura | transaction | COMMIT | 53 | 5.3 | 5–6 |
| backend | windmill | empty protocol check | EMPTY | 22 | 2.2 | 0–5 |
| backend | windmill | submitted SQL | SELECT | 76 | 7.6 | 0–15 |
| backend | windmill | transaction | COMMIT | 13 | 1.3 | 0–3 |
| backend | windmill | transaction | ROLLBACK | 9 | 0.9 | 0–3 |
| backend | windmill | transaction | START | 22 | 2.2 | 0–5 |
| keycloak | harvest | empty protocol check | EMPTY | 8 | 0.8 | 0–1 |
| keycloak | keycloak | empty protocol check | EMPTY | 2 | 0.2 | 0–2 |
| keycloak | keycloak | submitted SQL | DELETE | 2 | 0.2 | 0–2 |
| keycloak | keycloak | submitted SQL | INSERT | 41 | 4.1 | 4–5 |
| keycloak | keycloak | submitted SQL | SELECT | 174 | 17.4 | 14–43 |
| keycloak | keycloak | submitted SQL | UPDATE | 14 | 1.4 | 1–3 |
| keycloak | keycloak | transaction | BEGIN | 72 | 7.2 | 4–33 |
| keycloak | keycloak | transaction | COMMIT | 72 | 7.2 | 4–33 |
| keycloak | windmill | empty protocol check | EMPTY | 9 | 0.9 | 0–1 |
| keycloak | windmill | submitted SQL | SELECT | 21 | 2.1 | 0–3 |
| keycloak | windmill | transaction | ROLLBACK | 9 | 0.9 | 0–1 |
| keycloak | windmill | transaction | START | 9 | 0.9 | 0–1 |

Raw SQL and HAR remain private and are regenerated by the scripts. The tables are observations of this cohort, not an assertion that every background statement was caused by a voter.
