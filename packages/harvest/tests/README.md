<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Harvest request boundary tests

This coverage slice exercises actual Rocket routes, request guards and
error catchers with the local HTTP client. It uses the same route builder as production while avoiding startup of
service workers, probes and plugins. A static inventory covers the registered
POST routes that require forwarded JWT claims, so removing a guard cannot
silently remove the corresponding denial check. No production credentials or
external services belong in these tests.

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
```

For a complete native report, from the repository root:

```sh
python3 scripts/coverage/run.py harvest --baseline --offline
```

The package profile supplies public local SQL configuration for the existing
query-builder tests; no database is contacted by this Harvest slice. Coverage
clears runtime counters while retaining compiled dependencies for fast repeats.
Proc-macro profiles request a rebuild because their counters also run during
compilation. Never store deployment secrets in `test_environment`.

Role creation has a local HTTP integration control: the real route and Keycloak
client create the synthetic role with `ROLE_CREATE`, while read-only and
write-only claims cannot create it. The HTTP fixture is shared with Core. A fresh
child process clears ambient settings and isolates the global token cache;
its wait and socket operations are bounded, and LLVM instrumentation is retained.
This verifies the client protocol and authorization adapter, not a deployed
identity provider or JWT signatures. A second child sends each guarded route's
complete permission set with no backend configured: the request must pass
authorization and stop at the backend with HTTP 500, so a route that required
a different permission would fail its control.

Audit row tests include complete and reordered controls for both table names,
every missing/duplicated field, null/wrong types, malformed count row shapes,
and an empty ordering map. Publication mappings retain public 4xx explanations
while replacing internal details with a generic 500 message. Use existing interfaces for additional failure fixtures; do not rewrite the
service architecture solely for testability.

Most route bodies require configured database
transactions, Keycloak, brokers, storage or worker state; the denial checks and
complete-permission controls exercise their entry guards, not complete service
workflows. Functions
also include generated routing and error closures. The successful role-creation
protocol control adds assurance even though its containing function was already
entered by denial tests. Keep generated functions and uncovered service modules in the source inventory.

Further service-backed coverage needs explicit bounded local fixtures. Actual
branches, deployed JWT validation and optional feature/target configurations
remain separate obligations; this native LLVM result does not close them.

The child fixtures reject stale markers. A private temporary nonce selects
its child path, the parent owns cleanup, and unrelated ambient credentials must
not cause the fixture to skip its valid-control requests.
