---
sidebar_position: 7
title: Voting load tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Measure real voting journeys with **k6** or **Chromium** through `step-cli load`. Both authenticate distinct synthetic voters, read voter status, download published ballots from S3, and submit votes through the normal API.

| Engine | Measures | Preparation |
| --- | --- | --- |
| k6 | Authenticated HTTP journey through cast acceptance | Native encryption; no browser needed |
| Chromium | Full portal, including rendering and WASM encryption | Census and publication |

## What a run exercises

1. Authenticate each voter through Keycloak authorization, username/password login and PKCE token exchange.
2. Read `GetVoterStatus` through GraphQL for eligibility, cast status and signed publication URLs.
3. Download the event, elections, summaries and ballot styles from S3.
4. Submit `InsertCastVote` and verify the returned receipt.

Chromium opens the real portal, renders the election list, selects candidates and encrypts through WASM. k6 performs the HTTP journey using ciphertexts generated before the timed run. It requires no browser capture. Preparation and reporting run in Rust for both engines.

## Before you start

Run against a deployment with the S3 voting path enabled, its portal and cast services running, and a tenant reserved for synthetic voters. The tenant must have automatic trustees registered and running. The default fixture uses a threshold of two; `preparation.threshold` is configurable.

Install and authenticate the CLI using [CLI setup](../02-cli/01-cli_cli.md). In a repository devcontainer, enter `devenv shell` from the repository root; it provides k6. Coordination, census generation, encryption and reporting run natively in Rust. The devcontainer also provides Chromium; initialization records its executable automatically. `load check` launches it before browser preparation to verify dependencies. Outside devenv, follow the browser installation instructions in CLI setup.

## Prepare and run

`init` uses the tenant and administrator session already configured in the CLI. Its default workload is 100 voters, two concurrent voters per worker, and a single-election fixture.

```bash
step-cli load init \
  --target local

read -rs -p 'Shared synthetic voter password: ' LOAD_PASSWORD
export LOAD_PASSWORD

step-cli load check voting-load.yaml
```

Choose an engine below; matching examples on this page switch together.

```bash group="engine" tab="k6"
step-cli load prepare voting-load.yaml \
  --engine k6 \
  --output runs/smoke
step-cli load run runs/smoke \
  --workers 4
step-cli load report runs/smoke \
  --open
```

```bash group="engine" tab="Chromium"
step-cli load prepare voting-load.yaml \
  --engine chromium \
  --output runs/smoke
step-cli load run runs/smoke \
  --workers 4
step-cli load report runs/smoke \
  --open
```

Preparation creates the election, imports the census, completes the automatic key ceremony, publishes, opens voting and prepares worker inputs. The run command automatically writes `runs/smoke/report.html`; `report` opens or regenerates it. No administrator credentials are sent to workers.

Each preparation creates an isolated election unless `preparation.existing_event` is configured. Every run can be executed **once**. Keep an interrupted run for investigation; do not delete its attempt markers or rerun it with the same voters.

## Define your workload

Edit `voting-load.yaml` before preparation. All effective defaults are written by `init`. For example:

```yaml group="engine" tab="k6"
workload:
  engine: k6
  mode: vote
  count: 10000
  username_prefix: load-
  start: 0
  shard_size: 1000
  concurrency: 20
  password_env: LOAD_PASSWORD
execution:
  workers: 20
goals:
  status_ms: {p50: 100, p99: 500}
  cast_ms: {p50: 100, p99: 500}
  journey_ms: {p50: 2000, p99: 5000}
min_casts_per_second: 100
```

```yaml group="engine" tab="Chromium"
workload:
  engine: chromium
  mode: vote
  count: 1000
  username_prefix: load-
  start: 0
  shard_size: 100
  concurrency: 2
  password_env: LOAD_PASSWORD
execution:
  workers: 4
goals:
  status_ms: {p50: 100, p99: 500}
  cast_ms: {p50: 100, p99: 500}
  journey_ms: {p50: 2000, p99: 5000}
min_casts_per_second: 1
```

