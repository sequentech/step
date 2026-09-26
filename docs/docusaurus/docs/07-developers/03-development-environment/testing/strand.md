---
id: strand
title: Strand boundary tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Test file paths in this guide are relative to [`packages/strand/tests/`](https://github.com/sequentech/step/blob/release/10.0/packages/strand/tests). Commands state their working directory.

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
are crate-private. Keeping them outside `src` also keeps the test bodies
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
  generated-shape error closure. These remain visible paths rather than excluded
  or justified as universal impossibilities. New callers or changed proof shape
  construction should trigger another review.

Generated cryptographic serialization remains included: injected write failures
and malformed records exercise real wire contracts, with successful decryption
and proof verification controls. Tests do not call formatting or cloning merely
for a score. Enabled native backends, WASM and actual branches have separate accounting.

## Native backends

The default profile uses Ristretto. To exercise the integer backends and Rayon,
install a C/C++ toolchain and M4 for GMP, then run from `packages/`:

```bash
cargo test --locked -p strand --features num_bigint,malachite,rug,rayon
```

Rug uses GMP with an adapter borrowing Strand's operating-system RNG.
Malachite enables its random APIs explicitly. Backend tests must check literal
wire layouts and rejected inputs as well as successful protocol operations.
Do not treat an unselected feature as evidence about its implementation.

From the repository root, collect the corresponding profiles:

```bash
python3 scripts/coverage/run.py strand-native --baseline --offline
python3 scripts/coverage/run.py strand-backends --baseline --offline
```

The profiles keep their own source inventories and counters. CI compares each
buildable base/head pair independently. A restored configuration with an
unbuildable base establishes its first measurement; it cannot claim an increase
from zero. The default and NumBigInt/Rayon comparisons still run in that case.

`integer_backends.rs` pins each backend's byte order and plaintext representation,
checks small arithmetic, truncated records and broken writers, and verifies
proofs before changing their election label or public key. Fixed group contexts
must preserve all parameters and reject a changed generator, modulus, exponent
modulus or cofactor. A modulus is a group parameter, not a valid group element.

The ignored `backend/rug.rs::test_gen_coq_data` test is a manual transcript
exporter for an external Coq verifier. It compiles with Rug; its inline exporter,
serialization and printing code remain counted. Running it solely to exercise
that output does not verify Coq interoperability. Pair its output with the actual
verifier when testing that contract, and keep the result separate from native
protocol tests. Browser benchmark/demo harnesses likewise need their intended
browser or performance checks, not tests that only invoke diagnostic printing.

The OpenSSL backends use AES-GCM; `openssl_full` also uses P-384 signatures.
With the OpenSSL development headers installed, check their transport and
rejection contracts from `packages/`:

```bash
cargo test --locked -p strand --features openssl_core --test wire_boundaries --test stream_contracts
cargo test --locked -p strand --features openssl_full --test wire_boundaries --test stream_contracts
```

These checks cover authenticated context, modified ciphertexts, DER records,
truncation and failing writers. They are separate test runs, not OpenSSL coverage
measurements or validation of an installed FIPS provider.
