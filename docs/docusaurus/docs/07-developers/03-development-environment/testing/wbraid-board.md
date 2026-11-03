---
id: wbraid-board
title: Wbraid board storage tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

The board service is supplied by `packages/wbraid/crates/b4`, binary `b4v6`.
It stores opaque canonical protocol messages inline or through S3. Configuration,
trustee membership and protocol signatures are verified by its clients.

Run from `packages/` in the pinned devenv shell:

```sh
CARGO_TARGET_DIR="$PWD/rust-local-target" cargo test --manifest-path wbraid/Cargo.toml --locked -p b4 --features sqlite,postgres --lib
```

SQLite tests own temporary files. Storage and HTTP tests cover message ordering,
exclusive pagination, malformed requests, missing boards, inline/S3 boundaries,
foreign keys and durable reads. Archived boards retain their existing messages
and reject new insertions at the storage boundary.

Harvest's authenticated board writes use Windmill's trustee membership lookup.
`packages/windmill/tests/postgres_trustee_boards.rs` checks membership of the
same tenant and keys ceremony, missing claims, unknown trustees and boards,
lookup failures, and the report errors existing callers receive. The fixture
creates a private database from the Hasura migrations and rolls back each test.
With `HASURA_DB__*` pointing to a disposable PostgreSQL server, run from
`packages/` in the pinned devenv shell:

```sh
CARGO_TARGET_DIR="$PWD/rust-local-target" cargo test -p windmill --test postgres_trustee_boards
```

Application audit records use the independent immudb backend. Its disposable
database contracts are described in the [electoral-log test guide](./electoral-log.md).
