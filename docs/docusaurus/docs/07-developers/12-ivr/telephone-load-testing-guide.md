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

Every command below writes its output under
`packages/step-cli/scripts/telephone-load-test-output/` (gitignored) rather
than `/tmp`, so a run survives a container restart and stays easy to find
between steps.

## Configuration

`setup_telephone_load_test.py`, `run_telephone_load_test.py` and
`run_online_load_test.py` take **no command-line arguments** — every setting
lives in
[`packages/step-cli/scripts/telephone-load-test-inputs/layers.yaml`](https://github.com/sequentech/step/blob/main/packages/step-cli/scripts/telephone-load-test-inputs/layers.yaml),
under the `setup:` / `telephone_run:` / `online_run:` sections respectively.
Edit that file before each run instead of passing flags. A field left as
`null` falls back to the environment variable named in the comment beside
it (already exported in this repo's devcontainer).

## Prerequisites

- The `keycloak`, `graphql-engine` (Hasura), `trustee1` and `trustee2`
  containers must be running (`docker ps`; `docker start trustee1 trustee2`
  if they aren't — the keys ceremony hangs without them).
- Python 3 with PyYAML (already available in the devenv shell).

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

`layers.yaml`'s `setup:` section already points at the tracked example
election event with `voting_channel: TELEPHONE` and 20 voters — run it as-is:

```bash
python3 packages/step-cli/scripts/setup_telephone_load_test.py
```

This imports the tracked example election event, generates 20 voters (all
placed in the same area, with unique numeric username/PIN/date-of-birth
each), runs the keys ceremony, publishes, and opens `TELEPHONE` voting.
Outputs land in `setup.out_dir` (`telephone-load-test-output/run` by
default): `summary.json` and the voters CSV.

Each run appends a random 5-character suffix to the election event's alias
(e.g. `TECUMSEH - DATAFIX Test - K3F9Q`) — the admin portal's election event
list renders alias, not name, so that's the field that needs the suffix to
actually be visible there. The full alias is printed at the end of the run
and recorded as `election_event_alias` in `summary.json`.

Using your own election event JSON instead of the tracked example works the
same way — just point `setup.election_event_json` at it in `layers.yaml`. If
it has more than one area, set `setup.voter_area_name` to pick which one
voters are generated into (`null` defaults to the first area).

## 2. Get a DTMF template

`packages/step-cli/scripts/dtmf-template.example.txt` — the default for
`telephone_run.dtmf_template` — is already captured for the tracked example
election's first area and works as-is — skip to
[step 3](#3-stage-2--fan-out-the-simulated-calls).

If you're testing a **different** election event JSON or
`setup.voter_area_name`, capture a new template: the ballot portion
(candidate numbering, confirm/submit keys) depends on that election's
contests. First get a `phone_config.json` and a running session store by
running Stage 2 once with any existing template (the calls themselves may
fail if the template doesn't match your election — that's fine, the side
effects are what you need):

```bash
python3 packages/step-cli/scripts/run_telephone_load_test.py
```

Then drive one call by hand, noting every prompt and keystroke:

```bash
PHONE_CONFIG_PATH=packages/step-cli/scripts/telephone-load-test-output/calls/phone_config.json \
beyond/packages/rust-local-target/release/ivr-cli \
  --bundle dev --system-number +111111111111 \
  --number +15550000000 --show-internal-state
```

Log in with the first row of the voters CSV. Transcribe the full keystroke
sequence into a copy of `dtmf-template.example.txt` (and point
`telephone_run.dtmf_template` at your copy), replacing the identifier/PIN
lines with `{{VOTER_ID}}`/`{{PIN}}` or `{{DOB}}`/`{{PIN}}` — check which
fields this realm's IVR flow expects first:

```bash
TOKEN=$(curl -sf -X POST "http://keycloak:8090/realms/<realm>/protocol/openid-connect/token" \
  -d grant_type=client_credentials -d client_id=ivr-service -d client_secret=<KEYCLOAK_IVR_SERVICE_CLIENT_SECRET> \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["access_token"])')
curl -sf -H "Authorization: Bearer $TOKEN" "http://keycloak:8090/realms/<realm>/ivr-config"
```

(`<realm>` is in `summary.json`; the client secret is
`telephone_run.keycloak_ivr_service_client_secret` in `layers.yaml` — or
`.devcontainer/.env.development` for the local devcontainer stack.)

## 3. Stage 2 — fan out the simulated calls

```bash
python3 packages/step-cli/scripts/run_telephone_load_test.py
```

This generates `phone_config.json`, renders one DTMF input file per voter
from the template, and fans out `telephone_run.concurrency` parallel
`ivr-cli` calls. A local `valkey` container is started automatically as the
session store if none is reachable (reused across runs; disable with
`telephone_run.start_valkey: false`). Results land in `results.csv` and
per-call logs under `telephone_run.out_dir` (`telephone-load-test-output/calls`
by default).

## 4. Clean up: delete the election event

```bash
ELECTION_EVENT_ID=$(python3 -c 'import json; print(json.load(open("packages/step-cli/scripts/telephone-load-test-output/run/summary.json"))["election_event_id"])')
step-cli step delete-election-event --election-event-id "$ELECTION_EVENT_ID"
```

This calls the `delete_election_event` GraphQL mutation, which queues an
async task tearing down the election event's Postgres/Hasura rows, its
Keycloak realm, and its ImmuDB and document-store data — the command blocks
and polls until that task finishes (or fails/times out after 5 minutes), so
a `Success!` means cleanup is actually done, not just queued.

It reuses the session `step-cli` already has in `config/configuration.json`
next to the binary (left there authenticated as admin by
[Stage 1](#1-stage-1--provision-the-election-event-and-voters)). If that
token has expired, re-authenticate first with the same `step config` call
the setup script makes (see `configure_as` in
`packages/step-cli/scripts/setup_telephone_load_test.py`).

## Notes

- **Each voter casts exactly one vote.** Re-running Stage 2 against the
  *same* `telephone_run.run_dir` re-uses the same (already-voted) voters —
  every call logs in successfully but reports "voting is now complete"
  without casting a ballot, since there's nothing left for that voter to
  vote on. This is the system correctly rejecting a duplicate vote, not a
  failure. To place a fresh batch of calls, re-run
  [Stage 1](#1-stage-1--provision-the-election-event-and-voters) to
  provision a new election event and voter set.
- **Re-running after a dev container restart:** the auto-started `valkey`
  container is reused if it's already there (even if stopped), so you don't
  need to remove it manually between runs.
