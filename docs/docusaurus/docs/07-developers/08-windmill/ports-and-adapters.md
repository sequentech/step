---
id: ports-and-adapters
title: Ports and adapters
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Windmill separates business rules from the systems it talks to, so the rules
can be unit-tested without PostgreSQL, Keycloak, the bulletin board, ImmuDB or
S3.

| Module | Contains | May depend on |
| --- | --- | --- |
| `src/domain` | Pure rules: validations, state transitions, decisions. No I/O, no `async`, and the current time is passed in. | `sequent-core` types |
| `src/ports` | Traits for each outside capability a service needs. | `domain` |
| `src/adapters` | Production implementations that wrap `src/postgres` queries and service clients. `adapters::memory` holds in-memory implementations for tests. | everything |
| `src/services` | Use cases written against ports. Their public functions keep their signatures: they build the production adapters and call the use case. | `domain`, `ports` |
| `src/tasks` | Celery entry points. They own transactions and commit them. | everything |

Conventions:

- Port methods return `impl Future<Output = anyhow::Result<T>> + Send`, and
  port traits require `Sync`, so services stay usable from Celery tasks and
  Harvest. Implementations may use `async fn`.
- PostgreSQL adapters borrow the caller's `Transaction`. Services never commit.
- Error types that Harvest maps to HTTP statuses are part of the contract:
  return them unchanged.
- Unit tests use the in-memory adapters and assert outcomes: returned values,
  stored rows, audit entries and errors. They do not assert which calls were
  made.

Tally execution uses pure plans for eligible trustees, board-message replay,
and whether an execution waits for tie resolution or completes. Trustee
ordering is an injected resource so tests can select a deterministic subset.
The execution ledger borrows the task's transaction and records the snapshot
before changing the session status. Its ports retain Windmill's task error type
to preserve conversions from database and serialization failures. In-memory
ledger tests inject failures at each write and check which state was persisted;
the outer task still controls commit and rollback.

Run the unit tests from `packages/`:

```bash
cargo test -p windmill --lib
```

Files that only declare modules or traits have no measured lines. List them in
`scripts/coverage/profiles.toml` under `profiles.windmill.scope_exceptions`, or
the coverage gate reports them as unaccounted.
