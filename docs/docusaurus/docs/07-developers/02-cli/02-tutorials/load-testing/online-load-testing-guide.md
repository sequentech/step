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

## Configuration

`setup_telephone_load_test.py`, `run_telephone_load_test.py` and
`run_online_load_test.py` take **no command-line arguments** — every setting
lives in `packages/step-cli/scripts/telephone-load-test-inputs/config/layers.yaml`,
under the `setup:` / `telephone_run:` / `online_run:` sections respectively.
`config/` is gitignored (it holds real per-server credentials); copy the
tracked
[`layers.yaml.example`](https://github.com/sequentech/step/blob/main/packages/step-cli/scripts/telephone-load-test-inputs/layers.yaml.example)
template there first — see the
[telephone guide's Configuration section](../../../12-ivr/telephone-load-testing-guide.md#configuration)
for the exact commands. Edit that copy before each run instead of passing
flags — in particular, set
`setup.voting_channel: ONLINE` and `setup.out_dir:
online-load-test-output/run` before Stage 1 (the tracked example already
ships with `TELEPHONE`/`telephone-load-test-output/run` as the default,
shared with the [telephone load test](../../../12-ivr/telephone-load-testing-guide.md)).
A field left as `null` falls back to the environment variable named in the
comment beside it (already exported in this repo's devcontainer).

## Prerequisites

- The `keycloak`, `graphql-engine` (Hasura), `trustee1` and `trustee2`
  containers must be running (`docker ps`; `docker start trustee1 trustee2`
  if they aren't — the keys ceremony hangs without them).
- The voting portal on port 3000. Check first, then start it if needed:

  ```bash
  curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:3000/ || \
    (cd packages && yarn start:voting-portal)
  ```

- Python 3 with PyYAML (already available in the devenv shell).

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

Set `setup.voting_channel: ONLINE` and `setup.out_dir:
online-load-test-output/run` in `layers.yaml` (the same provisioning script
the telephone load test uses, with the ONLINE channel opened instead), then:

```bash
python3 packages/step-cli/scripts/setup_telephone_load_test.py
```

This imports the tracked example election event, generates 20 voters (all
placed in the same area, with unique numeric username/PIN each), runs the
keys ceremony, publishes, and opens `ONLINE` voting. Outputs land in
`setup.out_dir`: a top-level `tenants.json` index, and one
`tenant-<tenant_id>/` subdirectory per provisioned tenant holding that
tenant's own `summary.json` (including the portal `login_url`) and voters
CSV. By default there's exactly one tenant — see the equivalent note in the
[telephone guide's Stage 1](../../../12-ivr/telephone-load-testing-guide.md#1-stage-1--provision-the-election-event-and-voters)
for `setup.new_tenants`, which works the same way here.

## 2. Smoke-test one voter (optional but recommended)

Before fanning out, watch a single voter cast a ballot in a headed browser —
this catches a wrong URL, a closed channel, or changed selectors immediately.
Find your tenant's output subdirectory first:

```bash
TENANT_DIR=$(python3 -c 'import json; run_dir="packages/step-cli/scripts/online-load-test-output/run"; print(f"{run_dir}/{json.load(open(f\"{run_dir}/tenants.json\"))[\"tenants\"][0][\"dir\"]}")')
```

```bash
LOGIN_URL=$(python3 -c "import json; print(json.load(open('$TENANT_DIR/summary.json'))['login_url'])")
VOTER_USERNAME=$(awk -F, 'NR==2 {print $1}' "$TENANT_DIR/voters_20.csv")
VOTER_PASSWORD=$(awk -F, 'NR==2 {print $3}' "$TENANT_DIR/voters_20.csv")
VOTER_DATE_OF_BIRTH=$(awk -F, 'NR==2 {print $6}' "$TENANT_DIR/voters_20.csv")
LOGIN_URL="$LOGIN_URL" VOTER_USERNAME="$VOTER_USERNAME" VOTER_PASSWORD="$VOTER_PASSWORD" \
  VOTER_DATE_OF_BIRTH="$VOTER_DATE_OF_BIRTH" \
  yarn --cwd packages/voting-portal playwright test --config playwright.load.config.ts --headed
```

(Check the `username`/`password`/`dateOfBirth` column positions against the
CSV header — they depend on the `fields` list in the run's
`external_config.json`; `VOTER_DATE_OF_BIRTH` is only needed if the realm's
login form actually asks for a date of birth. The voter used here casts its
one vote, so start Stage 3 at `online_run.voter_offset: 1`.)

## 3. Stage 2 — fan out the browser sessions

Set `online_run.voter_offset: 1` in `layers.yaml` if you ran the smoke test
above, then:

```bash
python3 packages/step-cli/scripts/run_online_load_test.py
```

This checks every dependency up front (portal, Keycloak realm, Hasura,
Playwright — each failure prints the exact command that fixes it), then runs
once per tenant `tenants.json` lists (a single tenant by default), casting
every one of that tenant's voters. Per-tenant results land in
`tenant-<tenant_id>/results.csv` (`voter_id,status,duration_ms,ballot_id`)
under `online_run.out_dir`, with failure traces under that tenant's
`traces/`; a top-level `summary.json` aggregates totals across every tenant.

Inspect a failed voter's trace with:

```bash
yarn --cwd packages/voting-portal playwright show-trace <path-to-trace.zip>
```

### How many parallel clients?

Each client is a real Chromium page doing WASM ballot encryption: budget
roughly 0.5 GB RAM and one core per client, on top of the compose stack. On
a dev laptop `min(cores − 2, free_GB / 0.5)` is the practical ceiling —
usually 4–8. Above ~8 clients the webpack dev server itself becomes the
bottleneck; point `online_run.voting_portal_url` at a production build or a
deployed portal for bigger runs, or spread the load across machines (see
[Distributed runs](online-load-testing-design.md#distributed-runs-provision-once-load-from-several-machines) —
in short: copy the Stage-1 `setup.out_dir` to each machine and give each one a
disjoint `online_run.voter_offset`/`online_run.max_votes` slice).

## 4. Clean up: delete the election event(s)

One election event per tenant `tenants.json` lists — see the equivalent
[telephone guide's cleanup step](../../../12-ivr/telephone-load-testing-guide.md#4-clean-up-delete-the-election-events)
for the full per-tenant loop (list tenant/election-event id pairs, then
`step config` + `delete-election-event` for each). The command blocks and
polls until the async teardown task finishes, so a `Success!` means cleanup
is actually done.

## Notes

- **Each voter casts exactly one vote.** Re-running Stage 2 against the
  *same* `online_run.run_dir` re-uses the same (already-voted) voters, which
  the system correctly rejects as duplicate votes — not a failure. Either
  provision more voters than one run consumes and advance
  `online_run.voter_offset` between runs, or re-run
  [Stage 1](#1-stage-1--provision-the-election-event-and-voters) for a fresh
  election event and voter set.
- **Language**: the flow matches English button labels and pins the portal
  to English via the login URL's `lang` parameter, so the run does not
  depend on browser locale.
- **Candidate choice** is deterministic (first eligible candidates up to
  each contest's minimum, at least one). Set
  `online_run.candidates_pattern` to a regular expression to restrict which
  candidates may be selected by visible name.
