---
id: headless_load_test_cli
title: The `headless-load-test` CLI
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# The `headless-load-test` CLI

## Introduction

`headless-load-test` is a Rust CLI, part of this repository's Cargo workspace
(`packages/headless-load-test`), that provisions election events across many tenants
and then casts votes against them directly over the network — no browser, no
WebDriver. Where [Load Testing](./load_testing.md) covers `vote-cast`
(browser-driven, via headless Chrome) and `duplicate-votes` (DB-level,
bypasses the API entirely), `headless-load-test` exercises the real API path — login,
ballot encryption, and the cast-vote endpoint — without the overhead of a
browser.

Use `headless-load-test` when you want to know how the platform behaves under
concurrent voter traffic at the API/crypto layer. Use `vote-cast` when you
specifically need to exercise the voting UI. Use `duplicate-votes` when you
just need database rows and don't care about login or encryption.

In a single run, `headless-load-test`:

1. Logs in once as an admin identity.
2. Creates one or more tenants and, within each, one or more election
   events, importing the same election-event template into every one of
   them; provisions voters into each.
3. Publishes each election event and opens voting.
4. Casts votes against every election event concurrently, at a configured
   rate, for a configured duration.
5. Prints a summary report and exits with a non-zero status if anything
   failed — either a cast, or an election event that never made it to
   voting at all.

## Architecture

The crate is organized as one module per concern:

| Module | Responsibility |
|---|---|
| `config` | Parses `layers.yaml` and loads the election-event template as opaque JSON |
| `auth` | Keycloak token endpoint calls: password grant and client-credentials grant |
| `hasura` | Thin GraphQL-over-HTTP client (`HasuraClient`) and Hasura error classification |
| `types::hasura` | Custom scalar aliases `graphql_client` needs (`uuid`, `jsonb`, ...) |
| `provision` | Phase 1: tenant creation, import, publish, open voting, voter provisioning |
| `vote` | Phase 2: login, ballot-style fetch, ballot encryption, cast, outcome classification |
| `concurrency` | The rate limiter and voter pool Phase 2 runs on |
| `report` | Aggregates outcomes into per-election-event and per-run summaries |
| `run` | Wires all of the above into the end-to-end pipeline `main.rs` calls |

Each `.graphql` query file under `src/graphql/` pairs with a
`#[derive(GraphQLQuery)]` struct next to the code that uses it — the same
pattern `step-cli` uses (`packages/step-cli/src/commands/cast_vote.rs`), and
several mutations (`get_upload_url`, `get_task_execution`,
`update_event_voting_status`, `insert_cast_vote`) are copied close to
verbatim from there. `graphql_client` checks every query and variable
against `src/graphql/schema.json` at compile time, so a renamed field or a
typo'd argument is a build error, not a runtime surprise.

## Requirements

- This repository checked out, with the `devenv` Nix environment available
  (see the repo root `devenv.nix`).
- Network access from wherever you run `headless-load-test` to the target
  environment's Hasura GraphQL endpoint and Keycloak.
- An admin identity with permission to create tenants, import election
  events, publish, and open voting on the target environment — see
  [Authentication](#authentication) below; setting this up correctly is
  the part most likely to need attention on a new target environment.

## Building the tool

From the repo root, enter the Nix environment and build a release binary —
load testing is exactly the case where the unoptimized `dev` profile matters,
so don't skip `--release`:

```bash
cd /workspaces/step && devenv shell bash -- -c '
  cd packages && \
  CARGO_TARGET_DIR=/workspaces/step/packages/headless-load-test/rust-local-target \
  cargo build --release -p headless-load-test
'
```

The binary is then at
`packages/headless-load-test/rust-local-target/release/headless-load-test`.

## Authentication

There are two distinct identities in a `headless-load-test` run, using two different
OAuth grants, because they need two different things from Keycloak.

### Voter login: password grant

Phase 2 logs each voter in with `grant_type=password` against the public
`voting-portal` client — no `client_secret` (sending an empty string is
worse than omitting it: Keycloak's client-authenticator chain can read the
mere presence of the field as an attempted, then-missing, confidential-client
auth). This is exactly what a real browser session does, minus PKCE — the
whole point is to exercise the real voter authorization path
(`authorize_voter_election`,
`packages/sequent-core/src/services/authorization.rs:73-113`), not bypass
it.

### Admin login: client-credentials grant

Phase 1's privileged calls (`insertTenant`, `import_election_event`,
`generate_ballot_publication`/`publish_ballot`, `update_event_voting_status`,
`import_users`) all require the Hasura role `admin-user`
(`hasura/metadata/actions.yaml`). `headless-load-test` authenticates for these with
`grant_type=client_credentials` — a confidential OIDC client's own *service
account*, not a human user.

