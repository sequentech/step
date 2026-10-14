---
id: electoral-log
title: Electoral Log tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Test file paths in this guide are relative to [`packages/electoral-log/tests/`](https://github.com/sequentech/step/blob/main/packages/electoral-log/tests). Commands state their working directory.

The electoral log keeps its records in PostgreSQL, with Trellis committing them
to a Merkle log in the same transaction. The
[Electoral Log design](../../13-electoral-log/01-electoral-log-design.md#15-testing)
lists what each suite checks.

## Without a database

From the repository root:

```bash
cd packages
cargo test -p electoral-log --locked
cargo test -p trellis --features upstream-service --lib --example monitor --example post --locked
```

These run the unit tests (record encoding, checkpoint signing, audit findings,
tree arithmetic), the ballot box seal vectors and the message suites: real
sender and system signatures, altered statements, JSON and
Borsh interchange, fixed wire tags, malformed encodings, every truncated message
prefix and writers that fail at each payload boundary. The existing protocol
signs the statement; search metadata and the separate artifact are not
authenticated by `Message::verify`, and these tests make no broader claim.

## Against PostgreSQL

The PostgreSQL contract, ballot box, event database, provisioning and console
tests are ignored by default. They need a disposable PostgreSQL server and a role that may
create roles and databases; each test creates its own databases and partitions
and drops them at the end. Never point them at a server that holds real data.

```bash
cd packages
export ELECTORAL_LOG_TEST_DATABASE_URL="postgresql://test:test@127.0.0.1:5432/test?sslmode=disable"
export ELECTORAL_LOG_PG_HOST=127.0.0.1 ELECTORAL_LOG_PG_PORT=5432 \
  ELECTORAL_LOG_PG_USER=test ELECTORAL_LOG_PG_PASSWORD=test \
  ELECTORAL_LOG_PG_DATABASE=test ELECTORAL_LOG_PG_SSLMODE=disable \
  ENV_SLUG=electoral-log-tests
cargo test --locked -p electoral-log --lib --test postgres --test ballot_box --test events --test provisioning --test console -- --ignored --test-threads=1
cargo test --locked -p windmill postgres_wiring_tests --lib -- --ignored --test-threads=1
```

`step-dev test` runs the electoral-log part as the `cargo-test-db:electoral-log`
check, against a server on `127.0.0.1:5432` with a `test` role.

The Trellis regression needs an empty database of its own:

```bash
createdb trellis_security
TRELLIS_TEST_DATABASE_URL="postgresql://test:test@127.0.0.1:5432/trellis_security?sslmode=disable" \
  cargo test --locked -p trellis --features upstream-service --lib \
  service::processor::tests::postgres_source_reconciliation_and_tree_recovery \
  -- --ignored --exact --test-threads=1
```

The Windmill job of the Tests workflow runs all three on PostgreSQL 16. It is
selected when Windmill, the electoral log or Trellis change.

## Coverage

The `electoral-log-native` coverage profile measures the default features. The
ignored PostgreSQL tests do not run in it, so the store, ballot box and event
database adapters are measured only by the tests that need no database. Native
branch coverage and doctests are not measured.
