---
id: online-load-testing-design
title: Online Load Testing — Design
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Online Load Testing — Design

This documents how the ONLINE (voting portal) load-testing tooling works. For
step-by-step instructions to actually run it, see
[Online Load Testing Guide](online-load-testing-guide.md).

> This covers the **ONLINE** channel only, driven by `step-cli` + Playwright
> browsers. The **TELEPHONE** channel has its own tooling with the same
> two-stage architecture, documented under
> [Telephone Load Testing — Design](../../../12-ivr/telephone-load-testing-design.md).
> For duplicating votes at the database level on a k8s cluster, see
> [Load Testing](load_testing.md).

Each simulated voter drives a real browser through login → election selection
→ candidate selection → review → cast → confirmation, so all portal overhead
(Keycloak auth, GraphQL queries, ballot-style fetches, sequent-core WASM
ballot encryption, the cast mutation) is exercised for real — nothing is
stubbed, intercepted, or blocked.

All three scripts (setup and both Stage 2 runners) take no command-line
arguments; every setting lives in
`packages/step-cli/scripts/telephone-load-test-inputs/config/layers.yaml`
(gitignored — copied from the tracked `layers.yaml.example` next to it),
loaded by a shared `load_test_common.py` module.

## Two stages

1. **Stage 1 — Provisioning** (`step-cli`, network calls to Hasura/Keycloak
   only): the same script the telephone load test uses,
   `packages/step-cli/scripts/setup_telephone_load_test.py`, run with
   `setup.voting_channel: ONLINE` in `layers.yaml`. It imports an election
   event, bulk-creates voters, runs the keys ceremony, publishes, and opens
   **ONLINE** voting. Ends with an election event ready to accept portal
   votes, plus a voters CSV and a `summary.json` recording the portal login
   URL.
2. **Stage 2 — Casting** (`Playwright`): renders a voter manifest from the
   Stage 1 CSV and drives one Playwright run in `packages/voting-portal`,
   where every voter is an independent headless-Chromium session casting a
   real ballot against the dev container's real portal/Keycloak/Hasura.
   Implemented as `packages/step-cli/scripts/run_online_load_test.py` plus
   `packages/voting-portal/test/load/`.

## Design constraints

### Stage 1 is shared with the telephone test, not forked

The provisioning steps are identical for both channels — only the channel
opened at the end differs, so `setup_telephone_load_test.py` grew a
`voting_channel: ONLINE|TELEPHONE` YAML setting (default `TELEPHONE`,
preserving its original behavior) rather than a second script. The DTMF-safe
numeric credentials it generates are a telephone constraint but work fine in
the portal login form, so voter generation stays a single code path. For
`ONLINE` the script records `voting_channel`, `voting_portal_url` and
`login_url` (`{portal}/tenant/{tenant_id}/event/{election_event_id}/login`)
in `summary.json`; both Stage 2 runners check `voting_channel` and refuse a
run dir provisioned for the other channel, since each channel's eligibility
check gates on its own channel being open.

### Single-area pinning matters for ONLINE too

Contests are assigned per voter area, and different areas can have different
contest counts. Stage 1 pins every generated voter to one area
(`setup.voter_area_name`, defaulting to the election event's first area), so
every browser session sees the same contests and one deterministic
candidate-selection strategy is valid for every voter — the browser analog of
"one DTMF template works for every call".

### Concurrency is Playwright workers, not process fan-out

The telephone Stage 2 fans out with a thread pool because each call is a
cheap independent CLI process. Browsers are not cheap: N separate
`playwright test` invocations would each pay a Node + browser startup and
multiply memory. `run_online_load_test.py` therefore invokes Playwright
**once** with `--workers <concurrency>`; each worker reuses one browser
process, and every voter runs in a fresh browser context inside it (so
sessions don't leak between voters, and no explicit logout is needed). The
runner's `online_run.concurrency` setting keeps the same shape as the
telephone script's `telephone_run.concurrency` — the engine underneath
differs.

### One generated test per voter

`test/load/cast_ballot.load.spec.ts` reads the voter manifest at module load
and emits one Playwright `test()` per voter. That is what makes per-voter
results fall out of Playwright's JSON report for free — including voters
killed by the per-test timeout, which no in-test bookkeeping could record.
The runner parses that report into `results.csv`
(`voter_id,status,duration_ms,ballot_id`) and a `summary.json` with the same
shape as the telephone run's. Ballot ids come from rows the spec appends on
success. Without a manifest the spec falls back to
`VOTER_USERNAME`/`VOTER_PASSWORD`/`VOTER_DATE_OF_BIRTH` env vars and runs a
single voter — the smoke-test and selector-debugging mode.

### The success criterion is the rendered ballot id

The confirmation screen renders the ballot id (`data-testid="ballot-id"` in
`ConfirmationScreen.tsx`) only after a ballot has actually been cast — the
ONLINE analog of the telephone test's `"ballot locator"` receipt-prompt
regex. The flow asserts it is visible and non-empty and records it. A voter
whose event has several elections votes them all, one ballot id each.

### No retries

`retries: 0` in `playwright.load.config.ts`: a retried voter would attempt a
second vote and be rejected as a duplicate, skewing results. Failures are
reported with a Playwright trace (`retain-on-failure`) instead; successful
voters record no trace/video, keeping the diagnostic cost independent of the
load being generated.

### Preflight over mid-run failure

Before any browser launches, the runner verifies — each with a one-line
actionable error: the run dir's `summary.json` and voters CSV (tolerating a
moved/copied dir), `voting_channel == ONLINE`, the voting portal root URL,
the Keycloak **realm** well-known endpoint (distinguishing "Keycloak down"
from "realm missing — stale summary?"), Hasura's `/healthz`, the Playwright
binary (`yarn` not run) and the Chromium browser
(`yarn playwright install chromium` not run). This is the browser-world
equivalent of the telephone runner's valkey/`ivr-cli` checks.

### Local port bridging inside the devcontainer

The voting portal's login flow sends the *browser* to Keycloak/MinIO URLs
baked in for a developer's own OS browser (reachable via VS Code's
`forwardPorts`, e.g. `http://localhost:8090`) — a headless browser launched
from *inside* the devcontainer can't reach those the same way, since
`localhost` there is the devcontainer's own loopback, not the host's. The
runner bridges the known ports (Keycloak, Hasura, the MinIO config proxy)
with `socat` for the duration of the run, skipping any that already resolve
(e.g. a distributed run, or a devcontainer with host networking).