This wasn't the original plan. The design started from `step-cli config`'s
shape (a human user, password grant), on the assumption that some
password-grant-capable admin user would hold the `admin-user` role in the
target tenant realm. Running the actual binary against this repository's
own dev environment (`devenv up`) surfaced that this dev environment's seed
data doesn't have one: the only identity holding `admin-user` there is a
confidential client's service account, so the admin login had to move to
`client_credentials` to match. Two further requirements came out of that
same investigation, and apply to any target environment, not just this
one's dev seed:

1. **The client's `x-hasura-allowed-roles` protocol mapper must be
   role-based, not hardcoded.** Keycloak clients commonly get a
   hardcoded-claim mapper here (`oidc-hardcoded-claim-mapper`, a fixed JSON
   value) for a fixed low-privilege role — that's harmless for a client
   whose service account isn't meant to be an admin, but it means
   *assigning* the realm role `admin-user` to that service account's user
   has no effect: the token still carries whatever the hardcoded mapper
   says. The client used for admin auth needs the dynamic
   `oidc-usermodel-realm-role-mapper` instead, so the realm roles actually
   assigned to its service account show up in
   `https://hasura.io/jwt/claims.x-hasura-allowed-roles`.
2. **The client's token must carry an `x-hasura-tenant-id` claim.**
   Harvest's `JwtClaims` extractor requires it (a non-optional `String`
   field, `packages/sequent-core/src/services/jwt.rs:26`) — a token
   missing it fails Rocket's request guard outright (HTTP 401, logged as
   `Request guard 'JwtClaims' failed`), before any handler-level
   permission logic even runs. Some confidential clients' default mapper
   sets don't include this claim at all.
3. That `x-hasura-tenant-id` must equal Harvest's own `SUPER_ADMIN_TENANT_ID`
   environment variable — `insert_tenant`'s handler calls
   `authorize(&claims, true, None, vec![Permissions::TENANT_CREATE])`, and
   with no explicit `tenant_id_opt`, that function's super-admin branch
   checks the caller's tenant against exactly that env var
   (`packages/sequent-core/src/services/authorization.rs:15-50`).

None of this is `headless-load-test`-specific — it's what any headless Hasura Action
caller needs from a confidential client used for cross-tenant admin
operations. `--admin-tenant-id` in the flag reference below is that
super-admin tenant.

### Admin login: password grant, when a target requires step-up ("gold") auth

Some of the same privileged calls above — specifically
`generate_ballot_publication`/`publish_ballot` and
`update_event_voting_status` — additionally require `has_gold_permission`
(`packages/sequent-core/src/services/jwt.rs`), which checks the token's
`acr` claim against `Permissions::GOLD` ("gold") and a 60-second freshness
window. This is a Keycloak Level-of-Authentication (LoA/ACR) step-up, and
step-up is fundamentally an *interactive authentication flow* concept: a
`client_credentials` grant never runs one, so a service account's token can
**never** carry `acr: gold`, no matter which realm roles it holds. If the
target realm gates these actions behind gold, plain `client_credentials`
admin auth will fail those two calls specifically with `403 Forbidden` /
`Insufficient privileges`, even though every earlier step (tenant creation,
import, voter provisioning) succeeds fine under it.

`headless-load-test` supports a second admin login mode for exactly this
case: `grant_type=password`, via `--admin-keycloak-username` /
`--admin-keycloak-password` (both required together; the client id/secret
flags above are still required too — password grant for a confidential
client still authenticates the client itself, not just the user). This is
the same mechanism `step-cli config` has always used
(`packages/step-cli/src/utils/keycloak.rs::generate_keycloak_token`) — the
admin login didn't start as `client_credentials`; it moved there
specifically because tenant creation needed a role no password-grant user
held (see above). Now that a *different* privileged action needs step-up
that only a real, interactive-shaped login can provide, going back to
`step-cli`'s original approach — for admin auth specifically, not for
tenant creation — is the fix.

