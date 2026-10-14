---
id: orare
title: Orare and document renderer tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

From `packages/`, use the pinned Rust toolchain and the Chromium binary installed
by `.github/actions/setup-test-browser` (`CHROME` must point to that binary):

```sh
export DOC_RENDERER_BACKEND=inplace
export RUST_TEST_THREADS=2
cargo build --locked -p orare --features openwhisk --example contract-openwhisk
export ORARE_TEST_OPENWHISK_BINARY="$PWD/target/debug/examples/contract-openwhisk"
cargo test --locked -p orare --features openwhisk --test runtime_contracts
cargo test --locked -p doc_renderer
cargo test --locked -p orare --no-default-features --features aws_lambda --test runtime_contracts
cargo test --locked -p doc_renderer --no-default-features --features aws_lambda
```

The OpenWhisk macro test launches a compiled consumer with literal input/output
and malformed-argument controls. AWS tests call the generated adapter with
synthetic Lambda events, checking the HTTP response and error propagation for
missing bodies, malformed JSON and handler rejection. They make no AWS requests.

Renderer tests deserialize both wire variants, require their mandatory fields,
render synthetic HTML through real Chromium and inspect PDF markers. The local
adapter rejects S3 input; the AWS adapter rejects raw input before contacting
storage. Chromium's `--single-process` and `--no-zygote` arguments remain required
by the existing renderer environment.

The runtimes are separate feature configurations; do not enable both at once.
Remote S3 transfer and deployed Lambda/OpenWhisk HTTP routing require separate
integration fixtures. No native line percentage is inferred from these compiled
consumer and renderer checks; a coverage profile must account separately for
macro implementation, expanded consumer code and each runtime.
