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
  count: 1000000
  username_prefix: load-
  start: 0
  shard_size: 10000
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
  count: 1000000
  username_prefix: load-
  start: 0
  shard_size: 10000
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

A worker loads one shard at a time. Census generation computes one shared password hash and streams it into CSV; login still verifies every password. k6 ballots have fresh randomness and unique IDs even when every voter selects the same candidates. Preparation time is excluded from measured throughput.

For a status-only workload, use `workload.mode: status` with k6, configure only status/journey latency goals, and leave `min_casts_per_second: 0`. This measures login and voter status without casting.

See [deployment](./voting-load-deployment.md) for remote targets and containers, [results](./voting-load-results.md) for reports, and [worker design](./prepared-vote-load.md) for ownership and failure handling.
