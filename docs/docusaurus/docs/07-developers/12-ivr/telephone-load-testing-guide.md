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
lives in `packages/step-cli/scripts/telephone-load-test-inputs/config/layers.yaml`,
under the `setup:` / `telephone_run:` / `online_run:` sections respectively.
`config/` is gitignored (it holds real per-server credentials); copy the
tracked
[`layers.yaml.example`](https://github.com/sequentech/step/blob/main/packages/step-cli/scripts/telephone-load-test-inputs/layers.yaml.example)
template there first:

```bash
mkdir -p packages/step-cli/scripts/telephone-load-test-inputs/config
cp packages/step-cli/scripts/telephone-load-test-inputs/layers.yaml.example \
  packages/step-cli/scripts/telephone-load-test-inputs/config/layers.yaml
```

Edit that copy before each run instead of passing flags. A field left as
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
default): a top-level `tenants.json` index, and one `tenant-<tenant_id>/`
subdirectory per provisioned tenant holding that tenant's own `summary.json`
and voters CSV (with a single tenant, that's just one subdirectory).

By default Stage 1 provisions `setup.tenant_id` itself. Set
`setup.new_tenants: N` to instead create `N` brand-new tenants and import
the *same* election event into each — Stage 2 then places calls (or casts
online votes) across every tenant `tenants.json` lists. Leaving `tenant_id`
unset defaults `new_tenants` to `1`, so omitting it entirely provisions one
fresh tenant instead of reusing an existing one (creating a tenant needs
`setup.keycloak_admin_user`/`keycloak_admin_password` — a Keycloak
master-realm admin, distinct from `admin_portal_user` — to look up each new
tenant's freshly generated `api-key-client` secret).

A brand-new tenant starts blank — no trustees, and Keycloak/roles config
from the generic default template rather than `tenant_id`'s own. For each
one, the script: exports `tenant_id`'s Keycloak/roles config
(`export-tenant-config`), downloads it and re-uploads it so the new tenant
owns its own copy (documents are tenant-owned records — a document can't be
imported into a tenant that doesn't own it), imports that copy
(`import-tenant-config`), then copies `tenant_id`'s registered trustees
(`list-trustees` / `create-trustee`) so the keys ceremony has trustees to
work with at all. None of this needs configuring — it's automatic whenever
`new_tenants > 0`.

The keys ceremony defaults to `setup.ceremony_policy: AUTOMATIC`: each
trustee's `braid` service still does its DKG round the same way, but nothing
needs to log in as `trustee1`/`trustee2` to confirm it — the ceremony's
status flips to done on its own once every trustee's public key is on the
board, matching the Admin Portal's "automatic ceremony" option. Set it to
`MANUAL` to instead drive `complete-key-ceremony` as each configured
trustee, as the CLI always did previously.

Each run appends a random 5-character suffix to the election event's alias
(e.g. `TECUMSEH - DATAFIX Test - K3F9Q`) — the admin portal's election event
list renders alias, not name, so that's the field that needs the suffix to
actually be visible there. The full alias is printed at the end of the run
and recorded as `election_event_alias` in `summary.json`.

Using your own election event JSON instead of the tracked example works the
same way — just point `setup.election_event_json` at it in `layers.yaml`. If
it has more than one area, set `setup.voter_area_name` to pick which one
voters are generated into (`null` defaults to the first area).

Importing the election event also creates its own Keycloak realm
(`tenant-<tenant_id>-event-<election_event_id>`, printed as `keycloak_realm`
in `summary.json`), seeded with its own `ivr-service`/`ivr-voting` clients —
these are **not** in the tenant's realm alongside `api-key-client`. Since
every Stage 1 run creates a brand-new election event (and therefore a new
realm), fetch `telephone_run.keycloak_ivr_service_client_secret`/
`keycloak_ivr_voting_client_secret` from *this* realm after each run —
Keycloak admin console → the realm printed above → Clients →
`ivr-service`/`ivr-voting` → Credentials tab — rather than reusing a value
from a previous run's election event.

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

(`<realm>` is `summary.json`'s `keycloak_realm` — the election event's own
realm, not the tenant's; see the note in [step 1](#1-stage-1--provision-the-election-event-and-voters).
The client secret goes in `telephone_run.keycloak_ivr_service_client_secret`
in `layers.yaml` — or `.devcontainer/.env.development` for the local
devcontainer stack, which always targets the same seeded test election
event.)

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

## 4. Clean up: delete the election event(s)

Stage 1's `tenants.json` lists every tenant it provisioned into (one, unless
`setup.new_tenants` was set); each has its own election event to delete, and
each needs `step-cli` re-authenticated against *that* tenant first (a
session is scoped to one tenant at a time):

```bash
python3 -c '
import json
run_dir = "packages/step-cli/scripts/telephone-load-test-output/run"
tenants = json.load(open(f"{run_dir}/tenants.json"))["tenants"]
for t in tenants:
    summary = json.load(open(f"{run_dir}/{t[\"dir\"]}/summary.json"))
    print(t["tenant_id"], summary["election_event_id"])
'
```

For each `(tenant_id, election_event_id)` pair printed above:

```bash
step-cli step config --tenant-id "$TENANT_ID" --endpoint-url ... --keycloak-url ... \
  --keycloak-user "$ADMIN_PORTAL_USER" --keycloak-password "$ADMIN_PORTAL_PASSWORD" \
  --keycloak-client-id api-key-client --keycloak-client-secret "$API_KEY_CLIENT_SECRET"
step-cli step delete-election-event --election-event-id "$ELECTION_EVENT_ID"
```

(matching the same `config` call `configure_as` makes in
`packages/step-cli/scripts/setup_telephone_load_test.py`, with that
tenant's own `layers.yaml` values.) `delete-election-event` calls the
`delete_election_event` GraphQL mutation, which queues an async task tearing
down the election event's Postgres/Hasura rows, its Keycloak realm, and its
ImmuDB and document-store data — the command blocks and polls until that
task finishes (or fails/times out after 5 minutes), so a `Success!` means
cleanup is actually done, not just queued.

A tenant Stage 1 auto-created (`setup.new_tenants`) isn't deleted by this —
only its election event is. There's no `delete-tenant` command; removing the
tenant itself (if desired) is a manual Keycloak/Hasura cleanup.

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
- **IVR client secrets are per-election-event, not per-tenant.** Since Stage
  1 provisions a new election event realm every run, a
  `keycloak_ivr_service_client_secret`/`keycloak_ivr_voting_client_secret`
  that worked for a previous run's election event will not work for a new
  one — re-fetch them from the new realm each time (see the note in
  [step 1](#1-stage-1--provision-the-election-event-and-voters)). With more
  than one tenant (`setup.new_tenants > 1`), the flat
  `telephone_run.keycloak_ivr_service_client_secret`/
  `keycloak_ivr_voting_client_secret` can cover at most one of them — set
  `telephone_run.tenant_ivr_secrets` (keyed by `tenant_id`) for the rest.
