---
id: voting-crypto
title: Voting crypto boundary tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

The application crypto boundary is `packages/voting-crypto`, backed by wbraid's
provided VSC and NY ballot implementation. The retired backend is absent from
the application workspace.

```sh
cd packages
CARGO_TARGET_DIR="$PWD/rust-local-target" cargo test --locked -p voting-crypto
CARGO_TARGET_DIR="$PWD/rust-local-target" cargo test --locked -p sequent-core
```

Tests check canonical ciphertext encoding, trusted ballot contexts, encryption
randomness reconstruction, rejected proofs, RFC 8032 signing vectors and raw-key
serialization. Stored secret decoding accepts the previous application envelope
and the native envelope without rewriting existing records.

After changes to browser crypto, rebuild both committed and development WASM:

```sh
scripts/dev/step-dev wasm --release-package
scripts/dev/step-dev wasm
scripts/dev/step-dev wasm --status
scripts/dev/step-dev wasm --check-package
```
