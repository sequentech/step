---
name: build-and-test
description: Build, test, lint and format Rust, TypeScript or Java packages; rebuild sequent-core WASM; regenerate GraphQL types; change the Hasura schema; check REUSE headers. Use before running a compiler, test suite or formatter.
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Build and test

[Build, test and lint](../../../docs/docusaurus/docs/07-developers/03-development-environment/build-and-test.md)
holds the commands, toolchain pins and service URLs. For the fastest check of an
edit, use [fast-feedback](../fast-feedback/SKILL.md) first.

1. Check what already runs: `scripts/dev/step-dev mode status`. In backend
   modes Harvest, Windmill and beat rebuild on change; read their logs before
   starting a compiler. Reuse a dev server already listening on its port.
2. Rust: run Cargo in the devenv shell with this checkout's target directory,
   never the root-owned `packages/target`:
   `devenv shell -- bash -c 'cd packages && CARGO_TARGET_DIR="$PWD/rust-local-target" cargo test -p <crate>'`.
   Braid tests need `--release`. Check the crate's `[features]`.
3. TypeScript: from `packages/`, `yarn lint`, `yarn prettify` and the package's
   tests. Do not build `ui-core`/`ui-essentials` for ordinary source edits.
4. Java: `mvn clean verify` in `packages/keycloak-extensions/`.
5. Rust used by the portals: `scripts/dev/step-dev wasm`, then
   `step-dev wasm --status`; a failed rebuild is not a preview of new source.
6. GraphQL query or schema change: `yarn generate:<portal>` from `packages/`.
   Hasura schema changes go through `hasura console` so migrations are recorded.
7. Before committing: `cargo fmt -- --check`, `cargo clippy -p <crate>`, the
   prettier/lint checks for touched frontends, and `reuse lint`.

Report the commands, scope and results; a skipped check is not a pass.
