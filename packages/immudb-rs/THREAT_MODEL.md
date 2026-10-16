<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# immudb-rs threat model

immudb-rs is the async gRPC client for immudb. `build.rs` generates the client stubs (`src/schema.rs`) from the vendored `proto/immudb/immudb.proto` with `tonic-build`, and `src/client.rs` wraps them in `Client` for login, sessions, database management, SQL and transactions. It runs in server-side processes only: windmill workers, harvest, and the operator tools in step-cli, windmill's `external-bin/generate_logs.rs` and immu-board's `bb_helper`. electoral-log and immu-board each build a `BoardClient` on it. Every electoral-log write and read goes through this client, and so do reads of the pgaudit tables kept in immudb. The client holds an immudb credential, so it matters both to the integrity of the audit trail and to the confidentiality of the voter identifiers stored there. See the [system threat model](../../THREAT_MODEL.md).

## Assets

- **immudb credential** (`IMMUDB_USER` / `IMMUDB_PASSWORD`). Confidentiality: whoever holds it can read, write, create and delete the databases the server lets that user reach.
- **Bearer token and session id**. Confidentiality while they are valid.
- **Electoral-log entries**: sender and system signatures, statement kind, user id, username, area id, election id and ballot id. Integrity and completeness for audit. Confidentiality for the voter identifiers.
- **pgaudit records** (`pgaudit_hasura`, `pgaudit_keycloak`) kept in immudb. Confidentiality, because they record SQL statements run against the Hasura and Keycloak databases.
- **Per-event immudb databases**. Availability: deleting one deletes that event's audit trail.
- **Protocol definitions and generated stubs** (`proto/immudb/`, `src/schema.rs`). Integrity.

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| `src/client.rs` `Client::new(server_url, username, password)` | windmill, harvest, electoral-log and immu-board `BoardClient::new`, step-cli, operator binaries | internal service, operator | `server_url` parsed by tonic |
| `src/client.rs` `Client::login`, `Client::open_session`, `Client::use_database`, `Client::logout`, `Client::close_session` | the same callers | internal service, operator | `get_request` attaches the token and session id as request metadata; the immudb server authenticates them |
| `src/client.rs` `Client::sql_exec`, `tx_sql_exec`, `sql_query`, `streaming_sql_query`, `tx_sql_query` | electoral-log `BoardClient`, harvest, windmill; values come from admin and voter requests | internal service carrying untrusted values | values bound as `NamedParam`; statement text sent unchanged |
| `src/client.rs` `Client::new_tx`, `commit`, `rollback` | windmill, step-cli | internal service | transaction id parsed into `MetadataValue` with `?`; `unsafe_mvcc: false` |
| `src/client.rs` `Client::create_database`, `delete_database`, `list_databases`, `has_database`, `has_tables` | electoral-log, windmill, `bb_helper` | internal service, operator | `if_not_exists: true` on create; no authorization in this crate |
| gRPC responses from the immudb server | the immudb server | internal service | prost decoding; gRPC message size limits |
| `build.rs` | build and CI | operator | compiles client stubs only (`build_server(false)`) from the vendored protos |
| `bin/install_admin.sh` | operator machine | operator | downloads a pinned `immuadmin` release (v1.3.0) from GitHub over HTTPS |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| immudb-rs-T1 | Information disclosure | The immudb password, bearer token or session id ends up in application logs | Restricted access to service logs (deployment) | Partial |
| immudb-rs-T2 | Information disclosure | Someone on the network path between a service and immudb reads the credential, the token or the log contents | Network isolation (deployment) | Partial |
| immudb-rs-T3 | Spoofing | A host that impersonates the immudb server collects the credentials sent to it or returns fabricated results | Network isolation (deployment) | Not verified |
| immudb-rs-T4 | Tampering | Electoral-log entries are altered, dropped or rolled back on the immudb server or in its storage, or a database is replaced, without detection | Restricted access to the immudb server and its storage (deployment) | Not verified |
| immudb-rs-T5 | Tampering | Untrusted values reach SQL statement text and change a query or a write | The SQL methods take `Vec<NamedParam>` for values; statement text is the caller's responsibility | Partial |
| immudb-rs-T6 | Elevation of privilege | A compromised service misuses its immudb credential to read, change or delete log databases | Rights of the immudb user are set on the server (deployment) | Not verified |
| immudb-rs-T7 | Repudiation | An election event's log database is deleted without authorization or without a record of who deleted it | The server's permissions limit who can delete a database (deployment) | Partial |
| immudb-rs-T8 | Denial of service | An unresponsive or hostile server, or a very large result, stalls or exhausts the calling worker or API process | gRPC message size limits; decoding failures return as errors | Partial |
| immudb-rs-T9 | Denial of service | Sessions or transactions left open use up the server's session capacity | `Client::close_session`, `Client::logout` and `Client::rollback`; server-side session expiry (deployment) | Partial |
| immudb-rs-T10 | Information disclosure | Log contents (voter identifiers, ballot ids, pgaudit statements) are copied into application logs | Restricted access to service logs (deployment) | Partial |
| immudb-rs-T11 | Denial of service | A malformed response or metadata value panics the calling process | `Client::get_request` checks `is_some()` before `expect`; session id, token and transaction id are parsed with `parse()?`; prost decode errors surface as `tonic::Status` mapped to `anyhow::Error` | Mitigated |
| immudb-rs-T12 | Tampering | A tampered `immuadmin` binary or protocol definition reaches an operator machine or the build | `bin/install_admin.sh` pins release v1.3.0 and fetches it over HTTPS; protos are vendored under `proto/immudb/` and `build.rs` compiles client stubs only | Partial |
| immudb-rs-T13 | Elevation of privilege | A user authorized for one tenant or election event reads or changes another tenant's or event's log | Authorization is done by the calling services; immudb-rs does none of its own | Not verified |

## Assumptions

- The deployment limits network access to immudb's gRPC port to windmill, harvest and operator tools.
- Operators set strong immudb credentials, limit their rights, and restrict who can read service logs and environments.
- The integrity of the electoral log is covered by the system threat model (SYS-T34).
- Callers build SQL text only from constants and enum-derived identifiers, and bind every value as a `NamedParam`.
- windmill and harvest authorize each admin or voter request before calling the client; immudb-rs does no authorization of its own.
- Callers close sessions and end transactions when they finish with them.
- The immudb server enforces authentication, per-database permissions and session limits, and runs a version compatible with the vendored `proto/immudb/immudb.proto`.

## Review focus

1. Handling of secrets and personal data in logs.
2. Transport security between services and immudb.
3. How the integrity of the electoral log is checked.
4. SQL statement construction in the callers.
5. Rights of the immudb user, authorization in the callers, and database management.
6. Resource limits and the session and transaction lifecycle.
7. Operator tooling supply chain (`bin/install_admin.sh`).
