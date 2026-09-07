---
id: load-testing-guide
title: Load Testing Guide
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Load Testing Guide

Provision an election event with many voters, then cast votes against it at
scale through one of two channels:

- **TELEPHONE** — simulated IVR calls, one `ivr-cli` process per call.
- **ONLINE** — real headless browsers driven through the voting portal by
  Playwright.

Both channels share the same provisioning (Stage 1) and cleanup; only the
Stage 2 script and its `layers.yaml` section differ. To load one deployment
from several machines at once, see the
[Distributed Load Testing](distributed-load-testing.md) guide. How the
tooling works internally:
[Telephone design](../../../12-ivr/telephone-load-testing-design.md),
[Online design](online-load-testing-design.md).

## At a glance

|                        | TELEPHONE                            | ONLINE                              |
|------------------------|--------------------------------------|-------------------------------------|
| Stage 1 (provision)    | `setup_telephone_load_test.py`       | same script                         |
| `setup.voting_channel` | `TELEPHONE`                          | `ONLINE`                            |
| Stage 2 (load)         | `run_telephone_load_test.py`         | `run_online_load_test.py`           |
| Stage 2 config section | `telephone_run:`                     | `online_run:`                       |
| Extra tooling          | `ivr-cli`, a Redis/Valkey session store | Playwright + Chromium            |
| Cleanup                | `cleanup_telephone_load_test.py`     | same script                         |

All scripts live in `packages/step-cli/scripts/`.

## Configuration

- The scripts take **no command-line arguments** (except cleanup's safety
  flags). Every setting lives in
  `packages/step-cli/scripts/telephone-load-test-inputs/config/layers.yaml`.
- `config/` is gitignored because it holds real credentials. Create it from
  the tracked template:

  ```bash
  mkdir -p packages/step-cli/scripts/telephone-load-test-inputs/config
  cp packages/step-cli/scripts/telephone-load-test-inputs/layers.yaml.example \
    packages/step-cli/scripts/telephone-load-test-inputs/config/layers.yaml
  ```

- Sections: `setup:` (Stage 1 and cleanup), `telephone_run:`,
  `online_run:`. Every field is commented in the template.
- A field left as `****` or `null` falls back to the environment variable
  named beside it. Those variables are exported inside the dev container, so
  the unedited template works there. Outside it, fill the fields in.
- Channel-specific fields in `setup:`:
  - `voting_channel`: `TELEPHONE` or `ONLINE`.
  - `out_dir`: keep in sync with `telephone_run.run_dir` or
    `online_run.run_dir`. Convention: `telephone-load-test-output/run` or
    `online-load-test-output/run`.
  - `voting_portal_url`: ONLINE only, the URL a voter's browser uses.
- All outputs land under `packages/step-cli/scripts/*-load-test-output/`
  (gitignored), so they survive a container restart.

## Environment

Two ways to get the tooling. Pick one per machine.

### A. Inside the dev container

- Already there: `cargo`, Python with PyYAML, Yarn, the environment
  variables `layers.yaml` falls back to, and the whole stack.
- Containers that must be running: `keycloak`, `graphql-engine`,
  `trustee1`, `trustee2` (`docker ps`; `docker start trustee1 trustee2`).
  The keys ceremony hangs without the trustees.
- ONLINE only: the voting portal on port 3000. Check before starting a
  second one:

  ```bash
  curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:3000/ || (cd packages && yarn start:voting-portal)
  ```

- Build what your channel needs:

  ```bash
  # both channels
  cd packages && CARGO_TARGET_DIR=/workspaces/step/packages/step-cli/rust-local-target cargo build --release -p step-cli && cd ..
  export PATH="/workspaces/step/packages/step-cli/rust-local-target/release:$PATH"
  # TELEPHONE
  cd beyond/packages && CARGO_TARGET_DIR=/workspaces/step/beyond/packages/rust-local-target cargo build --release -p ivr-cli && cd ../..
  # ONLINE
  cd packages && yarn && cd voting-portal && yarn playwright install chromium && cd ../..
  ```

- The dev container can also target a **remote** deployment: put the
  deployment's public URLs and credentials in `layers.yaml` and run the
  same commands. This is the easiest way to run Stage 1 and cleanup against
  a remote server. Nothing from section B is needed.

### B. Outside the dev container

- Any Debian/Ubuntu machine with the repo checked out at the branch under
  test. One script installs everything, and it is idempotent: re-run it
  after a failure and it only redoes what is missing.

  ```bash
  packages/step-cli/scripts/install_load_client.sh                     # ONLINE Stage 2 only
  packages/step-cli/scripts/install_load_client.sh --with-provisioner  # + Rust and step-cli (Stage 1, cleanup)
  packages/step-cli/scripts/install_load_client.sh --with-telephone    # + ivr-cli and a local Redis (TELEPHONE Stage 2)
  packages/step-cli/scripts/install_load_client.sh --all
  ```