## Resource budget: parallel voting clients on a local machine

Each client is a Chromium page doing WASM ballot encryption:

- **Memory**: roughly 250–500 MB RSS per worker.
- **CPU**: login and GraphQL are I/O-bound, but ballot encryption is a real
  CPU burst per contest — and the same machine is usually running the whole
  compose stack.
- **Defaults**: `online_run.concurrency: 4`; rule of thumb
  `min(cores − 2, free_GB / 0.5)`, realistically capping around 8 on a dev
  laptop. The runner warns (doesn't fail) when concurrency exceeds
  `cores − 2`, since queueing skews the latency numbers.
- **Don't load-test webpack-dev-server**: `yarn start:voting-portal` is a
  single-process dev server; above ~8 clients it becomes the bottleneck
  before the backend does. For bigger runs point `online_run.voting_portal_url`
  at a production build served statically, or at a deployed portal.
- **Scope boundary**: browser clients measure realistic per-voter latency and
  exercise the full stack at modest concurrency; raw vote-volume throughput
  remains the job of `step-cli duplicate-votes`
  (see [Load Testing](load_testing.md)). The two compose: cast N real
  browser votes, then duplicate to volume.

## Distributed runs: provision once, load from several machines

Stage 2 (both `run_online_load_test.py` and `run_telephone_load_test.py`) is
almost machine-independent by construction: the only Stage-1 artifacts it
consumes are `summary.json` and the voters CSV — two small, portable text
files. The event itself lives in Keycloak/Hasura, not in the run dir. Since
neither script takes command-line arguments, each load machine needs its own
copy of `layers.yaml` (or the shared one edited before that machine's run) —
what breaks when Stage 2 moves to another machine, and the fixes:

1. **Endpoint URLs inside `summary.json` are whatever Stage 1 used.** In the
   devcontainer that's `127.0.0.1` addresses, meaningless remotely. Two
   complementary fixes:
   - *Preferred*: provision Stage 1 with the **public** URLs
     (`setup.endpoint_url`, `setup.keycloak_url`, `setup.voting_portal_url`),
     so `summary.json` is remote-valid as written — distribution is then
     purely file copying.
   - *Also*: both Stage 2 runners accept `keycloak_url` / `hasura_url`
     overrides (`telephone_run.*` / `online_run.*`, plus the online runner's
     `online_run.voting_portal_url`), so an existing locally-provisioned run
     dir can still be replayed remotely by pointing each machine's
     `layers.yaml` at it.
2. **Getting the run dir there**: `scp`/`rsync` the Stage-1 `setup.out_dir`
   (plus the DTMF template for telephone). Both runners tolerate a moved dir
   — the CSV path in `summary.json` falls back to `<run-dir>/<basename>`.
3. **Disjoint voter shards per machine — the one correctness invariant.**
   Two machines replaying the same CSV rows means duplicate votes (rejected,
   skewing results). `voter_offset` + `max_votes`/`max_calls` slice the CSV
   per machine: machine A `voter_offset: 0, max_votes: 500`, machine B
   `voter_offset: 500, max_votes: 500`, and so on (set per-machine in each
   machine's own `layers.yaml`). Provision Stage 1 with `setup.num_voters` ≥
   machines × per-machine votes. (The telephone runner derives each call's
   synthetic caller number from the voter id, so disjoint voter slices also
   keep caller numbers unique across machines.)
4. **Telephone-only machine-local dependencies**: each load machine needs the
   `ivr-cli` binary built, the IVR client secrets exported as env vars
   (`KEYCLOAK_IVR_SERVICE_CLIENT_SECRET`,
   `KEYCLOAK_IVR_VOTING_CLIENT_SECRET` — only the
   `.devcontainer/.env.development` *fallback* is devcontainer-bound), and a
   session store. The valkey auto-start assumes the devcontainer's compose
   network, so remotely each machine runs its own local valkey/redis and sets
   `telephone_run.valkey_url`. Nothing needs sharing between machines: IVR
   sessions are per-call state consumed only by that machine's own `ivr-cli`
   processes.
5. **Online-only**: each machine needs the JS dependencies and the Playwright
   Chromium installed (the preflight says exactly what to run), and the
   portal URL must be a deployed/production-served portal — which a
   distributed run implies anyway.
6. **Aggregation**: each machine produces its own `results.csv` +
   `summary.json`; the counts are additive and the CSVs concatenate (voter
   ids are globally unique), so a one-liner merges them. Combined
   votes/minute spans the earliest `started_at` to the latest `finished_at`.
