---
title: Browser tests, coverage and load
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Step uses Playwright Test with pinned headless Chromium, the real Rust services,
Keycloak, Hasura, PostgreSQL, RabbitMQ, S3 storage and two native trustees. The
runner derives its service definitions from the maintained devcontainer Compose
files. It owns an isolated project, databases, fixture tenant and CLI session.
No existing deployment or developer stack is needed.

## Run locally

Use `.devcontainer/e2e/devcontainer.json`, or run the host entry point with Python 3
and Docker Compose/Buildx installed. Docker must be accessible; this machine can
use `E2E_DOCKER_SUDO=1`. The image pins Rust, LLVM coverage tools, Playwright and k6.

```sh
scripts/e2e images
scripts/e2e run --run-id local-smoke
scripts/e2e run --run-id local-coverage --coverage e2e
scripts/e2e run --run-id local-combined --coverage combined
```

Run IDs are unique. Never reuse an attempted election/voter after a failed or
ambiguous cast. Browser retries are disabled. `--keep` retains a local stack for
diagnosis; `scripts/e2e cleanup RUN_ID` removes only that stack and its data.
`--skip-build` is for artifacts already built in this checkout, not a substitute
for verifying a different revision. Two Rust build jobs, one browser worker and
finite synthetic cohorts are the defaults.

The runner creates a private `.e2e/runs/RUN_ID/` directory. `report/` contains
publishable summaries and coverage; `private/` contains service/build logs,
Playwright HTML, failure traces, synthetic credentials and provisioning evidence.
Do not upload the entire run directory. Inspect the first failed stage, then its
private log. Source builds, generated WASM and run data stay under ignored `.e2e/`.

## Test ownership and acceptance

| Location | Responsibility |
| --- | --- |
| `packages/<portal>/test/e2e/*.spec.ts` | Package-owned browser behavior |
| `packages/e2e/journeys/` | Journeys crossing portal boundaries |
| `packages/e2e/fixtures.ts`, `playwright.config.ts` | Browser lifecycle, coverage and one shared configuration |
| `packages/voting-portal/test/load/flow.ts` | Shared real login, selection, encryption and cast flow |
| `packages/step-cli/src/load/` | Provisioning, native encryption, shard ownership, exact aggregation and receipt audit |
| `packages/e2e/runner/` | Isolated infrastructure, remote orchestration, telemetry and reports |
| Native crate tests | Crypto, SQL and service permutations below the browser boundary |

Keep tests beside their package. A central `e2e/` directory does not replace those
folders. Prefer role/name locators and readiness polling; use API/CLI setup for
unrelated state. A real cast must pass encryption, reject demo mode, return no
GraphQL errors, match its UI/API identifiers and exist in PostgreSQL through a
read-only audit connection. The audit journey downloads a fresh auditable ballot.

`packages/e2e/scenarios.yaml` records current ownership and migration status.
Existing Nightwatch admin management/login configuration matrices and Loadero
enrollment/report scenarios remain until equivalent coverage is verified. The
old E2E workflow's unrestricted command string is replaced by typed workflows.
The native k6/Chromium load stack from meta issue 12767 is preserved. Do not remove
Loadero merely because voting load works: enrollment/report and any used region
or network-shaping behavior must have replacements first.

Quick maintenance checks, from the repository root:

```sh
PYTHONPATH=packages/e2e python3 -m unittest discover -s packages/e2e/tests_runner
cd packages
yarn workspace @sequentech/e2e typecheck
yarn workspace @sequentech/e2e test --list
```

## CI and wall time

`E2E Tests` (`e2e.yml`) runs contracts for changes to the harness and exposes manual
full-stack runs with `none`, `e2e` or `combined` coverage. It caches tool/Keycloak
image layers and separate native build trees, bounds the job and always attempts
cleanup/reporting. Full-stack runs currently have a 90-minute cold-build ceiling;
the browser phase has a 12-minute ceiling. These are ceilings, not performance
claims. Record image preparation plus runner timings from an actual Actions run
before making this a required PR gate. The intended warm smoke budget is 15
minutes; coverage and broader lifecycle tests belong in a separate lane if they
exceed it. A contracts-only green run does not validate the application.

The Actions job summary includes stage timings and coverage Markdown. HTML and
machine-readable coverage are retained for 14 days. Missing reports and missing
profiles fail visibly. Build instrumentation is separate from normal load images.

## Coverage

Frontend webpack instrumentation produces Istanbul data across navigation and
test teardown. Reporting includes an unvisited-source baseline. Unit and webpack
transforms can assign different statement IDs, so the combined report unions
remapped executable source lines. It never adds duplicate line counts or averages
percentages. It does not claim combined branch/function coverage.

Native services use Rust/LLVM instrumentation and continuous profiles with counter
relocation, verified on the pinned Linux toolchain. Each process/service writes a
distinct profile. Reporting requires profiles from Harvest, Windmill, Beat and B4,
keeps the matching binaries, checks the revision/local patch digest, and uses the
compiler's own `llvm-profdata`/`llvm-cov` tools. Missing profiles fail collection.

