---
id: harvest
title: Harvest request boundary tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Test file paths in this guide are relative to [`packages/harvest/tests/`](https://github.com/sequentech/step/blob/main/packages/harvest/tests). Commands state their working directory.

This coverage slice exercises actual Rocket routes, request guards and
error catchers with the local HTTP client. It uses the same route builder as production while avoiding startup of
service workers, probes and plugins. A static inventory covers the registered
POST routes that require forwarded JWT claims, so removing a guard cannot
silently remove the corresponding denial check. Use synthetic credentials and isolated local fixtures; never point these tests
at a deployment.

`support/request_boundaries.rs` includes a valid forwarded-claims control before
checking missing/malformed credentials, document permissions, role mutation
permissions, tenant separation and Datafix error responses. Datafix requests
carry valid bodies, so only the missing credentials can produce their error.
Synthetic JWTs model claims already verified by the identity gateway. They are
not a test of JWT signature verification or a substitute for deployment access
controls.

`support/error_contracts.rs` checks the error JSON consumed by the portals,
including password-policy counts and truncated profile-validation totals.
The audit-query suite checks bound values and closed SQL identifiers; import
tests check unchanged upload bytes and domain-error HTTP mappings.
Tests live outside `src` and are compiled into the binary's test target so they
can use its private routes without widening the production API.

From `packages`, run the focused suites with locked cached dependencies:

```sh
# Synthetic configuration for SQL generation; these tests do not contact a database.
export KEYCLOAK_DB__HOST=127.0.0.1 HASURA_DB__HOST=127.0.0.1
export LOW_SQL_LIMIT=1000 DEFAULT_SQL_LIMIT=20 DEFAULT_SQL_BATCH_SIZE=1000
cargo test -p harvest --locked --offline --bin harvest -- request_boundaries
cargo test -p harvest --locked --offline --bin harvest -- error_contracts
cargo test -p harvest --locked --offline --bin harvest -- boundary_tests
cargo test -p harvest --locked --offline --bin harvest -- services::access
```

For a complete native report, from the repository root:

```sh
python3 scripts/coverage/run.py harvest --baseline --offline
```

The package profile supplies public local SQL configuration. Complete route
tests also need the PostgreSQL fixture described below. Each run
clears old LLVM counters and workspace binaries, so compile-time macro counters
are collected again; external dependencies stay cached. Never store deployment
secrets in `test_environment`.

Role creation has a local HTTP integration control: the real route and Keycloak
client create the synthetic role with `ROLE_CREATE`, while read-only and
write-only claims cannot create it. The HTTP fixture is shared with Core. A fresh
child process clears ambient settings and isolates the global token cache;
its wait and socket operations are bounded, and LLVM instrumentation is retained.
This verifies the client protocol and authorization adapter, not a deployed
identity provider or JWT signatures.

`support/route_permissions.rs` holds one row per guarded route, or per request
shape where a route's permissions depend on its body, and must cover the whole
inventory. A second child sends each row's minimum permission set with no
backend configured: the request must pass authorization and stop at the first
backend. The same request with any one permission removed must get the route's
current denial status and content type. Routes that write a task row before
checking permissions answer the backend failure in both cases, and routes with
an empty permission list have no denial row.

Audit row tests include complete and reordered controls for both table names,
every missing/duplicated field, null/wrong types, malformed count row shapes,
and an empty ordering map. Publication mappings retain public 4xx explanations
while replacing internal details with a generic 500 message.

Access decisions are plain functions in `src/services/access.rs`: the
permissions each user-management request needs, the extra permission for
documents holding voter secrets, and the admin-or-trustee check. Routes pass
their results to `authorize` unchanged, so `support/access_policy.rs` pins the
permission tables, including the order a denial lists them in. Sequent Core's
`authorize_with` and `has_gold_permission_at` take the super-admin tenant and
the current time as arguments, so their tests need neither the environment nor
the wall clock.

`HarvestServices`, managed by Rocket, provides database pools, document storage,
Keycloak administration, the task ledger, task queue, cast-vote insertion and
secret storage. Production adapters call the existing Windmill helpers. Route
tests inject in-memory adapters and a bounded local HTTP peer for Keycloak;
SQL still runs against real PostgreSQL. Task rows, queued messages, stored
secrets and electoral-log entries are observable test results. The adapters
preserve typed errors and the existing order of authorization and side effects.

The database fixture in `support/schema.rs` creates a private database, applies
all Hasura backend migrations in version order, and creates the Keycloak table
subset its queries use. Every test gets fresh pools and synthetic tenants because
route handlers commit their transactions. The configured PostgreSQL user needs
permission to create databases. On an isolated PostgreSQL 16 server, export the
following synthetic configuration before running the complete suite:

```sh
# From packages/. The server must already be running on this local port.
export HASURA_DB__HOST=127.0.0.1 HASURA_DB__PORT=5432
export HASURA_DB__USER=test HASURA_DB__PASSWORD=test HASURA_DB__DBNAME=test
export KEYCLOAK_DB__HOST=127.0.0.1 KEYCLOAK_DB__PORT=5432
export KEYCLOAK_DB__USER=test KEYCLOAK_DB__PASSWORD=test KEYCLOAK_DB__DBNAME=test
export LOW_SQL_LIMIT=1000 DEFAULT_SQL_LIMIT=20 DEFAULT_SQL_BATCH_SIZE=1000
cargo test -p harvest --locked
```

The route suites exercise tally sheets and ceremonies, report generation,
publication and statistics, the phone blacklist, cast-vote responses,
voter-authentication settings and voter-information letters. They cover valid
requests, backend failures, permission denials, task dispatch and committed
state. Cast-vote tests additionally pin each typed error's HTTP response and the
retry boundary. Complete-permission controls for routes that still use globals
stop at their first backend; they do not claim full service workflows.

Deployed workers, identity-provider signatures, RabbitMQ, S3 and ImmuDB remain
outside this profile. Functions include generated routing and error closures;
keep these and uncovered service modules in the source inventory. Actual
branches and optional feature/target configurations remain separate obligations.

The child fixtures reject stale markers. A private temporary nonce selects
its child path, the parent owns cleanup, and unrelated ambient credentials must
not cause the fixture to skip its valid-control requests.
