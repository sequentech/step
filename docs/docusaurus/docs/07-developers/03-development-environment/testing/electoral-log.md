<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Electoral log

`packages/electoral-log` stores signed application audit records through the
supplied wbraid opaque board store and maintains a recoverable PostgreSQL query
index. Protocol and audit storage use independent databases.

Run default contracts in the development shell:

```sh
devenv shell -- bash -c 'cd packages && CARGO_TARGET_DIR="$PWD/rust-local-target" cargo test -p electoral-log --locked'
```

For storage integration, set `WBRAID_AUDIT_TEST_DATABASE_URL` to an isolated,
disposable PostgreSQL database. The test writes synthetic boards and injects an
index failure; never point it at an application database.

```sh
cargo test -p electoral-log --locked --features wbraid-storage-tests --test board_storage
```

The contract covers atomic multi-row delivery, receipt deduplication, mismatched
replays, bound filtering, stable pagination, index recovery and retained archived
messages. Windmill's signing-outbox, queue redelivery, seal and tally tests retain
their business coverage against this native adapter. The queue delivery suite
uses the same disposable URL plus its own RabbitMQ fixture. In a devcontainer,
set `WINDMILL_TEST_DOCKER_NETWORK` to its Docker network so the fixture is reached
directly; hosted workers use their daemon host’s loopback.

Runtime configuration uses `WBRAID_AUDIT_DATABASE_URL` and the optional explicit
`WBRAID_AUDIT_PG_PASSWORD`. These settings are independent of the public board
server's `WBRAID_B4_PG_*` overrides. Checked-in Compose deployments provide a
private `audit-postgres` service and an `audit-init` one-shot helper.

```sh
bb_helper --board-name audit-index upsert-board
bb_helper --board-name event-board archive-board
```

The `audit_ingest` binary reads one JSON delivery per stdin line. Each contains
`tenant_id`, `election_event_id`, `table` (`pgaudit_hasura` or `pgaudit_keycloak`),
`delivery_id` and `rows`. Each row contains `audit_type`, `class`, `command`,
`dbname`, `server_timestamp` (microseconds since the Unix epoch), `session_id`, `statement` and `user`.
Use a stable source identity such as file identity plus byte range. The tool
acknowledges the SHA-256 delivery ID on stdout only after persistence; collectors
must retain unacknowledged input for replay. Changing a receipt's payload is an
error. Access is through the private database credentials.

Harvest exposes `/audit/electoral-log` and `/audit/pgaudit-list`; the Hasura action
names and frontend result shapes remain unchanged. The latter requires `logs-read`
and scopes records by tenant, election event and source table.

No live legacy data is rewritten by this source migration. Existing backups must
be retained; imports validate application signatures and preserve signed bytes.
