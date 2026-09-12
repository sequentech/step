<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Harvest request boundary tests

The first coverage slice exercises actual Rocket routes, request guards and
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

Omit `--baseline` to enforce 95%. The boundary slice does not establish that
package-wide target. Database transactions, successful identity-service calls,
brokers, storage and end-to-end service workflows need explicit local fixtures.
Keep Meta #13295 open until the measured profile and its remaining gaps satisfy
the acceptance criteria. Every measured source file remains in the denominator.

The package profile supplies public local SQL configuration for the existing
query-builder tests; no database is contacted by this Harvest slice. Coverage
clears runtime counters while retaining compiled dependencies for fast repeats.
Proc-macro profiles request a rebuild because their counters also run during
compilation. Never store deployment secrets in `test_environment`.

A red/green request regression caught role creation accepting `ROLE_READ`.
The route now requires `ROLE_WRITE`; read-only claims are rejected before the
Keycloak client is constructed. This proves the local permission boundary,
not a successful identity-provider integration.
