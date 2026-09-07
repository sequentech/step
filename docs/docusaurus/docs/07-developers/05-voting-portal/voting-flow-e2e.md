---
id: voting-flow-e2e
title: Capturing the full voting journey
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

The diagnostic harness drives the real voting portal with Playwright connected to
Obscura, records successful HTTP requests, and reconciles API-accepted ballots
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
yarn playwright test --config playwright.capture.config.ts obscura.spec.ts
```

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

The initial collector observes a database interval, which includes background
traffic. The overlay captures nested SQL plans, but the collector does not yet establish
per-request attribution or prove complete trigger/function coverage for a real
voter journey. Do not interpret its record count as physical reads,
connection checkouts or a complete SQL statement count.

## Refresh optional CLI load from the browser flow

Extraction is scripted, so changing the Playwright flow and capturing it again
updates the resource recipe without manually maintaining an asset list:

```sh
devenv shell python3 scripts/voting_e2e/resources.py \
  .cache/voting-e2e/run-001/journey.har \
  .cache/voting-e2e/run-001/resource-profile.json
```

The recipe retains resource ordering, offsets, sizes, document/JS/CSS/image/font
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
readiness failure. This replaces only the generated section below. Readiness failures and synthetic
browser checks are explicitly separated from verified voter journeys. A single
capture provides diagnostic timing; p50/p99 and votes/second comparisons require
multiple verified journeys and a defined measurement interval.

<!-- generated-e2e-results -->

## Recorded local results

Report generated: 2026-09-07.

Obscura 0.2.2, Playwright 1.62.1, arm64 synthetic compatibility check (2026-09-07T15:31:32.932Z):

| Check | Result |
|---|---|
| selector | Passed |
| wasm | Passed |
| webCrypto | Passed |
| stylesheetObserved | Passed |
| contextIsolation | Passed |
| harFlushed | Passed |
| Coverage: harBodySizesValid | Unavailable |

Probe interval: 130 ms. This is one synthetic browser check, not login-to-cast latency or a throughput benchmark.

| Capture prerequisite | Ready |
|---|---|
| portal | No |
| obscura | Yes |
| backend | No |
| keycloak | No |

**No real login-to-cast result is recorded.** Provision and start the application stack, then complete capture and persistence verification.
