---
id: prepared-vote-load
title: Voting worker design
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

`step-cli load` owns the public lifecycle. The internal runtime in `packages/voting-load` contains one finite executor shared by k6 and Chromium, provisioning, aggregation and optional diagnostic capture. Native ballot encryption is part of `step-cli` and uses `sequent-core`.

## Ownership and memory

Shard `s`, iteration `i` owns voter `prefix + (start + s × shard_size + i)`. Worker `n` of `N` handles shards `n, n + N, …`. The final shard may be shorter; neither workers nor iterations wrap around the census.

Census generation hashes the shared password once and streams rows. Encryption streams independently randomized ballots. Each worker loads one shard, and SQLite merges samples on disk. The number of voters affects disk usage and duration; `shard_size` bounds each worker's ballot input memory.

Prepared configuration and ciphertexts are hashed before execution. Each shard takes an exclusive durable attempt marker. Restarting a worker cannot silently cast that shard again. Republish or change voter ranges by preparing new inputs.

## Timing and evidence

The timed k6 journey uses fresh Keycloak cookies and PKCE, reads minimal voter status, downloads signed publication objects and casts its prepared ballot. Chromium uses the shared portal test flow to select, encrypt, confirm and read the receipt. Preparation never requires a Chromium capture.

Aggregation computes global percentiles from individual samples, checks voter ownership and receipt uniqueness, and retains missing work as a failure. HTTP inventories record client-facing requests. Optional database observers provide additional evidence; client traffic does not reveal internal Hasura SQL.

See [results](./voting-load-results.md) for measurement definitions and troubleshooting, and [deployment](./voting-load-deployment.md) for worker placement.
