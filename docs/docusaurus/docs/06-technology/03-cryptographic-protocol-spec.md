---
id: cryptographic-protocol-spec
title: Reference Overview
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The election cryptographic backend is the supplied `packages/wbraid` workspace.
Its `braid` crate runs distributed key generation, verifiable mixing and threshold
decryption; its `vsc` crate exposes the `cryptography` library used for the
cryptographic operations. Its `b4` crate supplies the native `b4v6` board service.

## Platform integration

| Component | Responsibility |
| --- | --- |
| `voting-crypto` and `sequent-core` | Encode ballots, encrypt them with Naor–Yung proofs, reconstruct audit openings, and check ballot trackers, signatures and receipts. The voting portal uses the same Rust implementation through WASM. |
| `protocol-board` | Bind canonical signed DKG and tally messages to their configuration, adapt native and browser trustee transports and persistence, and verify public transcripts. |
| Native `trustee` and the administration portal's browser trustee | Retain private identities and election shares locally; perform DKG, verify proofs, mix and publish partial decryptions. |
| Harvest and Windmill | Authorize voting and ceremonies, validate ciphertexts against the trusted published ballot style, construct tally batches, and coordinate results. |
| Velvet | Decode the decrypted plaintexts and apply the election's counting rules, channels and weights. |
| `electoral-log` | Store application electoral logs as opaque native board batches in a separate private PostgreSQL database; retain application signatures and ballot-box seals. |
| Ballot verifier | Check a revealed audit opening, tracker and ballot signature against the published ballot style. |
| `trustee verify` and browser `WasmVerifier` | Independently check public DKG and tally transcripts, including ballot proofs, ordered shuffle proofs, threshold decryption and ciphertext inclusion. |

## Trustee custody

Trustees register separate signing and share encryption public keys. Manual
ceremonies exchange public custody requests and signed public custody proofs.
Automatic ceremonies skip manual custody confirmation while retaining the same
cryptographic protocol. Both require the selected trustees' runners to participate.
The platform receives neither trustee identity files nor election shares, and
threshold decryption does not reconstruct the private key.

See [native and browser trustee configuration](../07-developers/09-braid/braid_trustees_configuration.md)
for setup, backups and recovery.

## Verification evidence

Single-contest and combined-contest audit decoding verifies the Naor–Yung proof
against the ballot style's public key and configuration, then checks that the
revealed plaintext and randomness reconstruct the encrypted values. A valid
tracker or voter signature alone does not validate an audit opening. Invalid
openings fail before any selections are returned.

Public transcript verification requires an independently trusted configuration
hash and tally identifier. A board's own self-signed configuration is insufficient
as its trust anchor. Incomplete evidence does not pass verification.

Cryptographic transcript verification checks the published tally input. Checking
how that input was selected from the sealed ballot box, including revotes and
weights, also requires ballot-box evidence and the platform's batching and counting
checks. See [public verification](../07-developers/09-braid/public-verification.md)
and [ballot-box seal verification](../04-election_auditors/02-tutorials/06-auditor_verify-ballot-box-seal.md).

The detailed supplied protocol and canonical serialization specification live in
`packages/wbraid/PROTOCOL.md` and `packages/wbraid/SERIALIZATION.md`. General TLS,
voter-field encryption, certificate signatures and document protection remain
separate application facilities.
