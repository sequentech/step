<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# electoral-log

This package contains the electoral log client and related helpers for ImmuDB-backed audit storage.

## Running Tests

Unit tests compile with the normal package test command:

```bash
cd /workspaces/step/packages
cargo test -p electoral-log
```

The ignored integration-style test in `src/client/board_client.rs` requires a reachable ImmuDB server.

By default, the test connects to `http://127.0.0.1:3322` using the default credentials `immudb` / `immudb`.

If your ImmuDB instance is reachable at a different hostname or port, set `ELECTORAL_LOG_IMMUDB_URL` before running the ignored test:

```bash
cd /workspaces/step/packages/electoral-log
ELECTORAL_LOG_IMMUDB_URL=http://immudb:3322 cargo test -- --ignored
```

Examples:

```bash
ELECTORAL_LOG_IMMUDB_URL=http://127.0.0.1:3322 cargo test -- --ignored
ELECTORAL_LOG_IMMUDB_URL=http://immudb:3322 cargo test -- --ignored
ELECTORAL_LOG_IMMUDB_URL=http://b3:3322 cargo test -- --ignored
```