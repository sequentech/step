<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# b3 threat model

b3 is the bulletin board shared by the braid trustees and the protocol manager in windmill. It has three parts. The message model (`src/messages/`) defines signed statements, their artifacts, the session `Configuration` and the `ProtocolManager` signer. It is linked into braid on trustee machines and into windmill workers. The gRPC server (`src/grpc/server.rs`, binary `src/bin/server.rs`, installed as `/usr/bin/b3` by `Dockerfile.prod`) runs in its own container. It stores messages in a dedicated PostgreSQL database, with one table per board, an `INDEX` table and an optional filesystem blob store. The clients are `src/client/grpc.rs`, which trustees use, and `src/client/pgsql.rs`, which windmill uses to read and write the database directly. The board carries the configuration, the DKG channels and encrypted shares, the election public key, ballot batches, mixes, decryption factors and plaintexts. Its integrity decides whether the published election key and the tally are genuine. Its availability decides whether a ceremony or tally can finish. By design the board is untrusted storage: its integrity rests on end-to-end signatures. See the [system threat model](../../THREAT_MODEL.md).

## Assets

- **Protocol manager signing key** (`src/messages/protocol_manager.rs` `ProtocolManager`, `ProtocolManagerConfig`): signs the configuration and every ballot batch. Confidentiality and integrity.
- **Configuration and `ConfigurationHash`** (`src/messages/artifact.rs`, `src/messages/newtypes.rs`): fix who may sign, the threshold, and the session that every statement is bound to. Integrity.
- **Election public key** (`DkgPublicKey`, `Statement::PublicKey` / `PublicKeySigned`): voters encrypt to it, so a substituted key breaks ballot secrecy. Integrity.
- **Encrypted DKG material** (`Channel.encrypted_channel_sk`, `Shares.encrypted_shares`, `TrusteeShareData`): trustee key shares. Confidentiality.
- **Ballots, mixes with shuffle proofs, decryption factors with proofs, plaintexts**: integrity and verifiability of the tally; plaintexts are results before publication (confidentiality).
- **Board log**: message order and completeness. Integrity and availability.
- **Board database credentials** (`B3_PG_*`): read and write access to every board. Confidentiality.
- **Board index** (`INDEX` table, `GetBoards`): board names, which identify elections. Low confidentiality.

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| `src/grpc/server.rs` `put_messages`, `put_messages_multi` (`put_messages_`) | braid trustees through `src/client/grpc.rs` `B3Client`; other clients the deployment lets reach `B3_BIND` | untrusted | `src/grpc/mod.rs` `validate_board_name`; schema version; `Message` decoding and canonical re-encoding |
| `src/grpc/server.rs` `get_messages`, `get_messages_multi` (`get_messages_`) | trustees, the braid verifier, other clients the deployment lets reach `B3_BIND` | untrusted | `validate_board_name` |
| `src/grpc/server.rs` `get_boards` | trustees (board discovery), other clients the deployment lets reach `B3_BIND` | untrusted | takes no input; read-only |
| `src/client/pgsql.rs` `PgsqlB3Client`, `PooledPgsqlB3Client`, `create_database`, `drop_database` | windmill workers; the b3 server | internal service | PostgreSQL credentials |
| `src/messages/message.rs` `Message` decoding (strand `StrandDeserialize`) and `Message::verify` | braid trustees and verifier, windmill, on bytes read from the board | untrusted input; trustee or internal-service caller | Borsh decoding must consume all input; mix-number range; sender in the configuration (`Configuration::get_trustee_position`); signature over the statement; `ConfigurationHash` |
| `src/messages/protocol_manager.rs` `ProtocolManagerConfig::get_signing_key` | windmill, with the key read from its secrets backend | internal service | PKCS#8 DER decoding in strand |
| `src/bin/server.rs` `ServerConfig::from_env` (`B3_PG_*`, `B3_BIND`, `B3_BLOB_ROOT`, `B3_MAX_MESSAGE_SIZE_BYTES`), `init_log` (`LOG_LEVEL`) | container environment | operator | parsing only |
| Blob store under `B3_BLOB_ROOT` (`put_messages_`, `get_messages_`) | the b3 server process | internal service | file names stay inside the board directory; off unless configured |
| `src/bin/m2.rs` monitor | operator | operator | not built: the `monitor` feature is commented out in `Cargo.toml` |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| b3-T1 | Spoofing | A message is forged in the name of a trustee or the protocol manager | `src/messages/message.rs` `Signer::sign` signs the Borsh-encoded `Statement` (Ed25519 through strand's dalek backend; ECDSA P-384 under strand `openssl_full`); `Message::verify` looks up the sender with `Configuration::get_trustee_position`, then checks the signature and the `ConfigurationHash` | Not verified |
| b3-T2 | Tampering | Artifact bytes are swapped under a valid signed statement | Every `Message::*_msg` constructor puts the SHA-512 of the artifact into the signed statement | Not verified |
| b3-T3 | Elevation of privilege | A configured party signs a statement reserved for another role | Signatures tie each statement to one configured key (`Message::verify`) | Not verified |
| b3-T4 | Spoofing | An untrusted or invalid configuration becomes a session's trust anchor | `src/messages/artifact.rs` `Configuration::is_valid` (distinct trustees, 2 to `MAX_TRUSTEES`, 1 < threshold <= n); `Message::verify` checks each statement's `ConfigurationHash` | Not verified |
| b3-T5 | Tampering | A message is replayed across boards or sessions, or duplicated within one | Every `Statement` variant carries the `ConfigurationHash` (`src/messages/statement.rs`) | Partial |
| b3-T6 | Denial of service | The board operator or database holder withholds, delays or reorders messages | `BIGSERIAL` ids assigned under `pg_advisory_xact_lock` in `insert` / `insert_copy`; `ORDER BY id` in `get`; trustees cross-sign shared hashes | Accepted (by design: the board is untrusted storage and can always stall the protocol) |
| b3-T7 | Tampering | A network attacker reaches the gRPC listener or the board database, or reads or alters traffic to them | End-to-end signatures (b3-T1, b3-T2); network isolation and transport security (deployment) | Not verified |
| b3-T8 | Denial of service | Malformed or hostile encodings crash the server or a reader | Group elements and scalars are decoded by strand | Partial |
| b3-T9 | Denial of service | Server resources are exhausted by large or numerous requests | Configurable message size limit; chunked insert transactions; bounded database pool | Partial |
| b3-T10 | Tampering | A board name injects into SQL identifiers or blob paths | Board-name validation (ASCII alphanumerics and `_`); row values bound as query parameters | Not verified |
| b3-T11 | Tampering | In blob-store mode, message files are tampered with or removed | Blob file names are a single path component, so they stay inside the board directory; the mode is off unless `B3_BLOB_ROOT` is set | Not verified |
| b3-T12 | Information disclosure | The protocol manager key or the database credentials leak | The dalek signing key is zeroized on drop; host access control (deployment) | Not verified |
| b3-T13 | Information disclosure | Board names or contents leak across tenants or election events | DKG secrets are encrypted before they are posted (`Channel.encrypted_channel_sk`, `Shares.encrypted_shares`); network isolation (deployment) | Not verified |
| b3-T14 | Tampering | Proofs or encrypted key material are reused across trustees, batches or boards | Statements bind the `ConfigurationHash` (b3-T5) | Not verified |
| b3-T15 | Repudiation | A trustee or the protocol manager denies a statement it posted | Signature over the full statement, which includes the artifact hash and the `ConfigurationHash` (`Signer::sign`, `Message::verify`) | Mitigated |
| b3-T16 | Spoofing | A party that holds no configured key posts messages to a board | Signatures (b3-T1); network isolation (deployment) | Not verified |
| b3-T17 | Denial of service | A configured party (a trustee or the protocol manager) posts signed but malformed messages that disrupt the protocol | Signatures identify the sender (b3-T15) | Partial |
| b3-T18 | Repudiation | A holder of database credentials deletes or rewrites a board's history after the fact | Rewritten content fails signature and hash checks (b3-T1, b3-T2); trustees keep their own local copies of what they read; deletion is an intended windmill operation (election-event deletion) | Partial |

## Assumptions

- braid trustees and the braid verifier verify every message, and each artifact against its statement hash, against a configuration they trust.
- windmill, as protocol manager, keeps each board's protocol manager key in its secrets backend.
- The deployment limits network access to the gRPC listener and the board database to trustees and internal services, and protects their traffic.
- The board database user is a dedicated role with rights only on the board database.
- Writers to the board database other than the gRPC server are trusted internal services or operators.
- strand provides correct signatures, SHA-512, canonical decoding of group elements and scalars, AEAD for channel keys and key-generation randomness. b3 draws no randomness of its own.

## Review focus

1. Message verification and its tests.
2. Session configuration handling.
3. Access control on the gRPC server and the board database.
4. Robustness of message decoding.
5. The PostgreSQL client.
6. Server resource limits.
7. Blob-store mode.
8. Domain separation of signatures, proofs and encryption.
9. Handling of database credentials and protocol manager key material.
10. Replay protection.
11. The runtime image and deployment configuration.