Start with 100 voters when checking a deployment. The examples above use 10,000 voters for k6 and 1,000 for Chromium; increase load only after reviewing the smaller run. Goals are workload targets, not promised capacity.

A worker loads one shard at a time. Census generation computes one shared password hash and streams it into CSV; login still verifies every password. k6 ballots have fresh randomness and unique IDs even when every voter selects the same candidates. Preparation time is excluded from measured throughput.

For a status-only workload, use `workload.mode: status` with k6, configure only status/journey latency goals, and leave `min_casts_per_second: 0`. This measures login and voter status without casting.

### How workers divide the census

Worker count controls independent processes or pods; `workload.concurrency` controls simultaneous voters within each worker. With 20 workers and concurrency 20, up to 400 voters run concurrently. These settings control offered load; the report measures the throughput the deployment actually achieves.

Shard `s`, iteration `i` owns voter `prefix + (start + s × shard_size + i)`. Worker `n` of `N` processes shards `n, n + N, …`. The final shard may be shorter. Each iteration gets one distinct voter; neither workers nor iterations wrap around the census.

Census CSV and encryption output are streamed, workers load one shard at a time, and reporting merges individual samples in SQLite on disk. `shard_size` bounds ballot input memory; total voter count determines disk usage and preparation time. Allow space for prepared ciphertexts and result samples on the coordinator and shared volume.

Prepared configuration and ciphertexts are checked by digest before execution. A durable, exclusive attempt marker prevents a restarted worker from silently casting a shard twice. Use a new preparation and unused voter range for each subsequent run.

## Target a remote deployment

Authenticate `step-cli step config` against the synthetic tenant in your target deployment, then initialize its workload:

```bash
step-cli load init \
  --target remote \
  --portal-url https://vote.example.org \
  --storage-origin https://ballots.example.org \
  --output remote.yaml
```

Use your deployment's portal and public S3/CDN addresses in place of the example domains. Keycloak and GraphQL addresses come from the authenticated CLI configuration. For multiple storage hosts, list each in `target.storage_origins`. These hosts must be reachable from every worker. Signed URLs are never edited.

Remote initialization selects `target.upload_mode: direct`. The `local` mode is only for devcontainer upload routing. The deployment's configured portal URL must agree with `target.portal_url`, because it determines Keycloak redirects. Install any private CA in the coordinator and worker trust stores.

```bash
read -rs -p 'Shared synthetic voter password: ' LOAD_PASSWORD
export LOAD_PASSWORD
step-cli load check remote.yaml
```

