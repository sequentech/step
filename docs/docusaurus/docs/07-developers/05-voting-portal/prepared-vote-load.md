---
id: prepared-vote-load
title: Prepared ballots and distributed load runners
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

## Separate preparation, load and verification

Census generation/import and WASM ballot encryption finish **before** measured casting. Each imported synthetic voter owns one independently encrypted ballot. The preparation browser follows the real S3 portal flow and intercepts the final `InsertCastVote` request, returning a local error without forwarding the mutation. It rejects other mutations and verifies that no prepared ballot is present in PostgreSQL. Currently each fixture voter must have exactly one eligible election; prepare separate event batches for broader fixtures.

The load controller assigns disjoint ballots to independent workers. Workers submit each assigned ballot at most once, with no retries or redirect following. k6 uses a constant arrival rate, independent of response latency. Chromium/Obscura workers submit the same prepared mutations with browser `fetch`; these are **cast-only transport tests**, not full UI journey measurements. `capture.py` remains the separate login, S3, encryption and confirmation E2E gate.

A collector merges raw sample timings and verifies accepted receipts against persisted ballot IDs and tenant/event/election scope. It evaluates p50, p99 and accepted casts per second across all workers. It never averages worker percentiles or adds worker rates with different time windows.

## Run locally in devenv

Use the E2E worktree inside the devcontainer. The pinned `devenv.lock` supplies k6; the measured binary is k6 1.6.0 on linux/arm64. Node/Playwright dependencies and Chromium must be installed before preparation. All paths below are examples for private, ignored artifacts. Do not commit census CSVs, encrypted ballots, bearer tokens or raw database logs.

1. Provision an election and complete key ceremony, S3 publication preparation and voting activation. Export the **imported server-assigned** election/area IDs into the CLI external configuration. Authenticate `step-cli` before census import. The existing online fixture setup supports this lifecycle; the commands below reuse an already activated local event.
2. Generate/import the census, choosing a fresh, nonoverlapping username range:

```sh
python3 scripts/voting_e2e/census.py PRIVATE_EXTERNAL_CONFIG PRIVATE_CENSUS_DIRECTORY \
  --cli PRIVATE_STEP_CLI --count 100 --username-start 98121000
```

The CLI configuration supplies event scope. Email addresses are unique synthetic `example.invalid` values. Generation and import logs remain private; the importer requires an explicit CLI success result and records a census digest. Preparation subsequently validates each voter's actual authentication and election eligibility. Avoid duplicate usernames across census batches.

3. Prepare unique ciphertexts using real portal/WASM encryption:

```sh
python3 scripts/voting_e2e/load.py prepare PRIVATE_TARGET \
  PRIVATE_CENSUS_DIRECTORY/voters_100.csv PRIVATE_PREPARATION --count 100 --concurrency 4
```

`PRIVATE_TARGET` uses the capture target schema: portal login URL, allowed origins, event/tenant, database log paths and environment-variable names containing observer DSNs. Preparation emits `ballots.json` and, only after validation, `prepared.json`. The latter binds the artifact digest and records that no prepared ballot was persisted. Partial preparation output cannot be dispatched.

Authentication tokens have a finite lifetime. Dispatch requires enough remaining lifetime for the scheduled run plus a 60-second margin. For ciphertexts prepared earlier, refresh authentication without re-encrypting:

```sh
python3 scripts/voting_e2e/load.py prepare PRIVATE_TARGET UNUSED_CSV_ARGUMENT \
  PRIVATE_REFRESHED_PREPARATION --refresh PRIVATE_PREPARATION/ballots.json \
  --offset 0 --count 100
```

Refresh uses the private credentials in the prepared artifact. It verifies that the current style ID and publication version still match; a changed publication requires re-encryption. It also rejects ballots already present in storage. Keep the **same durable claims ledger** after refreshing tokens. Preparation concurrency is configurable from 1 to 8 browser contexts. Preparation is batched: use `--offset` and `--count` to keep authentication lifetimes, memory and encryption time bounded for large censuses.

4. Run two independent local k6 worker processes with disjoint shards:

```sh
python3 scripts/voting_e2e/load.py local PRIVATE_TARGET \
  PRIVATE_PREPARATION/ballots.json PRIVATE_LOAD_OUTPUT \
  --ledger PRIVATE_DURABLE_CLAIMS \
  --nodes 2 --rate 5 --duration 10 --vus 4 --engine k6 \
  --p50 100 --p99 250 --min-cps 9
```

`--rate` is offered casts/s **per node**; `--duration` is seconds. This example requires exactly 100 unused prepared ballots and offers 10 casts/s globally. `--offset` selects a later unused range. A fixture shorter than the assigned cast count is an error, never an instruction to wrap around and vote again. k6 may schedule an extra tick at the duration boundary; the runner counts it as `arrival_boundary_ticks` and sends no extra cast. Accepted-count and missing-sample checks still require every assigned ballot. `--p50` and `--p99` are maximum milliseconds, while `--min-cps` is the minimum accepted casts/s across the combined measured window. These are configurable acceptance targets, not promised platform capacity.