The Markdown separates frontend and native coverage. Frontend scope excludes
generated GraphQL, translations, stories, mocks and tests. Native scope is linked
workspace code in the selected service/CLI binaries. WASM, Java, SQL, dependencies
and unlinked crates are outside these denominators. Combined mode currently adds
voting-portal unit coverage and sequent-core/step-cli native unit binaries; extend
other unit suites explicitly. All inputs come from the same checkout/build.

## Load testing and live dashboards

Use native `step-cli load` for direct operator workflows; `load reference` describes
its configuration. `scripts/e2e load --target NAME --engine k6` adds registered
target limits, live telemetry and cleanup. Use `--engine chromium` for actual
rendering, WASM encryption and UI interaction. Protocol and browser measurements
describe different work and should be compared with that distinction intact.

The manual `Designated tenant load test` workflow provisions once, distributes
immutable voter shards to one to four GitHub runners, aggregates all results,
reconciles receipts and cleans the owned election. Workers receive synthetic
inputs/passwords and telemetry credentials; administrator sessions stay with
preparation/cleanup. The default is eight voters, one runner and one concurrent
voter. Larger presets still obey the target's registered caps. There are no
automatic cast retries. Incomplete workers, failed goals, telemetry loss and
cleanup failure prevent a passing result. k6 also aborts sustained high errors.

Import `packages/e2e/monitoring/grafana.json` into an existing Grafana and choose its
Prometheus datasource. Both engines publish live completed/failed/accepted counts
and fixed-bucket journey latency histograms to an authenticated Pushgateway.
Scrape it every **five seconds**, with `honor_labels: true`. Grafana sums histogram
buckets before calculating aggregate quantiles. Run groups are removed after the
final scrapes; configure normal Prometheus retention for their history. Native
end-of-run reports retain exact quantiles and database-audit results.

k6 additionally exports each worker's HTML dashboard. For a direct single-worker
local run, `K6_WEB_DASHBOARD_PORT=5665` enables its live localhost dashboard;
publish/forward that port deliberately. Do not share a port across concurrent
shards. Actions uses port `-1` and the shared Grafana view, because a runner's
localhost is not a public dashboard. The exported native report remains useful
for very short runs with few dashboard samples.

## Recurring deployments: activation deliberately pending

No real environment is enabled. `packages/e2e/targets.json` is empty. Beyond's
`k8s/charts/kubernetes-app/examples/synthetic-checks.yaml` is a disabled template.
Choose **one** scheduler per target: its Kubernetes CronJob or the scheduled
GitHub workflow. The default cadence is 00:17/12:17 UTC, with missed-run grace and
no overlapping run per scheduler. Enabling both schedulers would duplicate probes.

Activation requires an explicit synthetic tenant, HTTPS service/storage URLs,
an immutable compatible runner image digest, least-privilege synthetic operator
credentials, a read-only receipt-audit DSN, and a reachable authenticated
Pushgateway. The tenant must have functioning trustees and the normal application
dependencies. Supply secret references; never put credentials in the registry.
Load permission is separately disabled and has voter/runner caps.

Build the deployable runner after an accepted normal run:

```sh
docker build -f .devcontainer/e2e/Dockerfile.runner -t REGISTRY/step-e2e:REVISION .
```

Publish it through the authorized release process, then configure the resulting
`@sha256:...` digest. GitHub uses environment `synthetic-NAME`, variable
`E2E_RUNNER_IMAGE`, and secrets `E2E_ADMIN_USERNAME`, `E2E_ADMIN_PASSWORD`,
`E2E_CLIENT_ID`, `E2E_CLIENT_SECRET`, `E2E_VOTER_PASSWORD`, `E2E_AUDIT_DSN`,
`E2E_PUSHGATEWAY_URL`, and optional `E2E_METRICS_TOKEN`. Beyond injects the same names
from `syntheticChecks.secretName`. Pin a test image compatible with the deployed
application version, not an unrelated latest main build.

Remote probes create an owned synthetic election, cast with fresh voters, audit
the receipt and delete that election. They never migrate/reset target databases.
Preparation records ownership before ceremony/census so partial setup can be
cleaned. Cancellation can interrupt cleanup: the stuck/cleanup alerts make that
visible, and the private ownership record identifies the exact election to clean.

Stable Prometheus status groups track last start, finish, success, latest result,
active state and cleanup. Starting a new run preserves the previous failure;
failure preserves the last success. Beyond renders expected-target inventory and
alerts for failed, missing/never-run, overdue, stuck and uncleaned probes, plus
failed/stuck loads. Prometheus evaluates these rules and sends alerts to the
existing Alertmanager route. Set `prometheusRuleLabels` to match its rule selector.
Success resolves the failure alert. Load tests never blanket-silence availability
alerts. Telemetry delivery failures fail the workflow; missing-run alerts provide
an independent check on the monitor itself.
