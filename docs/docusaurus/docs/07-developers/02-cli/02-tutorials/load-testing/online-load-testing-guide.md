---
id: online-load-testing-guide
title: Online Load Testing Guide
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Online Load Testing Guide

Step-by-step instructions for running the ONLINE (voting portal) load test:
provisioning an election event with many voters, then driving many parallel
browser sessions that each cast a real ballot through the portal UI. See
[Online Load Testing — Design](online-load-testing-design.md) for how this
works under the hood.

All commands below assume a terminal opened **inside the dev container**
(VS Code Dev Containers or GitHub Codespaces) — that's where `cargo`,
`step-cli`'s dependencies (Hasura/Keycloak), the `trustee1`/`trustee2`
containers and the voting portal dev server are reachable.

Every command below writes its output under
`packages/step-cli/scripts/online-load-test-output/` (gitignored) rather than
`/tmp`, so a run survives a container restart and stays easy to find between
steps.

## Prerequisites

- The `keycloak`, `graphql-engine` (Hasura), `trustee1` and `trustee2`
  containers must be running (`docker ps`; `docker start trustee1 trustee2`
  if they aren't — the keys ceremony hangs without them).
- The voting portal on port 3000. Check first, then start it if needed:

  ```bash
  curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:3000/ || \
    (cd packages && yarn start:voting-portal)
  ```

- `jq` (already installed in the dev container).

## 0. Build step-cli and install Playwright

```bash
cd packages && CARGO_TARGET_DIR=/workspaces/step/packages/step-cli/rust-local-target cargo build --release -p step-cli
cd /workspaces/step
export PATH="/workspaces/step/packages/step-cli/rust-local-target/release:$PATH"

# JS dependencies (installs @playwright/test) and the Chromium browser:
cd packages && yarn && cd voting-portal && yarn playwright install chromium
cd /workspaces/step
```

## 1. Stage 1 — provision the election event and voters

The same provisioning script the telephone load test uses, with the ONLINE
channel opened instead:

```bash
packages/step-cli/scripts/setup-telephone-load-test.sh \
  --election-event-json packages/step-cli/scripts/telephone-load-test-inputs/election-event.json \
  --voting-channel ONLINE \
  --num-voters 20 \
  --out-dir packages/step-cli/scripts/online-load-test-output/run
```

This imports the tracked example election event, generates 20 voters (all
placed in the same area, with unique numeric username/PIN each), runs the
keys ceremony, publishes, and opens `ONLINE` voting. Outputs land in
`--out-dir`: `summary.json` (including the portal `login_url`) and the voters
CSV.

## 2. Smoke-test one voter (optional but recommended)

Before fanning out, watch a single voter cast a ballot in a headed browser —
this catches a wrong URL, a closed channel, or changed selectors immediately:

```bash
LOGIN_URL=$(jq -r .login_url packages/step-cli/scripts/online-load-test-output/run/summary.json)
VOTER_USERNAME=$(awk -F, 'NR==2 {print $1}' packages/step-cli/scripts/online-load-test-output/run/voters_20.csv)
VOTER_PASSWORD=$(awk -F, 'NR==2 {print $3}' packages/step-cli/scripts/online-load-test-output/run/voters_20.csv)
LOGIN_URL="$LOGIN_URL" VOTER_USERNAME="$VOTER_USERNAME" VOTER_PASSWORD="$VOTER_PASSWORD" \
  yarn --cwd packages/voting-portal playwright test --config playwright.load.config.ts --headed
```

(Check the `username`/`password` column positions against the CSV header —
they depend on the `fields` list in the run's `external_config.json`. The
voter used here casts its one vote, so start Stage 3 at `--voter-offset 1`.)

## 3. Stage 2 — fan out the browser sessions

```bash
packages/step-cli/scripts/run-online-load-test.sh \
  --run-dir packages/step-cli/scripts/online-load-test-output/run \
  --voter-offset 1 \
  --concurrency 4 \
  --out-dir packages/step-cli/scripts/online-load-test-output/votes
```

This checks every dependency up front (portal, Keycloak realm, Hasura,
Playwright — each failure prints the exact command that fixes it), renders a
voter manifest, and runs one Playwright invocation with `--concurrency`
workers, each voter a fresh browser context casting a real ballot. Results
land in `results.csv` (`voter_id,status,duration_ms,ballot_id`), failure
traces under `traces/`, and a run `summary.json` under `--out-dir`.

Inspect a failed voter's trace with:

```bash
yarn --cwd packages/voting-portal playwright show-trace <path-to-trace.zip>
```

### How many parallel clients?

Each client is a real Chromium page doing WASM ballot encryption: budget
roughly 0.5 GB RAM and one core per client, on top of the compose stack. On
a dev laptop `min(cores − 2, free_GB / 0.5)` is the practical ceiling —
usually 4–8. Above ~8 clients the webpack dev server itself becomes the
bottleneck; point `--voting-portal-url` at a production build or a deployed
portal for bigger runs, or spread the load across machines (see
[Distributed runs](online-load-testing-design.md#distributed-runs-provision-once-load-from-several-machines) —
in short: copy the Stage-1 `--out-dir` to each machine and give each one a
disjoint `--voter-offset`/`--max-votes` slice).

## 4. Clean up: delete the election event

```bash
ELECTION_EVENT_ID=$(jq -r .election_event_id packages/step-cli/scripts/online-load-test-output/run/summary.json)
step-cli step delete-election-event --election-event-id "$ELECTION_EVENT_ID"
```

The command blocks and polls until the async teardown task finishes, so a
`Success!` means cleanup is actually done. It reuses the admin session Stage
1 left in `config/configuration.json` next to the `step-cli` binary; if that
token has expired, re-authenticate with the same `step config` call the
setup script makes (see `configure_as` in
`packages/step-cli/scripts/setup-telephone-load-test.sh`).

## Notes

- **Each voter casts exactly one vote.** Re-running Stage 2 against the
  *same* `--run-dir` re-uses the same (already-voted) voters, which the
  system correctly rejects as duplicate votes — not a failure. Either
  provision more voters than one run consumes and advance `--voter-offset`
  between runs, or re-run
  [Stage 1](#1-stage-1--provision-the-election-event-and-voters) for a fresh
  election event and voter set.
- **Language**: the flow matches English button labels and pins the portal
  to English via the login URL's `lang` parameter, so the run does not
  depend on browser locale.
- **Candidate choice** is deterministic (first eligible candidates up to
  each contest's minimum, at least one). Pass
  `--candidates-pattern <regex>` to restrict which candidates may be
  selected by visible name.
