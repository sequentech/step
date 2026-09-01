---
id: telephone-load-testing-guide
title: Telephone Load Testing Guide
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Telephone Load Testing Guide

Step-by-step instructions for running the telephone (IVR/DTMF) load test:
provisioning an election event with many voters, then driving many
simulated phone calls against it. See
[Telephone Load Testing — Design](telephone-load-testing-design.md) for how
this works under the hood.

All commands below assume a terminal opened **inside the dev container**
(VS Code Dev Containers or GitHub Codespaces) — that's where `cargo`,
`step-cli`'s dependencies (Hasura/Keycloak), and the `trustee1`/`trustee2`
containers are reachable.

## Prerequisites

- The `keycloak`, `graphql-engine` (Hasura), `trustee1` and `trustee2`
  containers must be running (`docker ps`; `docker start trustee1 trustee2`
  if they aren't — the keys ceremony hangs without them).
- `jq` (already installed in the dev container).

## 0. Build the CLIs

Both scripts look for their binary on `PATH` first, falling back to the
default release build path — so a release build is enough, no need to keep
using `cargo run`:

```bash
cd packages && CARGO_TARGET_DIR=/workspaces/step/packages/step-cli/rust-local-target cargo build --release -p step-cli
cd ../beyond/packages && CARGO_TARGET_DIR=/workspaces/step/beyond/packages/rust-local-target cargo build --release -p ivr-cli
cd /workspaces/step
export PATH="/workspaces/step/packages/step-cli/rust-local-target/release:$PATH"
```

## 1. Stage 1 — provision the election event and voters

```bash
packages/step-cli/scripts/setup-telephone-load-test.sh \
  --election-event-json packages/step-cli/scripts/telephone-load-test-inputs/election-event.json \
  --num-voters 20 \
  --out-dir /tmp/telephone-load-test-run
```

This imports the tracked example election event, generates 20 voters (all
placed in the same area, with unique numeric username/PIN/date-of-birth
each), runs the keys ceremony, publishes, and opens `TELEPHONE` voting.
Outputs land in `--out-dir`: `summary.json` and the voters CSV.

Using your own election event JSON instead of the tracked example works the
same way — just point `--election-event-json` at it. If it has more than one
area, pass `--voter-area-name <name>` to pick which one voters are generated
into (defaults to the first area).

## 2. Get a DTMF template

`packages/step-cli/scripts/dtmf-template.example.txt` is already captured
for the tracked example election's first area and works as-is — skip to
[step 3](#3-stage-2--fan-out-the-simulated-calls).

If you're testing a **different** election event JSON or `--voter-area-name`,
capture a new template: the ballot portion (candidate numbering, confirm/
submit keys) depends on that election's contests. First get a
`phone_config.json` and a running session store by running Stage 2 once with
any existing template (the calls themselves may fail if the template doesn't
match your election — that's fine, the side effects are what you need):

```bash
packages/step-cli/scripts/run-telephone-load-test.sh \
  --run-dir /tmp/telephone-load-test-run \
  --dtmf-template packages/step-cli/scripts/dtmf-template.example.txt \
  --out-dir /tmp/telephone-load-test-calls
```

Then drive one call by hand, noting every prompt and keystroke:

```bash
PHONE_CONFIG_PATH=/tmp/telephone-load-test-calls/phone_config.json \
beyond/packages/rust-local-target/release/ivr-cli \
  --bundle dev --system-number +111111111111 \
  --number +15550000000 --show-internal-state
```

Log in with the first row of the voters CSV. Transcribe the full keystroke
sequence into a copy of `dtmf-template.example.txt`, replacing the
identifier/PIN lines with `{{VOTER_ID}}`/`{{PIN}}` or `{{DOB}}`/`{{PIN}}` —
check which fields this realm's IVR flow expects first:

```bash
TOKEN=$(curl -sf -X POST "http://keycloak:8090/realms/<realm>/protocol/openid-connect/token" \
  -d grant_type=client_credentials -d client_id=ivr-service -d client_secret=<KEYCLOAK_IVR_SERVICE_CLIENT_SECRET> \
  | jq -r '.access_token')
curl -sf -H "Authorization: Bearer $TOKEN" "http://keycloak:8090/realms/<realm>/ivr-config"
```

(`<realm>` and the client secret are in `summary.json` and
`.devcontainer/.env.development` respectively.)

## 3. Stage 2 — fan out the simulated calls

```bash
packages/step-cli/scripts/run-telephone-load-test.sh \
  --run-dir /tmp/telephone-load-test-run \
  --dtmf-template packages/step-cli/scripts/dtmf-template.example.txt \
  --concurrency 10
```

This generates `phone_config.json`, renders one DTMF input file per voter
from the template, and fans out `--concurrency` parallel `ivr-cli` calls. A
local `valkey` container is started automatically as the session store if
none is reachable (reused across runs; disable with `--no-start-valkey`).
Results land in `results.csv` and per-call logs under `--out-dir` (defaults
to a fresh temp dir).

## Notes

- **Each voter casts exactly one vote.** Re-running Stage 2 against the
  *same* `--run-dir` re-uses the same (already-voted) voters — every call
  logs in successfully but reports "voting is now complete" without casting
  a ballot, since there's nothing left for that voter to vote on. This is
  the system correctly rejecting a duplicate vote, not a failure. To place a
  fresh batch of calls, re-run [Stage 1](#1-stage-1--provision-the-election-event-and-voters)
  to provision a new election event and voter set.
- **Re-running after a dev container restart:** the auto-started `valkey`
  container is reused if it's already there (even if stopped), so you don't
  need to remove it manually between runs.
