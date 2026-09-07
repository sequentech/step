---
id: prepared-vote-load
title: Voting load tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Generate once, partition once, cast once. The load controller runs independent k6 workers against the workload derived from a verified [Chromium journey](./voting-flow-e2e.md).

## Prepare

Use the existing devcontainer and `devenv shell`. [Provision an online event](../02-cli/02-tutorials/load-testing/online-load-testing-guide.md), publish it, activate voting and export the server-assigned event/election/area configuration. Authenticate `step-cli`, then generate a fresh census range:

```sh
python3 scripts/voting_e2e/census.py PRIVATE_CONFIG PRIVATE_CENSUS \
  --cli PRIVATE_STEP_CLI --count 100 --username-start 99000000
python3 scripts/voting_e2e/load.py prepare PRIVATE_TARGET \
  PRIVATE_CENSUS/voters_100.csv PRIVATE_PREPARED --count 100 --concurrency 4
```

Preparation uses Chromium's real login and WASM encryption, intercepts the final cast locally and verifies **zero stored casts**. Every voter receives a distinct encrypted ballot. Reusing ciphertext or ballot IDs can trigger duplicate/revote handling and gives a different workload, even when the election will not be tallied.

Generate `PRIVATE_CAPTURE/profile.json` with the Chromium capture command in the journey guide. Use a separate voter for that successful reference cast.

## Run

```sh
python3 scripts/voting_e2e/load.py local PRIVATE_TARGET \
  PRIVATE_PREPARED/ballots.json PRIVATE_RUN \
  --profile PRIVATE_CAPTURE/profile.json --ledger PRIVATE_CLAIMS \
  --nodes 2 --rate 5 --duration 10 --vus 20 \
  --p50 100 --p99 250 --journey-p50 5000 --journey-p99 10000 --min-cps 7
```

| Control | Meaning |
|---|---|
| `--nodes` | Independent worker processes |
| `--rate` | New journeys per second **per worker** |
| `--duration` | Arrival window in seconds |
| `--vus` | Available concurrent virtual users per worker; allow enough for the whole journey |
| `--pacing` | Multiplier for Chromium's recorded offsets; default 1, zero removes waits |
| `--p50`, `--p99` | Cast-response budgets in milliseconds |
| `--journey-p50`, `--journey-p99` | Whole-journey budgets in milliseconds |
| `--min-cps` | Minimum globally accepted casts per second |

Required ballots = **workers × rate × duration**. Shards are disjoint; workers never wrap the input or retry an attempted cast. Profile replay logs in afresh, so previously captured bearer-token expiry does not limit it. Publication changes invalidate prepared ballots and fail the journey.

To isolate the cast API, replace `--profile ...` with `--cast-only`. That mode uses prepared bearer tokens, so refresh them shortly before running if necessary:

```sh
python3 scripts/voting_e2e/load.py prepare PRIVATE_TARGET UNUSED_CSV \
  PRIVATE_REFRESHED --refresh PRIVATE_PREPARED/ballots.json --count 100
```

Refresh preserves ciphertexts and verifies publication identity. Chromium is also available for cast-only browser transport through `--engine chromium`; complete UI tests use the capture runner.

## Read and refresh results

Every merge automatically writes `results.json`, `performance.md` and `performance.svg`. Journey runs also write `traffic.json`, listing actual HTTP methods, endpoints, statuses, bytes and timings. The local controller records backend and Keycloak SQL summaries over the run interval. Raw files remain private.

Add `--publish-docs` to `local` or `merge` to refresh the current documentation table and chart as part of the run.

Percentiles use merged individual samples, never averages of worker percentiles. Throughput uses accepted casts over the common arrival window, extended through the last cast response. Startup, census generation, encryption and database verification are outside that window; login and profile fetches are inside journey timing. Failures, dropped arrivals, missing workers, goal breaches or unmatched receipts fail the run.

Regenerate charts without rerunning tests, and optionally replace the current measurement below:

```sh
python3 scripts/voting_e2e/load.py report PRIVATE_RUN
python3 scripts/voting_e2e/load.py report PRIVATE_RUN --publish-docs
```

The same commands work in CI. Publication replaces one current summary and chart; it does not append a run history or copy private artifacts into documentation.

## Workers on separate machines

Use `load.py dispatch` with the same run arguments to produce `node-000`, `node-001`, etc. Give each worker only its private assigned directory and the same prebuilt devcontainer image:

```sh
scripts/voting_e2e/worker.sh /private/assigned-shard
# After collecting every shard at the coordinator:
python3 scripts/voting_e2e/load.py merge PRIVATE_RUN
```

Set `--start-delay` long enough for startup and keep machine clocks synchronized. Service URLs must be reachable from every worker. The coordinator owns a durable claims ledger outside ephemeral disks; copying a shard or changing ledger directories is not a retry mechanism. VM/container provisioning is separate from this worker contract.

<!-- generated-load-results -->

## Current measurement

**PASS** · k6 journey · 3 workers × 2 arrivals/s × 10 s.

| Measurement | Result | Budget |
|---|---:|---:|
| Accepted casts | 60/60 | All |
| Cast p50 | 17.00 ms | ≤ 100.0 ms |
| Cast p99 | 26.23 ms | ≤ 250.0 ms |
| Accepted casts/s | 4.86 | ≥ 4.5 |
| Journey p50 | 2965.00 ms | ≤ 5000.0 ms |
| Journey p99 | 2967.00 ms | ≤ 10000.0 ms |

Persistence verified: **True**. Failed casts: 0; missing attempts: 0; scheduler drops: 0.

Local aarch64, 8 logical CPUs. Generator and services share the machine. Tail percentiles describe this sample; they are not a production capacity estimate.

Profile coverage: **3240 HTTP requests**, 54 per journey. Request counts match the Chromium recipe: **True**.

![Current voting performance](/img/voting-load-performance.svg)