For prepared browser transport use `--engine chromium` or `--engine obscura`. The latter requires a reachable CDP endpoint. The prepared-cast adapter includes the nonblocking CDP workaround described below; full portal journeys still fail the storage compatibility gate. The browser loads the real portal's small `/favicon.svg` resource before the coordinated start so the document has the correct local-network address space. It then sends casts from the portal origin, preserving browser CORS/network enforcement. Browser workers drop an arrival if their concurrency limit or scheduling deadline is exceeded.

## Measurements and acceptance

- k6 records cast latency, failed casts, accepted counts and dropped iterations with thresholds. Its setup wait, authentication and encryption are excluded from the cast timing. Node-local percentile thresholds are checked as well as global goals.
- Every worker writes private per-attempt samples, timing, response status/size and the scoped accepted receipt. `results.json` reports the aggregate goals and PostgreSQL verification. Missing/crashed workers, missing/duplicate samples, failed casts, dropped arrivals or missing persisted receipts fail the run.
- Cast p50/p99 use all attempted cast latencies, including failed attempts. Throughput is accepted casts divided by the common scheduled duration, extended through the last response if completion runs past that window. Successful results require persistence verification for every accepted receipt. Database verification and setup time are excluded from this measured window.
- The local controller records actual backend and Keycloak SQL JSON logs and summaries over the capture interval. These include background SQL and the coordinated-start wait; client-IP attribution is not per-request correlation. Observer verification queries are excluded.
- `run.json` records fixture digest, source commit/source-file hashes, architecture, CPU count, goals and node assignments. Each shard has checked configuration and ballot digests. Keep application commit/bundle, service versions, logging settings and machine specifications alongside results when comparing deployments. A co-located generator and server compete for CPU and do not establish distributed production capacity.

## Ephemeral devcontainer runners

The local controller's stages are also the worker contract for later ephemeral instances:

1. A durable coordinator provisions the census, prepares encrypted ballots, refreshes authentication shortly before load, and owns the claims ledger outside ephemeral disks.
2. Use `load.py dispatch` with the same arguments as `local` to create `node-000`, `node-001`, etc. Choose `--start-delay` long enough for workers to boot, with token lifetimes covering that interval. Preparation and dependency installation must not run inside the measured phase.
3. Give each ephemeral worker **only its assigned private shard** and a prebuilt devcontainer image containing the pinned source, dependencies and warmed Nix store. Each worker runs:

```sh
scripts/voting_e2e/worker.sh /private/assigned-shard
```

For a locally built worker image, a future launcher can use this contract:

```sh
docker run --rm --pull=never --network "$LOAD_NETWORK" \
  --mount "type=bind,source=$LOAD_SHARD,target=/private/assigned-shard" \
  --entrypoint bash "$LOAD_RUNNER_IMAGE" -lc \
  'cd /workspaces/step && scripts/voting_e2e/worker.sh /private/assigned-shard'
```

The image must already contain the E2E worktree at `/workspaces/step`. The bind mount persists receipts and the `attempted` marker after container deletion. A remote deployment needs routable service URLs, matching portal CORS rules and synchronized clocks; Docker-local hostnames cannot simply be copied between hosts. Independent Obscura nodes should own independent browser/CDP instances.

4. Collect every shard's output at the coordinator and run `load.py merge PRIVATE_LOAD_OUTPUT` with the observer DSN environment. Keep shards and claims after failure until storage reconciliation is complete. The same ballot is never automatically reissued to another worker.

`dispatch` reserves every selected ballot using exclusive durable claim files, and `worker` creates its own exclusive attempt marker before loading any ballot. Repeating a worker on the same shard fails immediately. Copying an attempted shard or changing the ledger directory is **not** a supported retry mechanism. Future remote orchestration must enforce single assignment in durable shared storage; local file claims alone do not coordinate independent hosts with copied disks.

Automatic VM/cloud provisioning, remote service deployment and a distributed durable scheduler are future work. This increment runs separate node processes on this machine and supplies the worker/artifact/goal contract for ephemeral devcontainers.

## Validation commands

```sh
python3 -m unittest discover -s scripts/voting_e2e -p 'test_*.py'
yarn --cwd packages/voting-portal typecheck:e2e
```

The regression tests cover disjoint/exhaustive partitioning, permanent claims, merged raw percentiles, full-window throughput, and failure/missing-sample rejection.

## Recorded local runs

Measured 2026-09-07 UTC on this machine, with two independent worker processes in the existing aarch64 devcontainer. The application remains main commit `5131ff4658` for the portal/S3 action and release/10 B3 for cast services; full SQL diagnostic logging is enabled. Preparation used the real portal and independently encrypted every ballot before load.

