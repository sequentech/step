---
id: wbraid-protocol
title: Native wbraid protocol tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

`packages/protocol-board` integrates the supplied `packages/wbraid` protocol.
Run its contracts and the native trustee tests from the application workspace:

```sh
cd packages
CARGO_TARGET_DIR="$PWD/rust-local-target" cargo test --release --locked -p protocol-board -p trustee
```

The suites cover DKG and tally lineage, trustee selection, signature rejection,
canonical ballot proof verification, restart behavior, custody proofs, empty
ballot batches and ceremony transitions. Native cryptographic runs require an
optimized build. Custody tests reject another trustee's key and a proof replayed
into another tally session.

Archive admission verifies Configuration messages through wbraid's construction
path and verifies other messages through its predicate verifier. It also checks
each signed configuration hash against the archived parent domain. A valid
signature from a trustee shared by two ceremonies cannot move a message from
one ceremony's archive into the other.

The supplied protocol's additional suites and browser emulator are documented
in `packages/wbraid/TESTING.md`. Its crates use a separate Cargo workspace:

```sh
CARGO_TARGET_DIR="$PWD/rust-local-target" cargo test --manifest-path wbraid/Cargo.toml --release --locked -p braid
```

Wire version 2 binds an empty mix to its input hash. A client rejects messages
with another wire version before decoding; all participating trustees must use
the same version.
