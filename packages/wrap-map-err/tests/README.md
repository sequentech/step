<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Error conversion macro tests

`behavior.rs` compiles functions through the public attribute macro and calls
them. These controls check actual generated Rust: default-error Result aliases,
qualified Results and renamed `TaskResult`/`WrapResult` imports, borrowed generic values, early returns, question marks,
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

The original package had no tests. Do not interpret the lack of a valid coverage
report as a measured zero. LLVM records both the expansion helpers exercised
by runtime unit tests and the public entry point exercised while compiling
consumer fixtures. The package percentage combines those two phases.

No source files or positive counters are excluded. The downstream Harvest build
also compiles Windmill's real Celery consumers. This caught renamed `TaskResult`
and `WrapResult` aliases, which now have dedicated compiled regressions here.
The macro recognizes the conventional `Result` suffix; it cannot resolve
arbitrarily named Rust aliases. Branch coverage and successful distributed task
execution remain separate evidence requirements in Meta #13299.
