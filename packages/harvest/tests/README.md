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
permissions, tenant separation and Datafix error responses. Synthetic JWTs model
claims already verified by the identity gateway. They are not a test of JWT
signature verification or a substitute for deployment access controls.

`support/error_contracts.rs` checks the error JSON consumed by the portals,
including password-policy counts and truncated profile-validation totals.
The audit-query suite checks bound values and closed SQL identifiers; import
tests check unchanged upload bytes and domain-error HTTP mappings.
Tests live outside `src` and are compiled into the binary's test target so they
can use its private routes without widening the production API.

From `packages`, run the focused suites with locked cached dependencies:

```sh
cargo test -p harvest --locked --offline --bin harvest -- request_boundaries
cargo test -p harvest --locked --offline --bin harvest -- error_contracts
```

For a complete native report, from the repository root:

```sh
python3 scripts/coverage/run.py harvest --baseline --offline
```

Omit `--baseline` to enforce the local 95% line improvement target. CI compares
lines, functions and LLVM regions separately against the actual PR base and
rejects any decrease; 95% is not its gate. The boundary slice does not establish that
package-wide target. Database transactions, complete identity-service workflows,
brokers, storage and end-to-end service workflows need explicit local fixtures.
Keep Meta #13295 open until the measured profile and its remaining gaps satisfy
the acceptance criteria. Every measured source file remains in the denominator.

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
identity provider or JWT signatures.

Audit row tests include complete and reordered controls for both table names,
every missing/duplicated field, null/wrong types, malformed count row shapes,
and an empty ordering map. Publication mappings retain public 4xx explanations
while replacing internal details with a generic 500 message. Six regressions failed on the original production implementation and pass after
the small guard/mapping fixes. Follow-up coverage should use existing interfaces;
service architecture changes solely for testability are outside this slice.

## Measured checkpoint and accepted limits

Source `20cb15c63a6f43edc13336074d5e01c0bdc9944d`, isolated native profile,
Rust 1.96.0 and cargo-llvm-cov 0.9.1: **43 tests pass, none ignored**;
**676/3,924 lines (17.23%)**, **64/879 functions (7.28%)** and
**788/3,455 LLVM regions (22.81%)**. The preceding 34-test suite measured
651/3,906 lines (16.67%), 63/879 functions (7.17%) and 758/3,437 regions
(22.05%). Production Clippy and workspace formatting pass with existing
warnings; 76 coverage-tool tests and Ruff pass.

Low coverage is an accepted limit of this increment. The remaining 3,248 lines
and 815 functions stay counted. Most route bodies require configured database
transactions, Keycloak, brokers, storage or worker state; the current denial
checks exercise their entry guards, not complete service workflows. Functions
also include generated routing and error closures. The successful role-creation
protocol control adds assurance even though its containing function was already
entered by denial tests. No generated functions or uncovered service modules
were excluded, and no production architecture was rewritten for testability.

Further service-backed coverage needs explicit bounded local fixtures. Actual
branches, deployed JWT validation and optional feature/target configurations
remain separate obligations; this native LLVM result does not close them.
