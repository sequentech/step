---
id: test-coverage
title: Windmill boundary tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Test file paths in this guide are relative to [`packages/windmill/tests/`](https://github.com/sequentech/step/blob/main/packages/windmill/tests). Commands state their working directory.

These public integration tests cover encryption-password confidentiality,
canonical import arithmetic and errors, material changes in reviewed tally
sheets, CSV display fidelity, and SQL escaping through PostgreSQL's real parser.
No test sends mail, SMS or election data to an external service.

Use PostgreSQL 16 on loopback port 3322 with the synthetic `test` user/password
and database, matching `.github/workflows/tests.yml`. Each query test uses its
own transaction; the unsafe-string-mode test uses a separate connection option.
It never changes shared database defaults. The coverage profile provides these
public fixture settings; it must run in the isolated worker, not on production.

The existing ignored voter-channel PostgreSQL regression is run explicitly as
separate evidence. The other ignored activity-log test requires a fuller
service fixture and must remain visible in the report. Native default-feature
coverage does not certify cloud transports, full election services, optional
features or branches. Keep untested workers in the measured source scope.

With PostgreSQL running, measure from the repository root:

```bash
python3 scripts/coverage/run.py windmill --baseline --offline
```

Run the existing voter-channel regression explicitly from `packages/`:

```bash
KEYCLOAK_DB__HOST=127.0.0.1 \
HASURA_DB__HOST=127.0.0.1 HASURA_DB__PORT=3322 \
HASURA_DB__USER=test HASURA_DB__PASSWORD=test HASURA_DB__DBNAME=test \
LOW_SQL_LIMIT=1000 DEFAULT_SQL_LIMIT=20 DEFAULT_SQL_BATCH_SIZE=1000 \
cargo test -p windmill --locked --offline --lib -- \
  services::cast_votes::tests::voters_by_channel_defaults_legacy_votes_and_uses_latest_valid_revote \
  --ignored --exact
```

This separate command does not add its counters to a previous coverage report.

Service fixtures are needed for cloud transports and complete authenticated
election workflows; these need dedicated service fixtures, not production
rewrites or tests that only exercise derives.

## Ports and adapters

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
