<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# immu-board threat model

immu-board is an early immudb client for the electoral-log board (`src/board_client.rs`) plus the `bb_helper` command-line tool (`src/bin/bb_helper.rs`). On `release/10.0` the crate is a workspace member, but no other workspace crate depends on the library: windmill, harvest and step-cli use `../electoral-log`, which has its own board client, and the only other reference is the unbuilt `../board-messages` test module. Its live use is `bb_helper upsert-board-db`, the entrypoint of the `immudb-init` container (`../Dockerfile.immudb-init.prod`, built by CI, and `../../.devcontainer/docker-compose-*.yml`), which runs at deployment start on the platform network and creates the board database in immudb. It holds immudb credentials, can create and delete log databases, and duplicates code that the live audit log depends on, which is why it matters to an election. See the [system threat model](../../THREAT_MODEL.md).

## Assets

- **immudb credentials and session tokens**. Confidentiality: they give access to the log databases.
- **Board and electoral-log databases in immudb** (the database named by `--board-dbname`, and any event database a caller names). Integrity and availability: they are the tamper-evident audit trail.
- **Electoral-log entries passing through `BoardClient`** (`ElectoralLogMessage`: `sender_pk`, `statement_kind`, signed `message` bytes, `user_id`, `username`). Integrity; `user_id` and `username` are personal data, so also confidentiality.
- **The `immudb-init` image**. Integrity: it runs with immudb credentials.

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| `src/bin/bb_helper.rs` `Cli` (arguments or environment), actions `upsert-board-db` and `delete-board-db` | Operator; the `immudb-init` container entrypoint | operator | clap `ValueEnum` limits actions and log levels; connection values are required |
| `immudb-init` container entrypoint (`../Dockerfile.immudb-init.prod`, `../../.devcontainer/docker-compose-base.yml`, `../../.devcontainer/docker-compose-remote.yml`) | Deployment orchestrator at start-up | operator | Runs only `upsert-board-db` |
| `src/board_client.rs` `BoardClient` (`new`, `get_electoral_log_messages`, `insert_electoral_log_messages`, `has_database`, `upsert_electoral_log_db`, `delete_database`) | Rust code that links the crate; none in the workspace on `release/10.0` | internal service | Values bound as `NamedParam`; table names from `Table::as_str`; `LIMIT` and `OFFSET` typed `usize`; database names sent as gRPC fields, not SQL |
| immudb gRPC responses parsed by `ElectoralLogMessage::try_from(&Row)` in `src/board_client.rs` | immudb server and the network path to it | internal service | `assign_value!` checks each column's value type; unknown columns are rejected |
| `src/util.rs` `get_event_board` | Library callers | internal service | Builds the board name from the tenant and event ids |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| immu-board-T1 | Spoofing | A host on the network path impersonates immudb or reads or alters its traffic | Network isolation (deployment) | Partial |
| immu-board-T2 | Information disclosure | immudb credentials or session tokens are disclosed | Credentials are supplied at run time, not stored by this package; secrets store and container access (deployment) | Partial |
| immu-board-T3 | Tampering | SQL injection through message fields, database names or paging values | `insert_electoral_log_messages` and `get_electoral_log_messages_from_db` bind every value as `NamedParam`; table names come from the `Table` enum; `LIMIT`/`OFFSET` are `usize`; `use_database`, `create_database` and `delete_database` take the name as a gRPC field | Mitigated |
| immu-board-T4 | Tampering | A compromised immudb server or network path returns altered, reordered or truncated log rows and the reader accepts them | Network isolation and immudb hardening (deployment) | Not verified |
| immu-board-T5 | Spoofing | A holder of immudb write access inserts entries under another sender's `sender_pk` | `insert_electoral_log_messages` stores caller-supplied fields as given; writing requires immudb credentials | Accepted (storage layer by design) |
| immu-board-T6 | Repudiation | A batch insert leaves the audit log incomplete or with duplicates | `insert_electoral_log_messages` runs the batch in one `TxMode::ReadWrite` transaction inside a session and returns an error when a statement or the commit fails | Not verified |
| immu-board-T7 | Repudiation | A whole board database, and with it the audit trail, is deleted | `delete-board-db` must be named explicitly (`Action::DeleteBoardDb`); `BoardClient::delete_database` acts only on an existing database | Partial |
| immu-board-T8 | Elevation of privilege | One tenant's or event's log is read or written by a caller acting for another | `get_event_board` derives the name from tenant and event ids; the database is selected per call with `use_database`/`open_session` | Partial |
| immu-board-T9 | Denial of service | Reading a very large log exhausts the reader's memory | `get_electoral_log_messages` queries in pages of `IMMUDB_DEFAULT_LIMIT` rows | Partial |
| immu-board-T10 | Denial of service | Malformed rows from immudb crash or stall the reader | `ElectoralLogMessage::try_from` and `assign_value!` return errors for unknown columns and unexpected value types | Partial |
| immu-board-T11 | Tampering | A tampered dependency or base image ends up in the `immudb-init` image | CI and image registry controls (deployment) | Not verified |
| immu-board-T12 | Tampering | This client drifts from `../electoral-log`, so a fix or review lands in one copy only | No other workspace crate depends on this library; windmill uses `../electoral-log` | Partial |

## Assumptions

- immudb authenticates every request and is reachable only from the platform network.
- Deployments set their own immudb credentials from a secrets store, not the development defaults.
- Entry signing and verification happen outside this package; it only stores and returns entries.
- Operators restrict who can run `bb_helper delete-board-db` and keep backups of the log databases.
- Only operators can access the platform's containers.
- CI builds the `immudb-init` image from reviewed sources.
- Callers derive board names only from trusted tenant and event ids.

## Review focus

- Handling of immudb credentials and the connection to immudb.
- Tamper evidence of the log from write to read.
- Atomicity of batch inserts.
- The `immudb-init` image build and runtime.
- Duplication with `../electoral-log`, and whether this crate can be retired or reduced to the init tool.
- Row parsing and memory use with a misbehaving server.