| Runner | Nodes | Persisted casts | Offered casts/s | Observed casts/s | p50 ms | p99 ms | Goals passed |
|---|---:|---:|---:|---:|---:|---:|---|
| k6 | 2 | 100/100 | 10 | 10.00 | 18.00 | 21.01 | Yes |
| Chromium | 2 | 4/4 | 2 | 2.00 | 28.50 | 32.00 | Yes |

The k6 run offered 5 casts/s per node for 10 seconds, with goals p50 ≤100 ms, p99 ≤250 ms and ≥9 accepted casts/s globally. The Chromium check offered 1 cast/s per node for two seconds, with the same latency goals and ≥1.5 casts/s. All receipts matched PostgreSQL; neither run dropped or failed a cast. The final k6 run recorded two extra duration-boundary ticks and sent no extra votes. These small local runs validate the harness and the specified offered rates; they do not establish maximum throughput or production tail latency.

The final census imported 110 new voters and prepared all 110 ballots with four concurrent browser contexts and zero persisted casts during preparation. The k6 and Chromium allocations were disjoint. Authentication refresh was also exercised on two remaining unused ballots; their encrypted payloads remained byte-for-byte identical, and both subsequently cast successfully through two k6 workers. Relaunching an attempted worker was rejected before any ballot was loaded, leaving its samples unchanged.

Exploratory results remain private and are not substituted for passing runs: a six-cast k6 smoke test passed; the first Chromium attempt was blocked before POST submission by the synthetic page’s local-network address space; the first 100-cast k6 run persisted all votes but exited failed on an extra boundary tick. Both harness issues were fixed and validated using unused voters, without retrying consumed ballots.

### Recorded API and SQL work

The final k6 load submitted exactly **100 POST `/v1/graphql` operations named `InsertCastVote`**, all HTTP 200 with validated receipts. Chromium submitted four such operations and loaded a small portal resource per worker before the measured window. Census import, login, status/S3 requests and encryption occur in preparation, outside this cast-only load. Each private per-attempt sample records the endpoint, operation, status, response size and latency; the existing full-journey capture supplies the broader login/S3 endpoint inventory.

The table below records submitted SQL during the final k6 controller interval, including the coordinated-start wait and a one-second log-drain interval. Background/maintenance work can be included. The private `backend-sql.json` and `keycloak-sql.json` contain the actual query records and nested plans; no credentials, voter IDs or SQL literals are published here.

| Database | Service | Category | Verb | Count |
|---|---|---|---|---:|
| backend | harvest | empty protocol check | EMPTY | 105 |
| backend | harvest | nested SQL plan | PLAN | 600 |
| backend | harvest | submitted SQL | INSERT | 100 |
| backend | harvest | submitted SQL | SELECT | 400 |
| backend | harvest | transaction | COMMIT | 100 |
| backend | harvest | transaction | START | 100 |
| backend | hasura | submitted SQL | SELECT | 27 |
| backend | hasura | submitted SQL | WITH | 4 |
| backend | hasura | transaction | BEGIN | 29 |
| backend | hasura | transaction | COMMIT | 29 |
| backend | windmill | empty protocol check | EMPTY | 10 |
| backend | windmill | submitted SQL | SELECT | 84 |
| backend | windmill | transaction | COMMIT | 5 |
| backend | windmill | transaction | ROLLBACK | 5 |
| backend | windmill | transaction | START | 10 |
| keycloak | harvest | empty protocol check | EMPTY | 5 |
| keycloak | keycloak | submitted SQL | DELETE | 2 |
| keycloak | keycloak | submitted SQL | SELECT | 29 |
| keycloak | keycloak | transaction | BEGIN | 29 |
| keycloak | keycloak | transaction | COMMIT | 29 |
| keycloak | windmill | empty protocol check | EMPTY | 2 |
| keycloak | windmill | transaction | ROLLBACK | 2 |
| keycloak | windmill | transaction | START | 2 |

Validation: 17 Python regression tests, the dedicated E2E TypeScript check, REUSE, and the Docusaurus production build. Obscura also passed four prepared casts after the nonblocking adapter fix, through two client workers sharing one local Obscura server. Its full-journey compatibility gate remains failing. Ephemeral containers and remote machines were not launched.

