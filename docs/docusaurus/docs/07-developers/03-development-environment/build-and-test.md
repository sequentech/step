---
id: build-and-test
title: Build, test and lint
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

`packages/` is both a Cargo workspace and a Yarn workspace: Rust code such as
`sequent-core` and `braid` also compiles to WebAssembly for the portals. The
[software architecture reference](../../05-reference/03-software-architecture/intro.md)
describes the components. For the edit, preview and focused-test loop use
[Fast feedback loops](./fast-feedback.md); this page lists the underlying
per-stack commands.

## Packages

| Kind | Packages |
| --- | --- |
| Rust libraries | `strand` (cryptographic primitives), `braid` (verifiable mixnet), `sequent-core` (ballots, crypto, reports, Keycloak client; also WASM) |
| Rust services and tools | `windmill` (Celery task workers), `harvest` (REST API), `immu-board` (bulletin board), `velvet` (tally and report pipes), `step-cli`, `e2e` |
| Frontends | `voting-portal`, `admin-portal`, `ballot-verifier`, `results-portal`, `workbench` |
| Shared frontend | `ui-essentials` (components, Storybook), `ui-core` (utilities, i18n) |
| Java | `keycloak-extensions` (Keycloak providers and themes) |

Hasura exposes PostgreSQL over GraphQL; Keycloak has one realm per tenant and
one per election event; RabbitMQ feeds the Celery workers; ImmuDB stores
tamper-evident logs; MinIO provides S3 storage.

## Toolchain

The devenv shell (`devenv shell` from the repository root, or the devcontainer)
provides the pinned tools: Rust from `rust-toolchain.toml` with the
`wasm32-unknown-unknown` target, Node.js 20 with Yarn, and JDK 17 for the
Keycloak extensions. `wasm-bindgen` is pinned to `=0.2.128` in `strand`,
`braid`, `sequent-core` and `wbraid`; the CLI pins in `devenv.nix` and the
`flake.nix` files move with it. `celery` comes from a fork pinned in
`packages/Cargo.toml`. Check a crate's `[features]` before building it: most
crates compile different code per feature set.

## Rust

Each checkout builds into its own `packages/rust-local-target`. The shared
`packages/target` belongs to the service containers, which build into it as
root.

```sh
devenv shell                           # from the repository root
cd packages
export CARGO_TARGET_DIR="$PWD/rust-local-target"
cargo build -p <crate>
cargo test -p <crate> [<test-name>]
cargo test --release -p braid          # braid tests need an optimized build
cargo fmt -- --check
cargo clippy -p <crate>
```

A single command can run the same steps non-interactively:
`devenv shell -- bash -c 'cd packages && CARGO_TARGET_DIR="$PWD/rust-local-target" cargo test -p <crate>'`.

In `backend` and `full` modes Harvest, Windmill and beat already rebuild on
change: run `scripts/dev/step-dev mode status` and read the checkout's service
container logs before starting another compiler.
`scripts/dev/step-dev test <crate> <test-name>` runs one focused test in the checkout's target directory.

## TypeScript

Run from `packages/` after `yarn install --frozen-lockfile`:

```sh
yarn start:voting-portal       # port 3000
yarn start:admin-portal        # port 3002
yarn storybook:ui-essentials   # port 6006
yarn build:<package>           # production build, e.g. build:admin-portal
yarn test:ui-essentials
yarn lint && yarn prettify     # lint:fix and prettify:fix repair
```

Check whether a dev server already listens on its port before starting one.
Portals and Storybook compile `ui-core` and `ui-essentials` from source; build
those packages only for production builds and journeys.

### sequent-core WASM

`scripts/dev/step-dev wasm` rebuilds the development artifact incrementally
without reinstalling dependencies, and `scripts/dev/step-dev wasm --status` reports whether
the committed package matches its sources. See
[Incremental WASM](./fast-feedback.md#incremental-wasm).

### GraphQL

Portal queries live in each portal's `src/queries/`. After a query or schema
change regenerate the types with `yarn generate:voting-portal`,
`yarn generate:admin-portal`, `yarn generate:ballot-verifier` or
`yarn generate:results-portal`. The VS Code task `update.graphql` refreshes
`packages/admin-portal/graphql.schema.json`; Windmill's queries under
`packages/windmill/src/graphql/` may need matching updates.

### Hasura migrations

Change the Hasura schema and metadata through `hasura console`, never the web
UI, so the console records migrations under `hasura/`:

```sh
cd hasura
hasura console --endpoint "http://graphql-engine:8080" --admin-secret "admin"
```

## Java

From `packages/keycloak-extensions/`: `mvn clean package` builds every
extension and `mvn clean verify` also runs the tests. The
[Keycloak guide](../06-keycloak/developers_keycloak.md) covers themes and live
reload.

## Licensing

Every file carries SPDX headers (REUSE). Copy the header style of neighbouring
files, or add a `.license` sidecar or `REUSE.toml` entry for files that cannot
hold comments, and run `reuse lint` from the repository root.

## Development services

| Service | URL | Credentials |
| --- | --- | --- |
| Keycloak | http://127.0.0.1:8090 | `admin` / `admin` |
| Hasura | http://127.0.0.1:8080 | admin secret `admin` |
| Voting portal | http://127.0.0.1:3000 | |
| Admin portal | http://127.0.0.1:3002 | |
| ImmuDB | http://127.0.0.1:3325 | `immudb` / `immudb` |
| MinIO console | http://127.0.0.1:9001 | |
| RabbitMQ | http://127.0.0.1:15672 | |

Which services run depends on the
[devcontainer mode](./fast-feedback.md#devcontainer-modes).