- What it installs: Python 3 with PyYAML, Node.js 20, the standalone
  Playwright package under `packages/voting-portal/test/load/` and Chromium
  with its system libraries. With the flags: the Rust toolchain and build
  dependencies, `step-cli`, the `beyond` submodule (needs git access to
  that private repository), `ivr-cli`, and `redis-server`.
- It creates `config/layers.yaml` from the template if missing. There are
  no environment-variable fallbacks outside the dev container, so fill in
  every `****` field.
- After `--with-provisioner`, add `packages/step-cli/rust-local-target/release`
  to `PATH` (the script prints the line).
- After `--with-telephone`, set `telephone_run.valkey_url: redis://127.0.0.1:6379`.

## 1. Stage 1 — provision the election event and voters

```bash
python3 packages/step-cli/scripts/setup_telephone_load_test.py
```

- Imports the election event (`setup.election_event_json`, the tracked
  example by default), generates `setup.num_voters` voters with numeric
  DTMF-safe credentials in one area, imports them, runs the keys ceremony,
  publishes, and opens `setup.voting_channel`.
- Outputs under `setup.out_dir`: `tenants.json`, and per tenant a
  `tenant-<id>/` directory with `summary.json` (ids, realm, URLs, login
  URL) and the voters CSV.
- Tenants: by default the event goes into `setup.tenant_id`.
  `setup.new_tenants: N` creates N fresh tenants (cloning `tenant_id`'s
  Keycloak/roles config and trustees) and provisions each;
  `setup.use_existing_tenants` adds tenants a previous run created. Both
  need `setup.keycloak_admin_user/password` (Keycloak master realm) to look
  up each tenant's `api-key-client` secret.
- Keys ceremony: `setup.ceremony_policy: AUTOMATIC` (default) needs the
  tenant's trustees to have live `braid` services. `MANUAL` logs in as
  `trustee1`/`trustee2` instead.
- Each run appends a random 5-character suffix to the event alias, so it is
  easy to find in the admin portal. The alias is in `summary.json`.
- Own election event JSON: point `setup.election_event_json` at it; with
  several areas set `setup.voter_area_name`.
