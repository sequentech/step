---
id: strand
title: Strand boundary tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Test file paths in this guide are relative to [`packages/strand/tests/`](https://github.com/sequentech/step/blob/feat/meta-13302-ui-essentials-coverage/main/packages/strand/tests). Commands state their working directory.

These tests complement the existing randomized protocol tests with small,
independently checkable examples and malformed inputs that must be rejected.
They use generated test keys and local temporary files. No external service,
production key or network connection is needed.

| File | What it protects |
| --- | --- |
| `wire_boundaries.rs` | Public-key and signature transport, the RFC 8032 verification vector, SHA-2 known answers, file reads across a buffer boundary, authenticated-encryption tampering, and collection shapes. |
| `stream_contracts.rs` | Literal container/scalar/hash wire layouts, fallible elements and sinks at every byte boundary, truncated inputs, and restored keys/proofs that must still decrypt or verify. |
| `protocol_boundaries.rs` | ElGamal arithmetic, proof binding to statements and context, decryption factors, a hand-calculated threshold polynomial, and tampering with each column of a product shuffle. |
| `support/proof_shapes.rs` | Deserialized shuffle proofs with missing or extra dimensions must return errors before indexed verification. Each test first verifies the original valid proof. |

The shape tests are compiled as an internal test module because proof fields
are crate-private. Keeping them outside `src` also keeps the new test bodies
out of the coverage reporter's source inventory. They are deliberately small:
three ciphertexts suffice to exercise the same dimension relationships as a
large shuffle.

From the repository root, run the complete native coverage check with:

```sh
python3 scripts/coverage/run.py strand --offline
```

Install the pinned tools and fetch locked dependencies first, as described in
the package coverage guide. For a fast edit/check cycle, run the three integration
test binaries or the `proof_shape_tests` library filter with Cargo.

The native default profile exceeds the 95% line improvement target. CI compares
lines, functions and LLVM regions separately against the actual PR base and
rejects any decrease; 95% is not the CI gate. The report also lists
function and region coverage and every unmeasured source file. Existing inline
tests are still included in LLVM's native aggregate, so this is not a claim of
production-only coverage, branch coverage, WASM coverage, or coverage of optional
backends. The remaining work is tracked in [Meta #13302](https://github.com/sequentech/meta/issues/13302).

## Measured checkpoint and remaining gaps

Source `6b04edb4144362bde2761beb053aa9fe79890219`, Rust 1.96.0 and
cargo-llvm-cov 0.9.1, isolated native default profile: **64 tests pass, none
ignored**; **3,470/3,555 lines (97.61%)**, **350/373 functions (93.83%)**,
**6,629/6,822 LLVM regions (97.17%)**. The preceding 48-test suite measured
95.61%, 84.72% and 95.79% respectively under the same profile. Production Clippy
and workspace formatting pass with existing warnings. Four unsigned zero guards
use `== 0` to satisfy Clippy without changing their behavior.

The 85 missing lines and 23 functions remain counted. In this concrete backend:

- `signatures/dalek.rs`: fixed-size signature decoding accepts all 64-byte
  arrays; JSON array conversion errors follow an already checked length. DER
  encoding errors require a backend/allocator failure. Revisit if these APIs or
  their upstream implementations become fallible for ordinary inputs.
- `zkp.rs`: `ChallengeInput` hashes its map directly, leaving its derived Borsh
  serializer unused; stored byte-vector encoding and normal proof-element
  encoding cannot fail with the in-memory sink. Revisit with a fallible backend
  or a new transport caller.
- `symmetric/rustcrypto.rs`: encryption failure requires an oversized message
  beyond the algorithm's supported length, impractical to allocate in this suite.
- `random/rand.rs`, `keymaker.rs`: unused `StrandStdRng` adapters, `next_u64` and
  the private `from_sk` constructor are not invoked solely to increase counters.
- `backend/ristretto.rs`: unused modular/inverse convenience methods and point
  debugging; `signatures/dalek.rs` signature debugging; `shuffler.rs` an internal
  generated-shape error closure. These remain visible debt rather than excluded
  or justified as universal impossibilities. New callers or changed proof shape
  construction should trigger another review.

Generated cryptographic serialization remains included: injected write failures
and malformed records exercise real wire contracts, with successful decryption
and proof verification controls. Tests do not call formatting or cloning merely
for a score. Optional backends, Rayon, WASM, and actual branches remain separate
measurement obligations.

The stack also makes Core's existing inline round-trip fixture always exercise
both its marked-empty ballot and mixed valid/explicit-invalid contests. The
previous random choice of those cases changed two line and nine region counters
between identical revisions, causing a real CI ratchet failure. Candidate sizes
and choices still vary; protocol implementations and the strict comparison rule
are unchanged. The shared hosted setup supplies the compatible pinned browser
for Velvet consumer tests on this earlier stacked PR as well.
