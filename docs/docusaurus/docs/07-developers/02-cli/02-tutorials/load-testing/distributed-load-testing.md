---
id: distributed-load-testing
title: Distributed Load Testing
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Distributed Load Testing

Load one deployed server from several client machines at once, with the
same scripts as the [Load Testing Guide](load-testing-guide.md). Read that
guide first; this one only covers what changes when Stage 2 runs on more
than one machine. The examples use the ONLINE channel; TELEPHONE
differences are listed at the end.

## Roles

- **Provisioner** — one machine that runs Stage 1 and the cleanup. Your dev
  container is the simplest choice: set the deployment's public URLs and
  credentials in `layers.yaml` and run the usual commands. No install script
  needed.
- **Load clients** — N machines that run Stage 2 only. Each casts a
  disjoint slice of the voters.

| Needed on                      | Provisioner              | Load client                        |
|--------------------------------|--------------------------|------------------------------------|
| Repo checkout, same branch     | yes                      | yes                                |
| Python 3 + PyYAML              | yes                      | yes                                |
| `step-cli`                     | yes                      | no                                 |
| Playwright Chromium (ONLINE)   | no                       | yes                                |
| `ivr-cli` + Redis (TELEPHONE)  | no                       | yes                                |
| `layers.yaml`                  | full `setup:` section    | `online_run:` / `telephone_run:`   |
| Stage 1 run dir                | writes it                | a copy                             |
| Must reach                     | Hasura, Keycloak         | portal, Keycloak, Hasura `/healthz` |

- Size clients for the concurrency you want: one core and about 0.5 GB RAM
  per browser. Several small clients beat one big one; an overloaded client
  shows up as slow votes that are not the server's fault.
- Clients need working NTP (cloud images have it) so the shared start
  instant means the same thing everywhere.

## Step 0 — prepare the clients

- On every client, clone the repo at the branch under test and run the
  install script (idempotent, re-run after any failure):

  ```bash
  git clone --branch <branch> <repo-url> ~/step && cd ~/step
  packages/step-cli/scripts/install_load_client.sh                   # ONLINE
  packages/step-cli/scripts/install_load_client.sh --with-telephone  # TELEPHONE
  ```

- Or prepare one VM and snapshot it as an image for the others.

## Step 1 — provision once

- On the provisioner, in `layers.yaml`'s `setup:` section:
  - `endpoint_url`, `keycloak_url`, `voting_portal_url`: the deployment's
    **public** URLs, as a voter's browser reaches them. They are recorded in
    `summary.json` and used by every client; an internal hostname here fails
    every client's preflight.
  - `voting_channel` and `out_dir` for your channel.
  - `tenant_id`, `admin_portal_user/password`, `keycloak_client_secret`.
    Keep `new_tenants: 0` unless you have a Keycloak master-realm admin for
    this deployment.
  - `num_voters` ≥ clients × votes per client. A voter casts once.
- In the Stage 2 section, set what every client shares (`concurrency`,
  timeouts, `out_dir`) and leave `voter_offset`, `max_votes`/`max_calls`,
  `start_at` and `start_delay` at their defaults; they are set per client
  below.
- Run Stage 1:

  ```bash
  python3 packages/step-cli/scripts/setup_telephone_load_test.py
  ```

## Step 2 — sync the run dir and config to every client

- Two directories go from the provisioner to each client, both under
  `packages/step-cli/scripts/`:
  - `<channel>-load-test-output/run/` — `tenants.json` plus each tenant's
    `summary.json` and voters CSV. Small text files; the scripts tolerate a
    different absolute path.
  - `telephone-load-test-inputs/config/` — the `layers.yaml`. Clients only
    read the Stage 2 section, so the `setup:` credentials may be blanked in
    the copy.
- Each client then gets its own **disjoint voter slice** via
  `voter_offset` and `max_votes` (`max_calls` for TELEPHONE). Overlapping
  slices are the one mistake that corrupts a run: the second attempt on a
  shared voter is rejected as a duplicate and counted as a failure.
