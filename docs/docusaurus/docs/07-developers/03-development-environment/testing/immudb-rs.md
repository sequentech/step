---
id: immudb-rs
title: ImmuDB client tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

From `packages/`, with the repository's pinned Rust toolchain:

```sh
cargo test --locked -p immudb-rs --tests
```

The RPC tests own a loopback HTTP/2 listener and return scripted gRPC responses.
They use synthetic credentials, retain the allocated port, impose a ten-second
limit per test and abort the server task on cleanup. No existing database or
network service is used.

Assertions inspect requests received at the transport boundary: authentication,
session and transaction metadata; SQL text and parameters; query modes; database
selection and deletion. Rejection tests pair injected failures with successful
controls. In particular, a successful unload must not hide a subsequent delete
failure, and failed logout must retain authentication for a retry.

The coverage profile also runs the existing electoral-log consumer suite against
an owned ImmuDB process. Install ImmuDB 1.9.6 or point the fixture at its binary:

```sh
ELECTORAL_LOG_TEST_IMMUDB_BINARY=/path/to/immudb \
  python3 scripts/coverage/run.py immudb-rs --baseline
```

Run that command from the repository root. The profile executes each revision's
own client and consumer tests, while reporting only `immudb-rs` source counters.
CI compares lines, functions and LLVM regions independently against the actual
PR base. Protocol definitions generated into Cargo's `OUT_DIR` are exercised by
the requests but are not handwritten package-source metrics. Actual Rust branch
coverage, TLS negotiation and multi-frame stream interruption need separate
measurements.
