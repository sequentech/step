<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# braid threat model

braid is the trustee software: distributed key generation (DKG), verifiable re-encryption mixnet and threshold decryption over the b3 bulletin board. Each trustee runs one `trustee` process (`src/bin/main.rs`, started by `scripts/trustee.sh` in the `Dockerfile.prod` and `Dockerfile.prod-vstl-build` images) on a trustee machine. The process listens on no port; it polls b3 over gRPC and takes part in the boards of every tenant and election event that use this trustee. No other workspace crate links braid and it has no WASM build. braid produces the election public key and the decrypted ballots, so its checks decide whether ballot secrecy holds and whether the tally is correct. Auditors run the `verify` binary on their own machines to recheck a finished board. See the [system threat model](../../THREAT_MODEL.md).

## Assets

- **Trustee signing key** (`TrusteeConfig.signing_key_sk`, `src/protocol/trustee2.rs`): one key signs the trustee's statements on every board. Confidentiality and integrity.
- **Trustee symmetric key** (`TrusteeConfig.encryption_key`): encrypts the trustee's channel secret key, which is published on each board. Confidentiality.
- **DKG secrets**: channel secret keys, polynomial coefficients, received shares and the combined secret share. Any `threshold` of the shares rebuild the election secret key. Confidentiality.
- **Shuffle permutation and re-encryption randomness**: they link input ballots to mixed ballots. Confidentiality.
- **Election public key and verification keys** (`DkgPublicKey`): voters encrypt to the public key, and decryption proofs are checked against the verification keys. Integrity.
- **Board transcript**: mix chain, decryption factors and plaintexts, meaning the result and the evidence for it. Integrity and availability.
- **Local message store** (`message_store/<board>`): the trustee's own ordered view of each board. Integrity and availability.
- **Trustee process availability**: one process serves every board, so stopping it stops every election that uses this trustee.

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| `src/bin/main.rs` `main` (flags `--b3-url`, `--trustee-config`, `--strict`; env `TRUSTEE_NAME`, `IGNORE_BOARDS`) | Container runtime | operator | Key file parsed as TOML; signing key as DER/base64, symmetric key as base64 |
| `scripts/trustee.sh` (env `SECRETS_BACKEND`, `TRUSTEE_CONFIG_PATH`, `TRUSTEE_CONFIG`, `VAULT_SERVER_URL`, `VAULT_TOKEN`, `AWS_SM_KEY_PREFIX`, `B3_URL`; AWS Secrets Manager or Vault) | Container runtime, secrets backend | operator | `TRUSTEE_NAME`, and for AWS `AWS_SM_KEY_PREFIX`, are required before the secrets backend is used; backend credentials |
| `src/protocol/board/grpc_m.rs` `GrpcB3Index::get_boards` (board index) | b3 server and the network path to it | untrusted | `is_board_name_valid` before the name becomes a store path; `IGNORE_BOARDS` |
| `src/protocol/session/session.rs` `Session::step` to `src/protocol/trustee2.rs` `Trustee::step`, `update_bootstrap`, `update` (board messages) | windmill as protocol manager, other trustees, any b3 writer | untrusted | Schema version; canonical decoding of group elements; `Message::verify` against the board configuration; configuration hash; `BoardOverwriteAttempt`; artifacts fetched by statement hash and signer position (`LocalBoard::get_artifact`) |
| `src/protocol/board/grpc_m.rs` `GrpcB3::insert_messages` (outbound) | This trustee | trustee | Every message is signed with the trustee key; `Trustee::step` checks the outgoing configuration hash |
| `src/protocol/board/local2.rs` `LocalBoard::get_store` (local SQLite) | Local volume | operator | Local `AUTOINCREMENT` order |
| `src/bin/verify.rs` (`--server-url`, `--board`) to `src/verify/verifier.rs` `Verifier::run` | Auditor, against any b3 server | untrusted (board content) | `Configuration::is_valid`; `Message::verify` on every message; verifier-mode actions |
| `src/bin/gen_trustee_config.rs` | Operator, `scripts/trustee.sh` | operator | Prints a new trustee or protocol-manager key set to stdout |
| `src/bin/main_m.rs` `run` (multiplexed daemon) | Operator | operator | Same as `main.rs`; not the image command |
| `src/bin/demo_tool.rs`, `dbg.rs`, `demo_election_config.rs` | Developer | operator | Not copied into `Dockerfile.prod` |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| braid-T1 | Spoofing | A statement forged by a party outside the configuration, or under a trustee's identity, is acted on | `src/protocol/trustee2.rs` `Trustee::update` calls `../b3/src/messages/message.rs` `Message::verify` (sender listed, Ed25519 signature over the statement, configuration hash) and rejects `MessageConfigurationMismatch` | Mitigated |
| braid-T2 | Elevation of privilege | A configured participant posts a statement reserved for another role | Each statement is signed and tied to its signer's position in the configuration (braid-T1); mixing starts only from protocol-manager ballot artifacts | Partial |
| braid-T3 | Tampering | The board or network path reorders, replaces or equivocates messages, or swaps an artifact under a signed statement | `src/protocol/board/local2.rs` `get_store` (local order), `add_bootstrap` / `add_message` (`BoardOverwriteAttempt`), `get_artifact` (hash from the signed statement); all-trustee agreement on hashes through `ChannelsAllSignedAll`, `PublicKeySignedAll` (`src/protocol/datalog/dkg.rs`) and `MixNumberSignedUpTo` (`shuffle.rs`), and per-trustee `PlaintextsSigned` (`decrypt.rs`) | Mitigated |
| braid-T4 | Tampering | Fewer than `threshold` trustees learn or control the election secret key through the DKG | Schnorr proofs on channel keys; Feldman check of each received share (`strand::threshold::verify_share`) | Partial |
| braid-T5 | Tampering | The DKG numbers channel and share hashes after dropping NULL entries, so artifacts can be attributed to the wrong trustee | `src/protocol/action/dkg.rs` `sign_channels`, `compute_pk_` | Open on release/10.0 (fix in sequentech/step#3522) |
| braid-T6 | Denial of service | One trustee withholds its DKG contribution or signature and stalls key generation | Any failed check stops the DKG with `VerificationError`; `ConfigurationSignedAll`, `ChannelsAllSignedAll` and `PublicKeySignedAll` need all n trustees | Accepted (by design: the DKG needs all n trustees; a stall happens before any ballot is cast and shows in the key ceremony) |
| braid-T7 | Tampering | A mixer replaces, drops, duplicates or links ballots | Each trustee checks every mix's shuffle proof (strand) before signing it; `MixChain`, `MixNumberSignedUpTo` and `MixRepeat` in `src/protocol/datalog/shuffle.rs` | Partial |
| braid-T8 | Tampering | Wrong plaintexts are published for a batch | Chaum-Pedersen proofs on decryption factors, checked against the decryptor's verification key; every trustee recomputes the plaintexts and signs its own result | Partial |
| braid-T9 | Tampering | Statements or proofs are replayed across boards, batches or protocol steps | Configuration hash in every statement, checked by `Message::verify` and `Trustee::update` | Partial |
| braid-T10 | Spoofing | A trustee takes part in a protocol run that the platform did not authorize | `IGNORE_BOARDS`; access to b3 (deployment) | Partial |
| braid-T11 | Information disclosure | Trustee long-term keys are exposed during provisioning, at rest or in the image | Key set kept in AWS Secrets Manager or Vault (deployment) | Partial |
| braid-T12 | Information disclosure | Share material on the board is read by others | Shares encrypted to each recipient's channel key (`dkg.rs` `compute_shares`); channel secret key encrypted with the trustee symmetric key under an AEAD | Accepted (by design: trustee state and the key-ceremony download are rebuilt from the board, so a leaked symmetric key exposes that trustee's share on every retained board; see braid-T11) |
| braid-T13 | Information disclosure | Whoever holds the protocol-manager key chooses which ciphertexts are batched and decrypted | Mixing starts only from protocol-manager ballot artifacts; each batch is shuffled by all `threshold` selected trustees before decryption | Accepted (by design: braid cannot judge ballot validity or batch composition; the protocol manager is trusted for that, see Assumptions) |
| braid-T14 | Denial of service | Hostile board content, or a hostile b3 server or network path, stalls a board or stops the process that serves every board | Canonical point and scalar decoding (`../strand/src/backend/ristretto.rs`); schema version check; board-name filter; the `main.rs` loop logs a failed board step and moves on to the other boards (unless `--strict`) | Partial |
| braid-T15 | Repudiation | A trustee denies a statement it posted | Every statement is signed (`b3::messages::message::Signer` for `Trustee`), kept with its signature on b3 and rechecked by `Verifier::run` | Mitigated |
| braid-T16 | Tampering | The universal verifier reports a board as valid when it is not | `src/verify/verifier.rs` `Verifier::run` checks every signature and configuration hash | Partial |
| braid-T17 | Denial of service | The verifier underflows when a batch has fewer mixes than the threshold | `src/verify/verifier.rs` `Verified::add_results` | Open on release/10.0 (fix in sequentech/step#3522) |
| braid-T18 | Tampering | A consumer of the board uses an election public key or results the trustees did not produce | braid derives `PublicKeySignedAll` (`src/protocol/datalog/dkg.rs`) and each trustee posts its own `PlaintextsSigned` | Partial |
| braid-T19 | Information disclosure | Trustees below threshold influence the election key during the DKG | Feldman check of every received share | Partial |
| braid-T20 | Information disclosure | Trustee secrets stay readable in process memory, swap or core dumps | `Trustee`'s `Debug` prints only the name; `TrusteeConfig` has no `Debug`; secret types from strand (see strand-T13) | Partial |
| braid-T21 | Tampering | A board name from the b3 index escapes the local store directory | `GrpcB3Index::is_board_name_valid` calls `../b3/src/grpc/mod.rs` `validate_board_name` (ASCII letters, digits and `_` only) before `message_store/<board>` is opened | Mitigated |
| braid-T22 | Tampering | The trustee image is tampered with at build time | `Dockerfile.prod` builds with `cargo build --release --locked` | Partial |
| braid-T23 | Denial of service | A malformed `Shares` artifact, or a configuration signed by a listed key other than the protocol manager's, panics the trustee process instead of failing one board, so every board that trustee serves stops | None on this branch (`src/protocol/action/dkg.rs`, `src/protocol/action/decrypt.rs`, `src/protocol/trustee2.rs`) | Open on release/10.0 (fix in sequentech/step#3522) |

## Assumptions

- Fewer than `threshold` trustees collude, and each selected set contains at least one honest mixer. All n trustees stay live for the DKG, and the `threshold` selected trustees stay live for each tally.
- Each trustee runs on its own machine under separate control, and the container runtime restarts the process when it exits.
- windmill is the protocol manager. It keeps the protocol-manager key in its vault, batches only ballots it has validated, in batches large enough to keep votes anonymous, and checks braid's outputs before using them.
- b3 is untrusted for integrity but needed for availability. The deployment protects b3 and the network path to it.
- strand provides sound proofs, canonical decoding, a CSPRNG (`StrandRng`) and constant-time secret arithmetic. braid draws no randomness of its own outside tests.
- Operators protect the trustee host, its secrets backend, key file and volumes.
- Election managers pick the trustee count and threshold for each key ceremony, within the limits of `Configuration::is_valid`.
- Auditors run `verify` against an authentic copy of the board.

## Review focus

- The DKG, on which every ballot's secrecy rests.
- Validation of board artifacts.
- Board ingestion and liveness.
- Role authorization.
- Configuration acceptance.
- The universal verifier, which auditors rely on as independent evidence.
- Provisioning and storage of trustee long-term keys. One key set covers every board.
- Domain separation in proofs and encryption.
- Resource use and memory hygiene of secret material.
- Only `src/protocol/board/grpc_m.rs` and `local2.rs` are compiled. `grpc.rs`, `local.rs` and `pgsql.rs` are dead code and out of scope unless they are re-enabled.
