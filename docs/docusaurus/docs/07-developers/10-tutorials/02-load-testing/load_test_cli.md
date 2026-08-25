---
id: load_test_cli
title: Headless Load Testing with `load-test`
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Headless Load Testing with `load-test`

## Introduction

`load-test` is a Rust CLI, built as part of this repository's Cargo
workspace, that provisions election events across many tenants and then
casts votes against them directly over the network — no browser, no
WebDriver. Where [Load Testing](./load_testing.md) covers `vote-cast`
(browser-driven, via headless Chrome) and `duplicate-votes` (DB-level,
bypasses the API entirely), `load-test` exercises the real API path — login,
ballot encryption, and the cast-vote endpoint — without the overhead of a
browser.

Use `load-test` when you want to know how the platform behaves under
concurrent voter traffic at the API/crypto layer. Use `vote-cast` when you
specifically need to exercise the voting UI. Use `duplicate-votes` when you
just need database rows and don't care about login or encryption.

In a single run, `load-test`:

1. Creates one or more tenants and, within each, one or more election
   events, importing the same election-event template into every one of
   them.
2. Publishes each election event and opens voting.
3. Casts votes against every election event concurrently, at a configured
   rate, for a configured duration.
4. Prints a summary report and exits with a non-zero status if any election
   event had failures.

## Requirements

- This repository checked out, with the `devenv` Nix environment available
  (see the repo root `devenv.nix`).
- Network access from wherever you run `load-test` to the target
  environment's Hasura GraphQL endpoint and Keycloak.
- Admin credentials with permission to create tenants, import election
  events, publish, and open voting on the target environment.

## Building the tool

From the repo root, enter the Nix environment and build a release binary —
load testing is exactly the case where the unoptimized `dev` profile matters,
so don't skip `--release`:

```bash
cd /workspaces/step && devenv shell bash -- -c '
  cd packages && \
  CARGO_TARGET_DIR=/workspaces/step/packages/load-test/rust-local-target \
  cargo build --release -p load-test
'
```

The binary is then at
`packages/load-test/rust-local-target/release/load-test`.

## Configuring a run

Every run needs two files: `layers.yaml`, which describes what to
provision and how hard to hit it, and an election-event template in the
same JSON shape `step-cli import-election` accepts.

### `layers.yaml`

```yaml
tenants:
  - slug: loadtest-2026-08-25-a
    election_events:
      - voters: 5000
        votes_per_second: 20
        duration: 10m
      - voters: 1000
        votes_per_second: 5
        duration: 5m
  - slug: loadtest-2026-08-25-b
    election_events:
      - voters: 20000
        votes_per_second: 50
        duration: 15m
```

