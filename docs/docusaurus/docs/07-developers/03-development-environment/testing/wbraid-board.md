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

Application audit records use the independent immudb backend. Its disposable
database contracts are described in the [electoral-log test guide](./electoral-log.md).

PostgreSQL TLS integration tests start disposable loopback PostgreSQL servers and
generate short-lived synthetic certificates with OpenSSL. Install the complete
PostgreSQL server tools (`initdb` and `pg_ctl`), then run from `packages/`:

```sh
PG_BIN=/path/to/postgresql/bin OPENSSL_BIN=/path/to/openssl \
CARGO_TARGET_DIR="$PWD/rust-local-target" cargo test --manifest-path wbraid/Cargo.toml --locked -p b4 --features postgres --test postgres_tls
```

The fixture never uses a shared database. It checks verified TLS, wrong CA,
untrusted certificates, hostname mismatch, missing CA configuration, plaintext
refusal, explicit local mode and transaction advisory locks. See
[native PostgreSQL TLS configuration](../../09-wbraid/postgresql-tls.md).