Choose **one** execution method below, finish its configuration, then prepare and run. For local processes, use the [prepare and run commands](#prepare-and-run) with `remote.yaml`. Start with a small cohort; increasing generator capacity does not establish server capacity.

## Docker

For a remote target, set the image and Docker network in `remote.yaml` before preparation:

```yaml group="engine" tab="k6"
execution:
  image: voting-load:k6
  network: bridge
```

```yaml group="engine" tab="Chromium"
execution:
  image: voting-load:chromium
  network: bridge
```

Build the selected worker image, then prepare and run:

```bash group="engine" tab="k6"
step-cli load image \
  --engine k6 \
  --tag voting-load:k6
step-cli load prepare remote.yaml \
  --engine k6 \
  --output runs/remote
step-cli load run runs/remote \
  --executor docker \
  --workers 4
```

```bash group="engine" tab="Chromium"
step-cli load image \
  --engine chromium \
  --tag voting-load:chromium
step-cli load prepare remote.yaml \
  --engine chromium \
  --output runs/remote
step-cli load run runs/remote \
  --executor docker \
  --workers 4
```

Local initialization selects the coordinator's network so a loopback-only portal remains reachable. In the repository devcontainer this is `container:<devcontainer hostname>`; on a Linux host it is `host`. For older configurations targeting a portal on the named devcontainer, use:

```yaml
execution:
  network: container:devcontainer
```

The images contain a standalone Rust worker and the selected engine; the coordinator mounts prepared inputs and passes only the synthetic password. The CLI translates devcontainer bind mounts to daemon-host paths automatically. For unusual remote-daemon layouts, set `execution.docker_mount_source` to the host path of the prepared `inputs` directory.

## Kubernetes

Use your current kubectl context, with permission to create Jobs, Pods, Secrets and PVCs in the configured namespace. Your cluster needs a ReadWriteMany storage class. Configure the image, namespace, storage class and volume size before preparation:

```yaml group="engine" tab="k6"
execution:
  executor: kubernetes
  workers: 20
  image: registry.example.org/team/voting-load:k6
  namespace: load-testing
  storage_class: shared-storage
  storage_size: 20Gi
  wait_timeout: 1h
```

```yaml group="engine" tab="Chromium"
execution:
  executor: kubernetes
  workers: 20
  image: registry.example.org/team/voting-load:chromium
  namespace: load-testing
  storage_class: shared-storage
  storage_size: 20Gi
  wait_timeout: 1h
```

Create the namespace if it does not already exist:

```bash
kubectl create namespace load-testing
```

Build and publish the image using your Docker registry credentials:

```bash group="engine" tab="k6"
step-cli load image \
  --engine k6 \
  --tag registry.example.org/team/voting-load:k6 \
  --push
step-cli load prepare remote.yaml \
  --engine k6 \
  --output runs/remote
step-cli load run runs/remote \
  --executor kubernetes \
  --workers 20
```

```bash group="engine" tab="Chromium"
step-cli load image \
  --engine chromium \
  --tag registry.example.org/team/voting-load:chromium \
  --push
step-cli load prepare remote.yaml \
  --engine chromium \
  --output runs/remote
step-cli load run runs/remote \
  --executor kubernetes \
  --workers 20
```

The CLI creates a Secret and PVC, transfers prepared inputs, starts the indexed Job, and collects worker results. It prints resource names and retains the Job, PVC and Secret for reconciliation. After collecting and reviewing results, use the resource name printed by the CLI:

```bash
read -r -p 'Run resource name printed by the CLI: ' LOAD_RESOURCE
kubectl delete job,pod,pvc,secret "$LOAD_RESOURCE" \
  --namespace load-testing \
  --ignore-not-found
```

Use your configured namespace in place of `load-testing`. Failed or interrupted jobs are not automatically retried. Keep worker clocks synchronized; the combined measured interval uses timestamps from all workers.

## Existing events and preparation

To reuse an already provisioned event, set `preparation.existing_event` to a previous run's `inputs/config.json`, and choose an unused `workload.start` range. The CLI imports the new census and prepares fresh ballots; it does not republish that event. The existing event must still be open and eligible.

Custom fixtures use `preparation.template`; explicit ballot selections use `preparation.choices`. Paths resolve relative to the workload YAML. Deployments that run S3 publication preparation separately can set `preparation.publication_preparer` to their application writer executable; its database and S3 environment must be configured on the coordinator.

## Read and share results

Open `report.html` from the run directory. It is a self-contained document suitable for sharing or printing to PDF: successful journeys, accepted casts per second, response distributions and configured goals.

![Example voting-load report captured from an actual local run](/img/voting-load-report.png)

### Local example: 100 voters per engine

These full voting journeys ran sequentially on the same local devcontainer deployment on 8 September 2026. Each engine used four workers with two concurrent voters per worker and its own census range. All 200 unique API receipts matched persisted PostgreSQL ballots. The screenshot shows the Chromium run.

| Engine | Successful voters | Workers × concurrency | Duration | Casts/s | Status p50 / p99 | Cast p50 / p99 | Journey p50 / p99 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| k6 | 100/100 | 4 × 2 | 1.84 s | 54.29 | 24.0 / 47.1 ms | 25.0 / 232.9 ms | 129.0 / 340.1 ms |
| Chromium | 100/100 | 4 × 2 | 62.80 s | 1.59 | 32.9 / 80.9 ms | 18.7 / 36.4 ms | 4837.5 / 6000.1 ms |

Preparation is excluded. Chromium includes page rendering and browser encryption; k6 uses prepared encrypted ballots. These small local runs illustrate the report and verify the journey; they are not deployment capacity estimates. No latency or throughput goals were configured for these examples.

### Interpret the measurements

| Measure | Meaning |
| --- | --- |
| Successful journeys | Distinct voters that completed the configured journey |
| Accepted casts/s | Unique API receipts divided by the interval from the first journey start to the last completion |
| p50 / p99 | Global percentiles of individual samples across all workers |
| Duration | Measured journey interval, excluding census generation and encryption preparation |
| Goals | Explicit thresholds from the workload configuration |

k6 includes authentication, voter status, publication downloads and cast acceptance. Chromium additionally includes rendering and browser encryption. Compare runs using the same engine and workload. Status-only reports show successful journeys per second because they cast no votes.

Every planned journey must succeed; voting runs also require unique receipts. A missing worker, failed journey or missed threshold makes the run fail. Reports are still produced for partial runs. An API receipt confirms acceptance; it does not independently prove database persistence or successful tallying.

### Regenerate, audit or capture a report

```bash
step-cli load report runs/smoke \
  --open
step-cli load report runs/smoke \
  --screenshot report.png
```

Screenshot capture needs Playwright and Chromium configured in `runtime`; ordinary HTML reporting does not. Screenshot dimensions are in `reporting`.

For an optional read-only receipt audit, use a DSN with `sslmode=require` for remote PostgreSQL; the native TLS connector validates the server certificate against system trust. Plain HTTP and non-TLS PostgreSQL are supported for isolated synthetic local deployments only. Remote CLI and Keycloak endpoints should use HTTPS.

```bash
read -rs -p 'Read-only backend PostgreSQL DSN: ' LOAD_AUDIT_DSN
export LOAD_AUDIT_DSN
step-cli load report runs/smoke \
  --dsn-env LOAD_AUDIT_DSN
unset LOAD_AUDIT_DSN
```

## Investigate a failure

The run contains `settings.yaml` (effective configuration), `setup/` (private provisioning logs), `inputs/` (publication and encrypted shards), and `inputs/results/` (worker logs, samples and attempt markers). Census CSV batches and import checkpoints live in `setup/census/`, including runs that reuse an existing event.

`results.json` retains the request inventory for diagnostics. Enable `workload.trace_http` before preparation for sanitized per-fetch protocol logs. These details stay out of the summary report. Client request counts describe traffic to Keycloak, Hasura and object storage; they do not reveal every internal Hasura SQL query. Optional developer capture tools can collect browser HAR and database statement-log intervals, which may include background work. Keep raw logs and inputs private: they may contain voter credentials, signed URLs and ballots.

If preparation fails, inspect `setup/setup.log` and `setup/setup-state.json` before creating another election. If execution fails, inspect the affected worker log and reconcile accepted receipts. Preserve the run and prepare a new range; deleting attempt markers risks duplicate casts.

## Remove a synthetic election

After retaining the report and reconciling failures, delete the event created by that run. Its ID is `election_event_id` in `inputs/config.json`, or in `setup/setup-state.json` if preparation stopped early. Authenticate the CLI against the same tenant first.

```bash
read -r -p 'Synthetic election event ID from this run: ' LOAD_EVENT_ID
step-cli step delete-election-event \
  --election-event-id "$LOAD_EVENT_ID"
```

A run configured with `preparation.existing_event` does not own that event; retain it for its other users. Deleting an election does not remove the local report or Kubernetes resources.

## Command and configuration reference

Use the [load CLI reference](../02-cli/voting-load-reference.md) for every command, configuration field and default. For simulated calls, use the separate [telephone load-testing guide](../12-ivr/telephone-load-testing-guide.md).