- `tenants[].slug` — the tenant slug to create. **Must be unique on the
  target environment.** Tenant creation is not idempotent (neither is
  election-event import — see [Troubleshooting](#troubleshooting)), so
  reusing a slug from a previous run fails fast rather than silently
  reusing the old tenant. Bake something unique into it, e.g. a date or run
  id, as in the example above.
- `tenants[].election_events[]` — one entry per election event to create
  inside that tenant, each importing the same `--election-event-template`.
- `voters` — how many voters to provision for that election event (skipped
  if the template already bundles voters).
- `votes_per_second` — target sustained cast-vote rate for that election
  event. Must be a positive number; fractional values are allowed (`2.5`).
- `duration` — how long to keep casting votes for that election event.
  Accepts a bare integer (seconds) or a number suffixed with `s`, `m`, or
  `h` — `90s`, `10m`, `1h` are all valid; `10x` or `soon` are not.

### Election-event template

This is the same JSON shape `step-cli import-election --file-path` accepts
— see [Import Election Event](../06-import-election-event.md) for the full
format. `packages/step-cli/data/test-election-template.json` is a ready-made
starting point. The same template is imported, unmodified, into every
election event `layers.yaml` describes; per-event IDs are remapped on
import, so no per-event authoring is needed.

`load-test` uploads this file as-is — it doesn't locally re-validate it
against the full import schema, the same way `step-cli import-election`
doesn't either. A malformed (not-valid-JSON) file is caught immediately with
a clear error; anything else the server rejects surfaces as an import
failure for the affected election event, not a crash of the whole run.

## Command-line flags

```bash
load-test \
  --layers-file layers.yaml \
  --election-event-template election-event.json \
  --endpoint-url https://api.example.sequent.vote/v1/graphql \
  --keycloak-url https://keycloak.example.sequent.vote \
  --admin-tenant-id 90505c8a-23a9-4cdf-a26b-4e19f6a097d5 \
  --admin-keycloak-user support-admin \
  --admin-keycloak-client-id admin-cli
```

| Flag | Env var fallback | Required | Description |
|---|---|---|---|
| `--layers-file <PATH>` | — | yes | Path to `layers.yaml`. |
| `--election-event-template <PATH>` | — | yes | Path to the election-event JSON template imported into every election event. |
| `--endpoint-url <URL>` | `LOAD_TEST_ENDPOINT_URL` | yes | Hasura GraphQL endpoint on the target environment. |
| `--keycloak-url <URL>` | `LOAD_TEST_KEYCLOAK_URL` | yes | Base Keycloak URL. Per-tenant, per-event realms (`tenant-{t}-event-{e}`) are resolved under this for both admin and voter logins. |
| `--admin-tenant-id <ID>` | `LOAD_TEST_ADMIN_TENANT_ID` | yes | Existing tenant whose realm the admin user authenticates against — this identity needs cross-tenant permission to create tenants, import, publish, and open voting. |
| `--admin-keycloak-user <USER>` | `LOAD_TEST_ADMIN_KEYCLOAK_USER` | yes | Admin username. |
| `--admin-keycloak-password <PASSWORD>` | `LOAD_TEST_ADMIN_KEYCLOAK_PASSWORD` | yes | Admin password. Always pass this via the environment variable, never the flag — it avoids the password landing in shell history or `ps`. |
| `--admin-keycloak-client-id <ID>` | `LOAD_TEST_ADMIN_KEYCLOAK_CLIENT_ID` | yes | OIDC client id for the admin login (`admin-cli` in the dev environment). |
| `--admin-keycloak-client-secret <SECRET>` | `LOAD_TEST_ADMIN_KEYCLOAK_CLIENT_SECRET` | no | OIDC client secret, if the admin client isn't public. |
| `--max-concurrent-tenants <N>` | — | no | Caps how many tenants are provisioned and run concurrently. Default: all tenants in `layers.yaml` at once. |

This mirrors `step-cli config`'s admin login shape
(`packages/step-cli/src/commands/configure.rs`), rather than that command's
single-profile, directory-keyed config file — `load-test` logs in once,
in-memory, and reuses that token concurrently across every tenant it
provisions.

## Running a load test

Against the local dev environment (`devenv up`, all default dev credentials
per the repo `CLAUDE.md`):

```bash
export LOAD_TEST_ADMIN_KEYCLOAK_PASSWORD=admin

packages/load-test/rust-local-target/release/load-test \
  --layers-file layers.yaml \
  --election-event-template packages/step-cli/data/test-election-template.json \
  --endpoint-url http://127.0.0.1:8080/v1/graphql \
  --keycloak-url http://127.0.0.1:8090 \
  --admin-tenant-id 90505c8a-23a9-4cdf-a26b-4e19f6a097d5 \
  --admin-keycloak-user admin \
  --admin-keycloak-client-id admin-cli
```

## What happens during a run

```
┌──────────────────────────────┐
│ 1. Provision (per tenant/event, in parallel) │
│    create tenant → import → publish → open voting → provision voters │
└──────────────┬───────────────┘
               │ all election events open
               ▼
┌──────────────────────────────┐
│ 2. Vote (per election event, rate-limited, in parallel) │
│    login (voting-portal, password grant) → fetch ballot style →      │
│    encrypt + hash (+ sign) → cast                                    │
└──────────────┬───────────────┘
               │ duration elapses, in-flight votes drain
               ▼
┌──────────────────────────────┐
│ 3. Report and exit            │
└──────────────────────────────┘
```

Provisioning runs one tokio task per (tenant, election event), up to
`--max-concurrent-tenants`. Once an election event's voting is open, a rate
limiter ticks at its configured `votes_per_second`, spawning one
login-and-cast task per tick, each against a distinct voter — concurrent
workers never share a voter identity within a run, so there's no shared
token or lock contention beyond Keycloak's own login throughput. Voters are
provisioned with deterministic credentials (`voter-{n}` / `voter-{n}` per
election event) so a run's cast-vote traffic is fully reproducible.

Casting is synchronous end-to-end (`insert_cast_vote` awaits the DB insert
before responding), so pass/fail for every vote is known immediately — the
tool never needs to poll for cast-vote results, only for the provisioning
steps that do enqueue an async task (import and ballot publication).

## Understanding the report

```
Tenant loadtest-2026-08-25-a / event 3f2b... :
  attempted: 12000   succeeded: 11987   failed: 13
    login failures: 2
    cast conflicts (409, concurrent same-voter write): 0
    revote-limit exceeded: 11
  p50: 84ms   p95: 210ms   p99: 340ms

Tenant loadtest-2026-08-25-a / event 9a1c... :
  attempted: 1500    succeeded: 1500    failed: 0
  p50: 61ms   p95: 140ms   p99: 190ms

Tenant loadtest-2026-08-25-b / event 7d7f... :
  attempted: 45000   succeeded: 45000   failed: 0
  p50: 77ms   p95: 175ms   p99: 260ms

3 election event(s), 0 with failures. Exit code: 0
```

A non-zero exit code means at least one election event had failures — check
which error class dominates before re-running: login failures point at
Keycloak throughput or bad voter credentials, cast conflicts point at
`votes_per_second` outrunning what a single voter's row lock allows (should
be ~0 as long as each worker owns a distinct voter), and revote-limit
failures mean the election's revote policy is tighter than the configured
vote rate implies.

## Troubleshooting

- **"tenant already exists" / import fails outright** — tenant creation and
  election-event import are both non-idempotent. Re-running `layers.yaml`
  unchanged reuses the same slugs against state that already exists. Bump
  the `slug` values (a date or run id suffix works well) before every run.
- **Cast-vote rate is lower than `votes_per_second` asks for** — the
  practical ceiling is almost always Keycloak login throughput, not the
  cast-vote endpoint itself (which is a single synchronous DB insert). Lower
  `votes_per_second` or spread load across more election events rather than
  one very hot one.
- **`VoterStateLocked` (409) shows up under `cast conflicts`** — expected
  only if two workers ever draw the same voter; if it's non-zero and workers
  are supposed to own distinct voters, that's a bug in the run, not expected
  load-test noise.
- **Admin login fails** — `--admin-tenant-id` must be a tenant that already
  exists (it's where the admin *logs in*, not a tenant being created), and
  that admin identity needs cross-tenant permission to create the tenants
  listed in `layers.yaml`.

## Comparison with other load-testing tools

| | `load-test` | `vote-cast` | `duplicate-votes` |
|---|---|---|---|
| Exercises login | yes (Keycloak, password grant) | yes (real UI) | no |
| Exercises ballot encryption | yes (native Rust, no WASM) | yes (browser WASM) | no |
| Exercises the cast-vote API | yes | yes | no — writes `cast_vote` rows directly |
| Needs a browser | no | yes (headless Chrome) | no |
| Provisions its own election events | yes | no — needs one already set up | no — needs an existing vote to duplicate |

See [Load Testing](./load_testing.md) for `vote-cast` and `duplicate-votes`.