The scheduling and threshold choices follow the official [k6 constant-arrival-rate](https://grafana.com/docs/k6/latest/using-k6/scenarios/executors/constant-arrival-rate/), [execution identifiers](https://grafana.com/docs/k6/latest/javascript-api/k6-execution/) and [threshold](https://grafana.com/docs/k6/latest/using-k6/thresholds/) documentation.


## Obscura screenshots and root-cause debugging

The pinned Obscura 0.2.2 binary uses native rendering. Its CDP `Browser.getVersion` reports a compatibility string (`Chrome/145.0.0.0`), not a real Chromium engine version. Synthetic pages and fresh server processes reproduced the following independently of the voting portal.

### Storage is discarded on navigation

Both `localStorage` and `sessionStorage` contained `PRESENT` before navigation and were `null` afterward. Chromium preserved both keys in three identical runs. This breaks the browser-state guarantees needed by the full authentication journey. In the pinned source, [page initialization rebuilds the JS runtime](https://github.com/h4ckf0r0day/obscura/blob/a1e09de68c7617b8079fbb1661b0548c501971c1/crates/obscura-browser/src/page.rs#L1781), while [bootstrap creates fresh storage maps](https://github.com/h4ckf0r0day/obscura/blob/a1e09de68c7617b8079fbb1661b0548c501971c1/crates/obscura-js/js/bootstrap.js#L10820); the inspected navigation path does not restore them.

![Storage present before navigation](/img/obscura-storage-before.png)

![Storage missing after navigation](/img/obscura-storage-after.png)

The debug script records DOM values separately from screenshots. An initial screenshot retained stale text despite the DOM reporting `MISSING`; changing the viewport width by one pixel refreshed the paint. The second image above is the refreshed rendering, agreeing with the captured DOM/storage values.

### Awaited evaluation and request interception stall each other

A synthetic POST reaches the local echo server when interception is disabled. With interception enabled, returning the unresolved fetch promise from `page.evaluate()` stalls: no POST reaches the server within the probe deadline. Starting the same fetch asynchronously and returning immediately lets Playwright receive the pause event and issue `route.continue()`; the server then receives exactly the intended POST/body.

The pinned [CDP dispatch loop](https://github.com/h4ckf0r0day/obscura/blob/a1e09de68c7617b8079fbb1661b0548c501971c1/crates/obscura-cdp/src/server.rs#L993) uses an interception-aware command pump for navigation, while ordinary runtime commands are awaited serially. This explains the observed stall when runtime evaluation waits for an intercepted request that itself needs another CDP command to proceed.

![Synthetic request waiting under awaited interception](/img/obscura-fetch-pending.png)

The Obscura prepared-cast adapter now starts the actual fetch without blocking the CDP evaluation and polls for the genuine response from the host. It retains origin interception, response validation, request timeout and permanent one-use ballot claims. It neither mocks a cast response nor retries a request. **Four of four real votes were accepted and verified in PostgreSQL**, through two client workers sharing one local Obscura server: p50 **26 ms**, p99 **27.97 ms**, **2 casts/s**. These four samples are a compatibility smoke check, not evidence of a latency advantage over Chromium.

The initial awaited-evaluation Obscura attempt produced no cast samples and failed. Its assigned ballots remain reserved; the successful workaround run used different unused voters. For the full portal flow, keep Chromium until storage persistence and the broader compatibility gate are fixed upstream.

### Measured resource comparison

Three fresh runs per engine alternated order. Each loaded the same small local resource, warmed up with ten requests, and completed 100 sequential read-only `GetVoterStatus` calls. Linux `/proc` sampling measured browser descendant processes, excluding the Node driver and backend. PSS is proportional resident memory; CPU values are sampled deltas during the measured requests. These are small-page/transport observations, not full-portal or encryption resource measurements.

| Median across three runs | Chromium 144 | Obscura 0.2.2 |
|---|---:|---:|
| Peak browser PSS | 296.82 MiB | 35.41 MiB |
| Browser CPU during 100 requests | 0.32 s | 0.06 s |
| Startup plus small-page load | 200.67 ms | 77.96 ms |
| Per-run status p50 | 17.30 ms | 16.00 ms |

Obscura was substantially lighter in this probe. That resource advantage does not make missing browser semantics acceptable for full-journey validation.

Reproduce screenshots and protocol behavior using synthetic local pages, without voter credentials:

```sh
node packages/voting-portal/test/load/obscura-debug.cjs PRIVATE_DEBUG_OUTPUT \
  --routed --no-extra-cdp
# The awaited variant intentionally fails on the pinned Obscura release:
node packages/voting-portal/test/load/obscura-debug.cjs PRIVATE_AWAIT_DEBUG_OUTPUT \
  --routed --no-extra-cdp --await-fetch
```

For browser resource measurements, use already prepared authenticated input; this command sends only status queries, never casts:

```sh
python3 scripts/voting_e2e/browser_cost.py PRIVATE_TARGET \
  PRIVATE_PREPARATION/ballots.json PRIVATE_COMPARISON_OUTPUT --repeats 3
```

The debug/comparison scripts launch fresh local Obscura servers on ports 19224/19223 respectively and stop those processes afterward. Synthetic screenshots above contain no voter information. Raw protocol traces, actual load receipts and census artifacts remain private.