Whether a given username can actually reach `acr: gold` this way is a
target-realm authentication-flow question, not something
`headless-load-test` controls: it depends on the realm's `direct_grant`
flow (and any client-level `authenticationFlowBindingOverrides`) actually
granting LoA level 2 for that grant type. In this repository's own dev
tenant, the confidential client `api-key-client` is bound to a custom
`direct_grant` flow (alias `gold direct grant`) that unconditionally grants
gold once the username validates — password itself isn't checked in that
flow (`direct-grant-validate-password` is `DISABLED`), so *any* password
value works for a user that flow accepts. The existing tenant user
`api-user` already holds every permission this tool's Phase 1 needs
(`admin-user`, `document-upload`, `voter-create`, `election-state-write`,
`publish-write`, `tenant-create`, ...) — pass `--admin-keycloak-username
api-user` with `--admin-keycloak-client-id api-key-client` and any
non-empty `--admin-keycloak-password` to use it.

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
  id, as in the example above. Ignored (used only as the report label) when
  `--target-tenant-id` is set — see below.
- `tenants[].election_events[]` — one entry per election event to create
  inside that tenant, each importing the same `--election-event-template`.
- `voters` — how many voters to provision for that election event.
- `votes_per_second` — target sustained cast-vote rate for that election
  event. Must be a positive number; fractional values are allowed (`2.5`).
- `duration` — how long to keep casting votes for that election event.
  Accepts a bare integer (seconds) or a number suffixed with `s`, `m`, or
  `h` — `90s`, `10m`, `1h` are all valid; `10x` or `soon` are not.

### Election-event template

This is the same JSON shape `step-cli import-election --file-path` accepts
— see [Import Election Event](../06-import-election-event.md) for the full
format. `packages/headless-load-test/data/election-event-template.json` is a
ready-made starting point. The same template is imported, unmodified, into
every election event `layers.yaml` describes; per-event IDs are remapped on
import, so no per-event authoring is needed.

It's a fork of `step-cli`'s own fixture
(`packages/step-cli/data/test-election-template.json`), not that file
directly: that fixture's `voting-portal` Keycloak client has no
`authorized-elections-oidc-usermodel-attribute-mapper` configured, so a
voter provisioned against it can never get an `authorized-election-ids`
claim and Phase 2 casting would 401 regardless of voter setup. `headless-load-test`'s
copy adds that mapper pair (sourced from the working devcontainer realm
export) to both `voting-portal` and `onsite-voting-portal`, so it's usable
for a real end-to-end run out of the box.

`headless-load-test` uploads this file as-is — it doesn't locally re-validate it
against the full import schema, the same way `step-cli import-election`
doesn't either. A malformed (not-valid-JSON) file is caught immediately with
a clear error; anything else the server rejects surfaces as an import
failure for the affected election event, not a crash of the whole run.

## Command-line flags

```bash
headless-load-test \
  --layers-file layers.yaml \
  --election-event-template election-event.json \
  --endpoint-url https://api.example.sequent.vote/v1/graphql \
  --keycloak-url https://keycloak.example.sequent.vote \
  --admin-tenant-id 90505c8a-23a9-4cdf-a26b-4e19f6a097d5 \
  --admin-keycloak-client-id headless-load-test-admin
```

