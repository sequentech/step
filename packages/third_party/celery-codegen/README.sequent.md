# Sequent patch of celery-codegen 0.5.5

This directory contains `celery-codegen` from the exact rusty-celery revision
already pinned by this workspace, `b4145925aaaf742b7ba54499fff45313bcadfca1`.
`UPSTREAM.json` records the source and SHA-256 hashes before the local patch;
`LICENSE` is the upstream Apache-2.0 license. The upstream version and dependency
requirements are unchanged.

The sole source change in `src/task.rs` replaces the named enclosing constant
with an anonymous `const _: ()`. Rust treats implementations inside that
anonymous constant as belonging to the surrounding module for the
`non_local_definitions` lint. The generated task implementation, trait methods,
serialization, options, and task names are unchanged. The workspace patches
only this code-generation package; the Celery runtime remains at the pinned
Git revision.

`windmill/tests/celery_codegen_contract.rs` denies the offending lint and checks
both synchronous and asynchronous generated tasks, metadata, serialized
message arguments, and successful/error returns without contacting a broker.
Run it with `cargo test -p windmill --test celery_codegen_contract` from
`packages`. The retained development verification also runs the identical test
in a minimal temporary consumer against the original and patched dependency.

This is third-party source, excluded from first-party workspace members just as
the original Git dependency was. Changes to this patch are controlled dependency
changes and require a new intake request and review before Site import.
Remove this override only after the chosen upstream revision includes an
equivalent fix and the contract tests pass.
