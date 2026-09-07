---
id: prepared-vote-load
title: Voting worker design
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

The [voting performance guide](./voter-status-performance.md) covers setup, local execution, containers, Kubernetes and reports. The worker model is deliberately finite: every iteration owns one voter and one cast attempt.

## Data ownership

For shard `s`, local iteration `i` uses username `prefix + (start + s × shard_size + i)`. Node `n` of `N` handles shards `n, n + N, n + 2N, …`. The final shard may be shorter; no range wraps and no voter is reused.

k6 uses [shared iterations](https://grafana.com/docs/k6/latest/using-k6/scenarios/executors/shared-iterations/) and `scenario.iterationInTest`, so faster virtual users can complete more work without duplicating voter inputs. A `SharedArray` contains only the current shard's encrypted ballots, not the whole census. Chromium uses the same ranges and opens a new context for each full UI journey.

| Artifact | Contents | Growth |
| --- | --- | --- |
| Census CSV batches | Patterned usernames, shared prehash, eligibility | Disk proportional to voters; one row in generator memory |
| Encrypted JSONL shards | Fresh ballot ID, ciphertext and election ID | Disk proportional to voters; at most `shard_size` ballots per worker |
| Worker configuration | Endpoints, scope, ranges, goals and protocol | Constant size |
| Attempt claims | One exclusive file per shard | Proportional to shards |
| Raw result samples | Timing, outcome and API receipt ID | Streamed to disk |
| Aggregate report | Global percentiles, throughput, requests and failures | Independent of voter count |

## What the timings mean

Preparation includes census import, one publication bootstrap and native encryption. These costs happen before timed k6 iterations. Each iteration includes a fresh login, voter status, publication downloads and cast response. Chromium additionally includes rendering and WASM encryption.

Accepted casts/s uses the interval from the first journey start to the last completion, including the final in-flight work. Global p50/p99 are calculated from individual samples in disk-backed SQLite. The report does not average per-worker percentiles or treat a successful HTTP status with GraphQL errors as a successful vote.

An API receipt confirms API acceptance. Independent persistence verification needs database observation; add `report --dsn-env LOAD_AUDIT_DSN` for a batched receipt audit, or use the [diagnostic capture](./voting-flow-e2e.md) for SQL attribution. The worker deliberately does not execute one administrative database query per voter.

## Failure and capacity

An attempted shard is never automatically retried, even after a pod replacement. Keep its claim and reconcile receipts before any manual recovery. Start a new voting run with a fresh census range and newly prepared ballots.

Bounded client memory does not guarantee a particular backend throughput. Measure encryption time and disk space during preparation, worker CPU/RSS/network during load, and Keycloak/database/cast-service saturation. Increase worker count and virtual users separately to identify which resource limits throughput. Shared passwords remove repeated hashing during import; authentication still performs its normal password verification.

<!-- generated-load-results -->