- TELEPHONE: the import creates a new Keycloak realm
  (`summary.json`'s `keycloak_realm`) with its own `ivr-service` and
  `ivr-voting` clients. Fetch their secrets from **that** realm after every
  Stage 1 run (Keycloak admin console → realm → Clients → Credentials) into
  `telephone_run.keycloak_ivr_service_client_secret` and
  `keycloak_ivr_voting_client_secret`. Secrets from a previous run's event
  do not work. With several tenants use `telephone_run.tenant_ivr_secrets`.

## 2. Stage 2 — TELEPHONE: fan out simulated calls

- DTMF template: `dtmf-template.example.txt` (the default
  `telephone_run.dtmf_template`) matches the tracked example election's
  first area. Skip to the run command unless you changed the election JSON
  or area.
- Capturing a new template:
  - Run Stage 2 once with any template to get a `phone_config.json` and a
    session store (the calls may fail; the side effects are what you need).
  - Drive one call by hand with the first voter of the CSV and note every
    prompt and keystroke:

    ```bash
    PHONE_CONFIG_PATH=packages/step-cli/scripts/telephone-load-test-output/calls/phone_config.json \
    beyond/packages/rust-local-target/release/ivr-cli --bundle dev --system-number +111111111111 \
      --number +15550000000 --show-internal-state
    ```

  - Copy the keystrokes into a copy of the template, replacing the
    identifier and PIN with `{{VOTER_ID}}`/`{{PIN}}` or `{{DOB}}`/`{{PIN}}`.
    Which pair the realm expects is in `GET <keycloak>/realms/<realm>/ivr-config`
    (authenticate with the `ivr-service` client credentials). Point
    `telephone_run.dtmf_template` at your copy.
- Run:

  ```bash
  python3 packages/step-cli/scripts/run_telephone_load_test.py
  ```

- Generates `phone_config.json`, renders one DTMF input file per voter, and
  places `telephone_run.concurrency` parallel `ivr-cli` calls per tenant.
- Session store: uses `telephone_run.valkey_url` if set; otherwise
  `127.0.0.1:6379`; otherwise, inside the dev container, starts a `valkey`
  container on the compose network (reused across runs; disable with
  `start_valkey: false`).

## 2. Stage 2 — ONLINE: fan out browser sessions

- Optional smoke test: watch one voter cast a ballot in a headed browser
  first. It catches a wrong URL, a closed channel or changed selectors
  immediately. The voter used is then spent, so set
  `online_run.voter_offset: 1` afterwards.

  ```bash
  TENANT_DIR=packages/step-cli/scripts/online-load-test-output/run/tenant-<tenant_id>
  LOGIN_URL=$(python3 -c "import json; print(json.load(open('$TENANT_DIR/summary.json'))['login_url'])")
  VOTER_USERNAME=$(awk -F, 'NR==2 {print $1}' "$TENANT_DIR/voters_20.csv")
  VOTER_PASSWORD=$(awk -F, 'NR==2 {print $3}' "$TENANT_DIR/voters_20.csv")
  VOTER_DATE_OF_BIRTH=$(awk -F, 'NR==2 {print $6}' "$TENANT_DIR/voters_20.csv")
  LOGIN_URL="$LOGIN_URL" VOTER_USERNAME="$VOTER_USERNAME" VOTER_PASSWORD="$VOTER_PASSWORD" VOTER_DATE_OF_BIRTH="$VOTER_DATE_OF_BIRTH" \
    yarn --cwd packages/voting-portal playwright test --config test/load/playwright.config.ts --headed
  ```

  (Check the column positions against the CSV header; `VOTER_DATE_OF_BIRTH`
  is only needed if the realm's login form asks for it.)
- Run:

  ```bash
  python3 packages/step-cli/scripts/run_online_load_test.py
  ```

- Preflights portal, Keycloak realm, Hasura and Playwright (each failure
  prints the fix), then casts every voter of every tenant with
  `online_run.concurrency` parallel browsers.
- Playwright is looked up under `packages/voting-portal/test/load/node_modules`
  first, then the Yarn workspace. `PLAYWRIGHT_BIN` overrides it.
- Sizing: each browser is a real Chromium doing WASM encryption. Budget one
  core and about 0.5 GB RAM per parallel client. On a dev laptop 4 to 8 is
  the practical ceiling, and above about 8 the webpack dev server itself
  becomes the bottleneck: point `online_run.voting_portal_url` at a
  production build or a deployed portal for bigger runs.
- Failure traces: `<out_dir>/tenant-<id>/traces/`. Open one with
  `playwright show-trace <trace.zip>` (from `packages/voting-portal/test/load`
  or via `yarn --cwd packages/voting-portal playwright show-trace`).

## 3. Results

- Per tenant, under `<out_dir>/tenant-<id>/`: `results.csv`
  (TELEPHONE: `voter_id,exit_code,ballot_cast`; ONLINE:
  `voter_id,status,duration_ms,ballot_id`), per-call logs or traces, and a
  `summary.json`.
- Run-wide `<out_dir>/summary.json`: totals across every tenant plus
  `started_at`, `finished_at`, `elapsed_secs` and `cast_per_second`. All
  tenants share one deployment and one database, so throughput is only
  meaningful summed over all of them.
- The script exits non-zero if any voter failed.

## 4. Clean up

```bash
python3 packages/step-cli/scripts/cleanup_telephone_load_test.py
```

- Reads Stage 1's `tenants.json` and, for each tenant, deletes the election
  event (`delete-election-event`, which blocks until the async teardown of
  Postgres rows, Keycloak realm, ImmuDB and documents is done), then deletes
  every non-bootstrap tenant (`delete-tenant`). `setup.tenant_id` itself is
  never deleted.
- Flags scope how destructive a run is:
  - `--events-only`: delete election events, keep every tenant.
  - `--new-tenants-only`: also delete only the tenants marked
    `"source": "new"` in `tenants.json` (created by `setup.new_tenants`).
- Deleting a tenant needs the `tenant-delete` role in the bootstrap tenant's
  realm, assigned to `setup.admin_portal_user`. The dev container's realm
  import has it; deployments provisioned by `beyond`'s `client-setup` chart
  do not, so add it by hand first (Keycloak admin console → bootstrap tenant
  realm → Realm roles).
- By hand, per tenant:

  ```bash
  step-cli step config --tenant-id "$TENANT_ID" --endpoint-url ... --keycloak-url ... \
    --keycloak-user "$ADMIN_PORTAL_USER" --keycloak-password "$ADMIN_PORTAL_PASSWORD" \
    --keycloak-client-id api-key-client --keycloak-client-secret "$API_KEY_CLIENT_SECRET"
  step-cli step delete-election-event --election-event-id "$ELECTION_EVENT_ID"
  step-cli step delete-tenant --tenant-id "$TENANT_ID"   # authenticated as the bootstrap tenant
  ```

## Notes

- **Each voter casts exactly one vote.** Re-running Stage 2 on the same run
  dir reuses spent voters; the system rejects the duplicate votes, which is
  correct, not a failure. Either provision more voters than one run needs
  and advance `voter_offset` between runs, or re-run Stage 1.
- ONLINE pins the portal to English via the login URL's `lang` parameter
  and matches English labels. Candidate choice is deterministic (first
  eligible candidates up to each contest's minimum); restrict it with
  `online_run.candidates_pattern`.
- TELEPHONE: the auto-started `valkey` container is reused across dev
  container restarts, even if stopped. That also holds when its URL was
  pinned in `telephone_run.valkey_url`: the script restarts or recreates a
  container by its own name. A `valkey_url` pointing at a different host is
  trusted as-is and fails fast if unreachable.
