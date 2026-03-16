---
applyTo: "packages/sequent-core/**/*.rs,packages/braid/**/*.rs,packages/strand/**/*.rs,packages/immu-board/**/*.rs,packages/windmill/src/services/insert_cast_vote.rs,packages/windmill/src/services/reports/**/*.rs"
---
<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Crypto And Audit Review

- Review changes here as cryptographic and audit critical, not as routine refactors.
- Flag any weakening of proof, signature, hash, encryption, randomness, key-handling, or canonical serialization checks.
- Do not allow validation bypasses for malformed ballots, invalid proofs, mismatched hashes, bad signatures, or parse failures that should stop processing.
- Verify native and WASM code paths preserve the same ballot semantics and verification behavior.
- Receipt, bulletin-board, export, and audit outputs must remain reproducible and traceable to the same ballot content and hashes.
- For changes to audit logging or bulletin-board storage, check immutability assumptions, ordering assumptions, and whether retries can duplicate or hide evidence.
- Backwards compatibility matters: older election events, stored payloads, and serialized objects must still verify or fail in a controlled, explicit way.
- Expect negative tests for invalid proofs, signature failures, malformed ciphertexts, mismatched hashes, replay or retry scenarios, and serialization round trips.
