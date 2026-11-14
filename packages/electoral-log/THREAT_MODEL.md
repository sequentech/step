<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# electoral-log threat model

electoral-log is the Rust library behind the electoral log, the platform's tamper-evident audit trail. `src/messages/` builds a `Message` for each audited action: cast votes and cast-vote errors, voting-period changes, key and tally ceremonies, publications, Keycloak user events, external API calls and reconciliation runs. It signs the message's `Statement` with a sender key and the system key. `src/client/board_client.rs` (`BoardClient`) creates each event's immudb database and table, inserts entries and runs filtered, paged and counted reads. The library runs in the windmill workers, in harvest (directly and through windmill), and in step-cli on an operator's machine. `src/bin/bb_helper.rs` is an operator CLI that creates or deletes one log database. The package holds no keys or credentials of its own: callers pass in the signing keys and the immudb credentials. Admins, voters (through the ballot locator) and auditors use the log to show what happened in an election, so its authenticity, completeness and confidentiality matter. See the [system threat model](../../THREAT_MODEL.md).

## Assets

- **Electoral-log entries** (`electoral_log_messages` rows in each event's immudb database): integrity and completeness of the audit trail, availability.
- **Signing keys passed in `SigningData`** (sender and system `StrandSignatureSk`, `src/messages/message.rs`): confidentiality. Anyone who holds them can write entries that verify.
- **immudb credentials and session tokens** passed in by callers: confidentiality. They give access to every log database the immudb account can reach.
- **Personal data inside entries**: user ids, usernames, voter IP and country, phone numbers, certificate subject names, and old and new voter values from reconciliation runs. Confidentiality.
- **Encoding stability** of `Statement`, `StatementBody` and `StatementType` (Borsh): integrity. Stored entries must keep decoding and verifying.

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| `src/messages/message.rs` `Message::*_message` constructors and `Message::sign` | windmill and harvest code recording an action. Part of the content comes from voters, admins, Keycloak events and the external voter registry | internal service | Typed newtypes and enums in `src/messages/newtypes.rs`; head built by `StatementHead::from_body` (`src/messages/statement.rs`) |
| `src/messages/message.rs` `Message::verify` | Any consumer holding the event's system public key | internal service / auditor | Checks the sender signature against the sender key carried in the message and the system signature against the key the caller passes |
| Borsh decoding (`strand_deserialize`) of stored `message` bytes into `Message` | windmill reports and ballot locator, step-cli exports | internal service | Decoding returns errors; variants are append-only |
| `src/client/board_client.rs` `BoardClient::new` | windmill, harvest and step-cli with an immudb URL and credentials | internal service / operator | immudb login |
| `BoardClient::insert_electoral_log_messages`, `insert_electoral_log_messages_batch` with `open_session`, `new_tx`, `commit` | windmill writers | internal service | Values bound as `NamedParam`; one immudb transaction per call; column sizes fixed in the table definition |
| `BoardClient::get_electoral_log_messages_filtered`, `count_electoral_log_messages` | windmill services and reports (including the voter ballot locator, served through harvest); step-cli exports. The admin Logs tab queries immudb through immudb-rs and does not use `BoardClient` | internal service | Column names and operators from `ElectoralLogVarCharColumn` and `SqlCompOperators`; values bound as parameters |
| `BoardClient::get_electoral_log_messages_batch`, `get_electoral_log_messages_at_offset`, `get_electoral_log_messages` | windmill activity-log CSV and PDF exports (batch and offset reads); `get_electoral_log_messages` only in tests | internal service | `i64` ids, limits and offsets; streaming reads |
| `BoardClient::upsert_electoral_log_db`, `delete_database`, `has_database` | windmill election-event import (upsert); `bb_helper` (upsert and delete); tests | internal service / operator | `CREATE TABLE` and `CREATE INDEX` with `IF NOT EXISTS`; delete only if the database exists |
| `ElectoralLogMessage` `TryFrom<&Row>` | immudb query responses | internal service | Typed parsing of each column |
| `src/bin/bb_helper.rs` (`upsert-board-db`, `delete-board-db`) | Operator with immudb credentials | operator | Credentials from flags or `IMMUDB_*` environment variables. The deployed immudb-init images build immu-board's `bb_helper`, not this one |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| electoral-log-T1 | Spoofing | An entry is forged to look as if the platform or a given user wrote it | `Message::sign` signs the Borsh `Statement` with the sender key and the system key, and `Message::verify` checks both. The sender key travels inside the message, so it identifies a user only when matched against a `VoterPublicKey` or `AdminPublicKey` entry | Not verified |
| electoral-log-T2 | Tampering | The content of a signed statement is changed after signing | The signatures cover the whole `Statement` (`src/messages/statement.rs`); the test `publication_failures_are_signed_errors_with_task_and_publication_context` checks that a changed body fails `verify` | Partial |
| electoral-log-T3 | Tampering | Stored entries are altered or duplicated after they are written | immudb append-only storage (deployment) | Not verified |
| electoral-log-T4 | Repudiation | A user denies an action the log attributes to them | The caller supplies the sender key through `SigningData::new`; key custody and the key-binding entries belong to windmill | Partial |
| electoral-log-T5 | Repudiation | Entries are dropped or rolled back, or the log database is replaced, at the storage layer without anyone noticing | immudb append-only storage (deployment) | Not verified |
| electoral-log-T6 | Tampering | Readers get incomplete results without noticing | immudb storage guarantees (deployment) | Not verified |
| electoral-log-T7 | Elevation of privilege | SQL injection through filters, ordering, limits or ids | Values bound as `NamedParam`; column names and operators come from the `ElectoralLogVarCharColumn` and `SqlCompOperators` enums, and windmill's ordering from its `OrderField` and `OrderDirection` enums; ids, limits and offsets are integers | Partial |
| electoral-log-T8 | Information disclosure | immudb credentials or session tokens end up in application logs | Restricted access to logs (deployment) | Not verified |
| electoral-log-T9 | Information disclosure | Personal data in entries reaches readers who do not need it, or cannot be erased from the append-only store | `ExternalApiRequest` descriptions keep only the operation and its outcome (`StatementHead::from_body`) | Partial |
| electoral-log-T10 | Information disclosure, Tampering | A caller reads or writes another tenant's or event's log | One immudb database per event; access control belongs to the calling services | Not verified |
| electoral-log-T11 | Denial of service | Large or repeated reads exhaust worker memory or immudb | Paged and streaming reads; immudb-rs caps gRPC messages at 128 MiB | Partial |
| electoral-log-T12 | Denial of service | Audit writes fail and the action goes unrecorded | Insert errors are returned to the caller | Partial |
| electoral-log-T13 | Tampering | A change in variant order makes stored entries decode as a different statement or stop verifying | Variants are append-only, pinned by the tests `statement_body_borsh_discriminants_are_append_only`, `statement_type_borsh_discriminants_are_append_only` and `legacy_cast_vote_body_remains_deserializable`; `get_cast_vote_channel_schema_version` sets the `version` column | Accepted (the `StatementBody` comment documents that `ExternalReconciliation` and `CastVoteWithChannel` have different discriminants on release/10.0 and main, so readers must match the writer's branch) |
| electoral-log-T14 | Repudiation | An event's whole log database is deleted | `BoardClient::delete_database` is reached from `bb_helper delete-board-db` and the tests; windmill's election-event deletion drops the database through immudb-rs directly | Partial |
| electoral-log-T15 | Information disclosure | Operator CLI credentials leak through process arguments or shell history | `src/bin/bb_helper.rs` reads the password from `--password` or, when the flag is not given, from `IMMUDB_PASSWORD`, which keeps it out of the process arguments | Partial |

## Assumptions

- windmill protects the signing keys and passes the right one for each event in `SigningData`. This package stores no keys.
- harvest and windmill authorize every read and write of the log.
- RabbitMQ and immudb run on a private network with per-deployment credentials, immudb keeps data append-only, and the accounts the services use have only the rights they need.
- Auditors and other consumers that need authenticity verify entries against the event's system public key.
- Server clocks are correct: `timestamp()` in `src/lib.rs` sets the statement and row times.
- Every reader, including step-cli and external auditors, is upgraded before writers emit new `StatementBody` variants, as the `CastVoteWithChannel` comment requires.
- step-cli and `bb_helper` are run only by trusted operators.

## Review focus

1. Signing and verification of entries.
2. Who can write entries, and through which paths.
3. Completeness of reads and exports.
4. Secrets and personal data in logs.
5. Personal data in entries and who can read it.
6. Query construction in `BoardClient`.
7. The immudb deployment: network, accounts and storage guarantees.
8. Borsh enum order across release branches, and importing or exporting logs between them.
