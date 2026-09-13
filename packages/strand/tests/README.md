<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Strand boundary tests

These tests complement the existing randomized protocol tests with small,
independently checkable examples and malformed inputs that must be rejected.
They use generated test keys and local temporary files. No external service,
production key or network connection is needed.

| File | What it protects |
| --- | --- |
| `wire_boundaries.rs` | Public-key and signature transport, the RFC 8032 verification vector, SHA-2 known answers, file reads across a buffer boundary, authenticated-encryption tampering, and collection shapes. |
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
the package coverage guide. For a fast edit/check cycle, run the two integration
test binaries or the `proof_shape_tests` library filter with Cargo.

The native default profile passes the 95% line threshold. The report also lists
function and region coverage and every unmeasured source file. Existing inline
tests are still included in LLVM's native aggregate, so this is not a claim of
production-only coverage, branch coverage, WASM coverage, or coverage of optional
backends. The remaining work is tracked in Meta #13293.
