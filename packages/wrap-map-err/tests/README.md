<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Error conversion macro tests

`behavior.rs` compiles functions through the public attribute macro and calls
them. These controls check actual generated Rust: default-error Result aliases,
qualified Results, borrowed generic values, early returns, question marks,
unchanged non-Result functions, and asynchronous suspension/resumption. A leaf
error that can convert only to the original error type guards against silently
changing the meaning of `?`.

`support/expansion.rs` checks syntax that cannot be a compiled consumer: invalid
attributes, non-function items, unsupported Result arguments and const-function
diagnostics. It also verifies that function metadata survives expansion.

From `packages`, run `cargo test -p wrap-map-err --locked --offline`. For a full
measurement with the pinned tools, run from the repository root:

```sh
python3 scripts/coverage/run.py wrap-map-err --offline
```

The original package had no tests. Do not interpret the lack of a valid runtime
coverage report as a measured zero. Runtime coverage measures expansion helpers;
the native proc-macro entry point executes in the compiler, outside those line
counters. Public integration tests cover it through compilation. No source files
or positive counters are excluded. Branch coverage and full Windmill/Celery task
integration remain separate evidence requirements in Meta #13299.