- Example, four clients with 500 votes each:

  ```bash
  cd ~/step/packages/step-cli/scripts
  CLIENTS=(loadclient-0 loadclient-1 loadclient-2 loadclient-3)
  PER_CLIENT=500
  for i in "${!CLIENTS[@]}"; do
    host=${CLIENTS[$i]}
    rsync -a online-load-test-output/run/ "$host":step/packages/step-cli/scripts/online-load-test-output/run/
    rsync -a telephone-load-test-inputs/config/ "$host":step/packages/step-cli/scripts/telephone-load-test-inputs/config/
    ssh "$host" "sed -i '/^online_run:/,\$ { s/^  voter_offset:.*/  voter_offset: $((i * PER_CLIENT))/; s/^  max_votes:.*/  max_votes: $PER_CLIENT/ }' \
      step/packages/step-cli/scripts/telephone-load-test-inputs/config/layers.yaml"
  done
  ```

  (The `sed` range limits the edit to the `online_run:` block, since
  `voter_offset` also exists under `telephone_run:`.)

## Step 3 — launch all clients

- `LOAD_TEST_START_AT`: a shared UTC instant. Every client preflights and
  renders its manifest immediately, then holds until that instant. A
  misconfigured client fails before the run, not during it.
- `LOAD_TEST_START_DELAY`: seconds added per client. Different values ramp
  the load up in steps; the same value (or none) starts everyone together.
- Both override `start_at` / `start_delay` in `layers.yaml`, so the config
  stays identical across clients.

  ```bash
  START_AT=$(date -u -d '+3 minutes' +%Y-%m-%dT%H:%M:%SZ)
  RAMP_STEP=60   # one more client every minute; 0 = all at once
  for i in "${!CLIENTS[@]}"; do
    ssh "${CLIENTS[$i]}" "cd step && LOAD_TEST_START_AT=$START_AT LOAD_TEST_START_DELAY=$((i * RAMP_STEP)) \
      nohup python3 packages/step-cli/scripts/run_online_load_test.py > load.log 2>&1 &"
  done
  ```

- Confirm every client is armed before the instant arrives. Each logs
  `Holding until <time>` once its preflight passed, or `Error:` with the fix:

  ```bash
  for host in "${CLIENTS[@]}"; do echo "== $host"; ssh "$host" "grep -E 'Holding until|Error' step/load.log"; done
  ```

- `tail -f load.log` on a client follows the run. It ends with
  `Done: <cast>/<total>` and the `summary.json` path.

## Step 4 — collect and aggregate

- Pull every client's Stage 2 `out_dir` into one directory per client and
  merge:

  ```bash
  mkdir -p collected
  for host in "${CLIENTS[@]}"; do
    rsync -a "$host":step/packages/step-cli/scripts/online-load-test-output/votes/ "collected/$host/"
  done
  python3 ~/step/packages/step-cli/scripts/aggregate_online_load_test.py collected/
  ```

- Output: a per-client table, a merged `results.csv` with a `client`
  column, and a `summary.json` with totals, the span from the earliest
  client start to the latest finish, `cast_per_second` over that span,
  p50/p95/max vote durations, each client's `start_at`/`start_delay_secs`,
  and a warning for any voter id two clients both attempted.
- Failure traces stay on the clients under each tenant's `traces/`.

## Step 5 — clean up

- From the provisioner, as in the main guide. Clients hold nothing that
  needs cleaning beyond their output directories.

## TELEPHONE differences

- Clients: `install_load_client.sh --with-telephone`, and
  `telephone_run.valkey_url: redis://127.0.0.1:6379` in the copied config.
  Sessions are per call and local to each client; nothing is shared.
- Copy the DTMF template along with the config if it is not the tracked
  example.
- IVR client secrets (`telephone_run.keycloak_ivr_*_client_secret`) come
  from the event realm Stage 1 created; put them in the config before
  copying it.
- Slice with `voter_offset` and `max_calls` inside the `telephone_run:`
  block: use the `sed` range `/^telephone_run:/,/^online_run:/`.
- Launch `run_telephone_load_test.py` instead; `LOAD_TEST_START_AT` and
  `LOAD_TEST_START_DELAY` work the same way.
- The aggregator currently reads the ONLINE `results.csv` layout; for
  TELEPHONE, sum each client's `summary.json` totals by hand.

## On a real deployment

- Stage 2 preflights `<hasura_url>/healthz` from the clients; expose it or
  set `online_run.hasura_url` to a reachable Hasura.
- Cloudflare-fronted portals may challenge headless Chromium; allow-list
  the clients' egress IPs.
- Creating tenants needs a Keycloak master-realm admin; without one, use an
  existing `setup.tenant_id` and its `api-key-client` secret.