| Flag | Env var fallback | Required | Description |
|---|---|---|---|
| `--layers-file <PATH>` | — | yes | Path to `layers.yaml`. |
| `--election-event-template <PATH>` | — | yes | Path to the election-event JSON template imported into every election event. |
| `--endpoint-url <URL>` | `HEADLESS_LOAD_TEST_ENDPOINT_URL` | yes | Hasura GraphQL endpoint on the target environment. |
| `--keycloak-url <URL>` | `HEADLESS_LOAD_TEST_KEYCLOAK_URL` | yes | Base Keycloak URL. Per-tenant, per-event realms (`tenant-{t}-event-{e}`) are resolved under this for both the admin login and every voter login. |
| `--admin-tenant-id <ID>` | `HEADLESS_LOAD_TEST_ADMIN_TENANT_ID` | yes | The super-admin tenant the admin client authenticates against — see [Authentication](#authentication). Not one of the tenants `layers.yaml` creates. |
| `--admin-keycloak-client-id <ID>` | `HEADLESS_LOAD_TEST_ADMIN_KEYCLOAK_CLIENT_ID` | yes | A confidential client in that tenant's realm whose service account carries the `admin-user` Hasura role. |
| `--admin-keycloak-client-secret <SECRET>` | `HEADLESS_LOAD_TEST_ADMIN_KEYCLOAK_CLIENT_SECRET` | yes | That client's secret. Prefer the environment variable over the flag — it keeps the secret out of shell history and `ps`. |
| `--admin-keycloak-username <NAME>` | `HEADLESS_LOAD_TEST_ADMIN_KEYCLOAK_USERNAME` | no | Switches admin login to `grant_type=password`. Set together with `--admin-keycloak-password` — see [Admin login: password grant](#admin-login-password-grant-when-a-target-requires-step-up-gold-auth). |
| `--admin-keycloak-password <PASSWORD>` | `HEADLESS_LOAD_TEST_ADMIN_KEYCLOAK_PASSWORD` | no | That user's password-grant credential. Prefer the environment variable over the flag, same reasoning as the client secret. |
| `--max-concurrent-tenants <N>` | — | no | Caps how many tenants are provisioned and run concurrently. Default: all tenants in `layers.yaml` at once. |
| `--target-tenant-id <ID>` | `HEADLESS_LOAD_TEST_TARGET_TENANT_ID` | no | An *existing* tenant to provision every election event into, instead of creating a fresh tenant per `tenants[].slug`. See [Targeting an existing tenant](#targeting-an-existing-tenant) below. |

`headless-load-test` logs in once, in-memory, and reuses that admin token
concurrently across every tenant it provisions — unlike `step-cli`'s own
`config` subcommand, which writes a single-profile config file keyed by the
binary's own directory and so isn't built for running several tenants at
once.

### Targeting an existing tenant

By default `headless-load-test` creates a brand-new tenant for every
`tenants[]` entry in `layers.yaml` (`insertTenant`, non-idempotent — see
[Troubleshooting](#troubleshooting)). Pass `--target-tenant-id <ID>` to skip
tenant creation entirely and import every election event straight into an
*existing* tenant instead — useful for a quick run against a tenant you
already have set up (e.g. this repository's own dev tenant,
`90505c8a-23a9-4cdf-a26b-4e19f6a097d5`), without burning a fresh slug each
time.

When set, `tenants[].slug` is no longer used to create anything — it's kept
only as the label election-event reports are grouped under. If `layers.yaml`
lists more than one `tenants[]` entry, all of them import into that same
target tenant; for genuine multi-tenant load, omit the flag and let each
entry create its own tenant as usual.

Note that this only skips *tenant* creation — election-event import itself
is still not idempotent, so re-running the same `layers.yaml` and
`--election-event-template` against the same `--target-tenant-id` still
creates a brand-new election event inside it each time, not an update to a
previous run's.

`--target-tenant-id` and `--admin-tenant-id` are independent and commonly
point at the same tenant in a local dev run, but they don't have to: the
former is where election events get provisioned, the latter is only the
realm the admin service account itself logs into.

## Running a load test

Against the local dev environment (`devenv up`):

```bash
export HEADLESS_LOAD_TEST_ADMIN_KEYCLOAK_CLIENT_SECRET=<secret for the client you set up per Authentication>

packages/headless-load-test/rust-local-target/release/headless-load-test \
  --layers-file layers.yaml \
  --election-event-template packages/headless-load-test/data/election-event-template.json \
  --endpoint-url http://127.0.0.1:8080/v1/graphql \
  --keycloak-url http://127.0.0.1:8090 \
  --admin-tenant-id 90505c8a-23a9-4cdf-a26b-4e19f6a097d5 \
  --admin-keycloak-client-id <that client's id>
```

The dev environment's default seed data does not, out of the box, have a
client that satisfies every requirement in
[Authentication](#authentication) at once — see
[Troubleshooting](#troubleshooting) for exactly what's missing and how to
fix it locally.

## What happens during a run

```
┌────────────────────────────────┐
│ 0. Admin login (once)           │
│    client_credentials, in-memory, reused across every tenant │
└──────────────┬──────────────────┘
               ▼
┌──────────────────────────────────────────────┐
│ 1. Provision (one task per tenant; its election events run  │
│    concurrently with each other within that task)           │
│    create tenant → import → publish → open voting →         │
│    provision voters (bulk CSV import)                       │
└──────────────┬───────────────────────────────┘
               │ election event's voting is open
               ▼
┌──────────────────────────────────────────────┐
│ 2. Vote (per election event, rate-limited, concurrent)       │
│    login (voting-portal, password grant) → fetch ballot style→│
│    build synthetic ballot → encrypt + hash (+ sign) → cast   │
└──────────────┬───────────────────────────────┘
               │ duration elapses, in-flight votes drain
               ▼
┌──────────────────────────────┐
│ 3. Report and exit            │
└──────────────────────────────┘
```

### Phase 1 — Provisioning

`provision::provision_election_event` runs, per election event:

1. **Import** — `get_upload_url` → PUT the template JSON → `import_election_event`.
   This only *enqueues* the import; the mutation returns before it's done,
   so `headless-load-test` polls the `task_execution` it comes back with
   (`provision::tasks::poll_task_execution`, 120s timeout / 2s interval)
   before doing anything else with the new election event.
2. **Publish** — `generate_ballot_publication` → poll
   `get_ballot_publication_status` (60s timeout / 3s interval, matching
   `step-cli`'s own loop) → `publish_ballot`.
3. **Open voting** — `update_event_voting_status(OPEN, [ONLINE])`. `ONLINE`
   specifically, because that's the only channel `voting-portal`-issued
   tokens are authorized for.
4. **Provision voters** — a single bulk CSV import
   (`username,password,area_name`), through the same
   `get_upload_url` → PUT → `import_users` mutation and celery task
   (`packages/windmill/src/tasks/import_users.rs`) the admin-portal's
   Voters-tab import wizard uses. One upload provisions the whole batch,
   and the `password` column gets hashed and written as a real,
   immediately-usable Keycloak credential server-side — no
   required-action/temporary-password complication.

   Deliberately does **not** set an `authorized-election-ids` attribute.
   Eligibility is normally established dynamically: the custom Keycloak
   protocol mapper `AuthorizedElectionsUserAttributeMapper`
   (`packages/keycloak-extensions/conditional-authenticators/src/main/java/sequent/keycloak/protocol/oidc/mappers/AuthorizedElectionsUserAttributeMapper.java`)
   computes that claim at token-issuance time from the voter's `area-id`
   attribute via `sequent_backend_area_contest` — exactly the
   area→area_contests→contest→election path — falling back to every
   election in the event only if the area has none. The CSV's
   `area_name` column (which the importer resolves to the `area-id`
   attribute by name) is both necessary and sufficient for this; a real
   voter export from this platform leaves its own
   `authorized-election-ids` column blank for the same reason.

Tenant creation (`provision::create_tenant`, the one Phase 1 step that
happens once per tenant rather than once per election event) works the
same way as import: `insertTenant` only enqueues realm creation and hands
back an id before the realm exists, so it's followed by the same
`task_execution` poll.

### Phase 2 — Voting

`vote::cast_one_vote` runs, per attempted vote:

1. **Login** — password grant against `tenant-{t}-event-{e}`, client
   `voting-portal`.
2. **Fetch ballot style** — `GetBallotStyles` (no arguments; scoping is
   entirely by the voter's own JWT via Hasura row-level permissions),
   decoding `ballot_eml` into `sequent_core::ballot::BallotStyle`.
3. **Build a synthetic ballot** — `vote::ballot::build_synthetic_contests`
   picks the first candidate in every contest that isn't an explicit-
   invalid/blank/write-in/category-list placeholder.
4. **Encrypt, hash, (optionally) sign** — `vote::ballot::prepare_ballot`,
   dispatching on `contest_encryption_policy` the way voting-portal's own
   `prepare_encrypted_ballot` does, calling `sequent-core`'s native Rust
   functions directly (no WASM needed — this is already Rust). Ported from
   `create_singular_ballot`/`create_multi_ballot` in
   `beyond/packages/ivr-core/src/execution/phases/ballot_utils.rs`, which
   are `pub(super)` there and so can't be imported directly.
5. **Cast** — `insert_cast_vote(election_id, ballot_id, content)`.

`insert_cast_vote` is synchronous end-to-end (Harvest's handler awaits the
DB insert before responding), so every cast's outcome is known immediately
— casting never needs to poll, only the async provisioning steps above do.

Outcomes are classified into `vote::CastOutcome`: `Success`,
`VoterStateLocked` (a same-voter concurrent-write conflict — expected to be
essentially zero, since the concurrency model guarantees distinct voters
per in-flight cast), `RevoteLimitExceeded`, or a catch-all
`Rejected { code, message }`. Classification reads
`errors[0].extensions.code`, **not** the HTTP status — Hasura always
responds 200 regardless of what the underlying Harvest action returned, so
the status-code table in `insert_cast_vote.rs` isn't visible to a GraphQL
client at all. `VoterStateLocked` and a genuine `CheckStatusFailed` happen
to share that same `code`, so they're told apart by the lock's fixed
message text (`"The voter state is being updated; retry the vote"`).

## Concurrency model

- **One tokio task per tenant** (`run::run_tenant`), bounded by
  `--max-concurrent-tenants` via a `tokio::sync::Semaphore`. Each tenant
  task creates its tenant once, then spawns one further task per election
  event — those run concurrently with each other, unthrottled by the
  tenant-level semaphore, since they share nothing but the tenant they
  belong to.
- **Within one election event, a voter pool, not a lock
  (`concurrency::run_rate_limited`).** Every provisioned voter starts in an
  `mpsc` channel. A `tokio::time::interval` ticks at the configured
  `votes_per_second`; on each tick, `headless-load-test` tries to take one voter out
  of the channel (`try_recv`, non-blocking) and spawns a cast for it. The
  voter is only returned to the channel once that cast completes. A voter
  is therefore only ever in one of two places — idle in the channel, or
  inside exactly one in-flight task — which is what actually guarantees no
  two in-flight casts ever share a voter, not a per-voter mutex.
- **A tick with no free voter is skipped, not queued.** If `votes_per_second`
  asks for a rate the voter pool can't sustain (every voter still waiting
  on its previous cast), that tick is simply dropped. Configuring an
  unsustainable rate shows up as a lower achieved rate in the report, not
  as the run stalling or queuing up unboundedly.
- **The configured `duration` stops new casts, then drains in-flight
  ones** — once the deadline passes, no further ticks spawn new work, but
  whatever's already running is still awaited so its outcome makes it into
  the report.

## Understanding the report

```
Tenant loadtest-2026-08-25-a / event 3f2b6e2c-...:
  attempted: 12000   succeeded: 11987   failed: 13
    cast conflicts (409, concurrent same-voter write): 0
    revote limit exceeded: 11
    login failures: 2
  p50: 84ms   p95: 210ms   p99: 340ms

Tenant loadtest-2026-08-25-a / event 9a1c4f10-...:
  attempted: 1500   succeeded: 1500   failed: 0
  p50: 61ms   p95: 140ms   p99: 190ms

Tenant loadtest-2026-08-25-b / event 7d7f840a-...:
  attempted: 45000   succeeded: 45000   failed: 0
  p50: 77ms   p95: 175ms   p99: 260ms

3 election event(s) voted, 1 with cast failures, 0 never provisioned. Exit code: 1
```

If provisioning itself failed for an election event (tenant creation,
import, publish, opening voting, or voter provisioning), it never reaches
voting at all and so has no `attempted`/`succeeded` line — instead it shows
up as a `Provisioning failed: ...` line before the summary, and counts
against "never provisioned" rather than "with cast failures".

A non-zero exit code means at least one election event had a cast failure,
or never got provisioned. Check which error class dominates before
re-running: login failures point at Keycloak throughput or bad voter
credentials, cast conflicts should be ~0 by construction (see
[Concurrency model](#concurrency-model)) and a nonzero count there means the
pool guarantee broke, not ordinary headless-load-test noise, and revote-limit
failures mean the election's revote policy is tighter than the configured
vote rate implies.

## Troubleshooting

- **Admin login fails, or `insertTenant`/`import_election_event`/etc.
  return `field 'X' not found in type 'mutation_root'`** — that specific
  message is Hasura's way of saying the field exists but the caller's role
  can't see it (a genuine permission gap, not a missing field). Walk
  through every requirement in [Authentication](#authentication) in order:
  the client needs `serviceAccountsEnabled`, a *role-based*
  `x-hasura-allowed-roles` mapper (not hardcoded), its service account
  needs the realm role `admin-user` actually assigned, and its issued
  token needs an `x-hasura-tenant-id` claim matching Harvest's
  `SUPER_ADMIN_TENANT_ID`. In this repository's own dev environment, no
  single seeded client satisfies all of these at once as shipped — the
  quickest fix locally is to add the dynamic role mapper's
  `x-hasura-tenant-id` claim (copy the mapper from a client that already
  has one, e.g. `voting-portal`'s, onto the client you intend to use for
  admin auth) and grant that client's service-account user the
  `admin-user` realm role via the Keycloak admin console.
- **Provisioning fails uploading the template or voters CSV with `tcp
  connect error: Connection refused` to `127.0.0.1:9000`** — this is
  `get_upload_url`'s presigned MinIO PUT URL. It's deliberately
  `127.0.0.1:9000` because that's meant for a real browser on your actual
  machine (VS Code forwards that port there); it's not reachable from
  *inside* the devcontainer, where `headless-load-test` itself runs — the
  devcontainer is a sibling of the `minio` container on the same compose
  network, and a sibling container can't reach another through the Docker
  *host's* published port, only by service name. Fix it locally with a
  one-off forward so `127.0.0.1:9000` inside the devcontainer actually
  reaches `minio:9000`:
  ```bash
  socat TCP-LISTEN:9000,bind=127.0.0.1,fork,reuseaddr TCP:minio:9000 &
  ```
  Run once per devcontainer session, before `headless-load-test`; the
  signed URL's `Host` header is still `127.0.0.1:9000`, so the AWS SigV4
  signature stays valid through the forward.
- **"tenant already exists" / import fails outright** — tenant creation and
  election-event import are both non-idempotent. Re-running `layers.yaml`
  unchanged reuses the same slugs against state that already exists. Bump
  the `slug` values (a date or run id suffix works well) before every run.
- **Cast-vote rate is lower than `votes_per_second` asks for** — the
  practical ceiling is almost always Keycloak login throughput, not the
  cast-vote endpoint itself (which is a single synchronous DB insert), or
  the voter pool being smaller than what the rate needs to stay saturated
  (see [Concurrency model](#concurrency-model)). Lower `votes_per_second`,
  raise `voters`, or spread load across more election events rather than
  one very hot one.
- **`VoterStateLocked` shows up under `cast conflicts`** — expected to be
  zero; the concurrency model structurally prevents two in-flight casts
  from sharing a voter. A nonzero count means that guarantee broke, which
  is a bug worth reporting, not ordinary headless-load-test noise.
- **Every cast fails with `Rejected` /
  `CheckStatusFailed("auth_time is not a valid integer")`** — Keycloak only
  sets the `AUTH_TIME` session note (and so the `auth_time` claim) for the
  `authorization_code`/browser login flow, never for `grant_type=password`.
  Harvest's `check_status`
  (`packages/windmill/src/services/insert_cast_vote.rs`) requires
  `auth_time` for the `ONLINE` channel, so every voter login
  `headless-load-test` does (`grant_type=password`, no browser) hits this
  by construction, on any target environment. There's a dev-only escape
  hatch: set `HARVEST_ALLOW_ONLINE_AUTH_TIME_IAT_FALLBACK=true` on the
  target's Harvest process to fall back to `iat`, the same fallback the
  `TELEPHONE` channel and `has_gold_permission` already use elsewhere. In
  this repository's own dev environment it's wired through
  `.devcontainer/docker-compose-base.yml` and set in
  `.devcontainer/.env.development` — **not** `.devcontainer/.env`, which is
  gitignored and gets unconditionally overwritten from `.env.development`
  by `.devcontainer/scripts/initialize-command.sh` on every devcontainer
  init/rebuild, so an edit there alone doesn't survive one. After changing
  it, recreate the `harvest` container — a source edit alone doesn't pick
  up new env vars either
  (`docker compose -p step_devcontainer -f .devcontainer/docker-compose.yml
  up -d --no-deps harvest`). It's deliberately absent from every real
  deployment's config — do not set it there.

## Comparison with other load-testing tools

| | `headless-load-test` | `vote-cast` | `duplicate-votes` |
|---|---|---|---|
| Exercises login | yes (Keycloak, password grant) | yes (real UI) | no |
| Exercises ballot encryption | yes (native Rust, no WASM) | yes (browser WASM) | no |
| Exercises the cast-vote API | yes | yes | no — writes `cast_vote` rows directly |
| Needs a browser | no | yes (headless Chrome) | no |
| Provisions its own election events | yes | no — needs one already set up | no — needs an existing vote to duplicate |

See [Load Testing](./load_testing.md) for `vote-cast` and `duplicate-votes`.
