# SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Sequent Voting Platform — an end-to-end verifiable, secure online voting system. The `packages/` directory is **both a Cargo workspace and a Yarn workspace**, sharing Rust code with the frontend via WebAssembly compilation.

## Build Commands

### Rust (from `packages/`)

> **Note:** `cargo` is not on PATH by default. Use `devenv shell` (from the repo root) to enter the nix environment, and use the package-specific `rust-local-target/` dir to avoid permission errors (the shared `packages/target/` is owned by root because Docker service containers build into it as root).

> **Do not run `cargo build` manually to check that code compiles.** The `windmill` and `harvest` containers (and `sequent-core`, which both depend on) already auto-rebuild on file changes inside the dev container. After editing code in one of these, check that container's logs instead (`docker logs windmill --tail 100` / `docker logs harvest --tail 100`) to confirm it compiled — don't kick off a separate `cargo build`, which just duplicates that work. `cargo test`, `cargo fmt`, and `cargo clippy` are unaffected by this and still run manually as usual.

```bash
# Run any cargo command inside devenv:
cd /workspaces/step && devenv shell bash -- -c 'cd packages && CARGO_TARGET_DIR=/workspaces/step/packages/<pkg>/rust-local-target cargo <command> -p <package>'

# Examples:
cargo build                          # Build all Rust packages
cargo build -p <package>             # Build one package (e.g. braid, strand, windmill, sequent-core)
cargo build --release                # Production build
```

### TypeScript (from `packages/`)
```bash
yarn                                 # Install all JS dependencies
yarn build:ui-core                   # Build ui-core library
yarn build:ui-essentials             # Build ui-essentials library (must rebuild after changes)
yarn build:voting-portal             # Build voting portal
yarn build:admin-portal              # Build admin portal
yarn start:voting-portal             # Dev server on port 3000
yarn start:admin-portal              # Dev server on port 3002
yarn storybook:ui-essentials         # Storybook on port 6006
```

### Java (from `packages/keycloak-extensions/`)
```bash
mvn clean package                    # Build all Keycloak extensions
mvn clean verify                     # Build and test
```

## Test Commands

### Rust (from `packages/`)
```bash
cargo test                           # All tests
cargo test -p <package>              # Single package tests
cargo test <test_name>               # Single test by name
cargo test --release -p braid        # Braid tests require --release
```

### TypeScript (from `packages/`)
```bash
yarn --cwd ./ui-essentials test      # Jest tests for ui-essentials
yarn generate:voting-portal          # Regenerate GraphQL types after schema changes
yarn generate:admin-portal           # Same for admin portal
```

## Lint & Format

### Rust (from `packages/`)
```bash
cargo fmt -- --check                 # Check formatting
cargo fmt                            # Fix formatting
cargo clippy                         # Lint
```

### TypeScript (from `packages/`)
```bash
yarn lint                            # ESLint all frontend packages
yarn lint:fix                        # Auto-fix
yarn prettify                        # Check Prettier
yarn prettify:fix                    # Fix Prettier
```

### License compliance
```bash
reuse lint                           # Every file must have SPDX headers
```

## Architecture

### Rust Packages (backend + WASM)
- **braid** — Verifiable re-encryption mixnet (cryptographic shuffle). Features: `native`, `wasm`, `jemalloc`
- **strand** — Cryptographic primitives (Curve25519/Ristretto, Ed25519, SHA2/SHA3). Features: `rayon`, `wasm`, `fips_core`
- **sequent-core** — Shared core: ballot structures, crypto ops, PDF reports, Keycloak integration. Compiles to WASM for frontend. Features: `wasm`, `reports`, `keycloak`, `s3`, `sqlite`
- **windmill** — Celery-based task execution engine (RabbitMQ, GraphQL client, WASM plugin mgmt)
- **harvest** — Election management REST API (Rocket framework)
- **immu-board** — Tamper-evident bulletin board
- **velvet** — PDF/report generation CLI
- **step-cli** — CLI for election administration
- **e2e** — End-to-end testing framework

### TypeScript Packages (frontend)
- **voting-portal** — Voter interface (React 19, Apollo Client, MUI 7, Keycloak auth, uses sequent-core WASM)
- **admin-portal** — Admin interface (React Admin, MUI data grid, TinyMCE, uses sequent-core + braid WASM)
- **ballot-verifier** — Standalone ballot verification tool
- **ui-essentials** — Shared UI component library (MUI, Storybook). Used by all portals
- **ui-core** — Lower-level shared utilities (HTML sanitization, i18n)

### Infrastructure
- **Hasura** — GraphQL API layer over PostgreSQL
- **Keycloak** — Identity management (one realm per tenant + one per election event)
- **RabbitMQ** — Task queue for Celery workers
- **ImmuDB** — Tamper-evident audit logging
- **MinIO** — S3-compatible object storage

### Key Patterns
- Rust crypto code compiles to WASM for browser use via `wasm-pack`
- Extensive Cargo feature flags for conditional compilation — always check `[features]` in Cargo.toml
- Celery + RabbitMQ for async task execution
- GraphQL codegen: queries live in `src/queries/`, types generated with `yarn generate:*`
- After editing ui-essentials components: `yarn prettify:fix:ui-essentials && yarn build:ui-essentials`
- **sequent-core WASM rebuild**: when changing `sequent-core`, the WASM package often needs rebuilding for frontend changes to take effect

## Important Conventions

- **REUSE license headers required on every file**:
  ```
  // SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
  // SPDX-License-Identifier: AGPL-3.0-only
  ```
- **Vendored Cargo dependencies**: `packages/vendor/` — Cargo.toml uses `[source.vendored-sources]`
- **Pinned crate**: `wasm-bindgen` 0.2.104 — do not change
- **Forked crate**: `celery` uses a custom fork (Findeton/rusty-celery)
- **Hasura changes must go through** `hasura console` (not the web UI directly) for migrations to be tracked
- **Rust toolchain**: 1.96.0 stable, WASM targets: `wasm32-unknown-unknown`
- **Node.js**: 20.x, package manager: Yarn (workspaces)
- **Java**: JDK 17 for Keycloak extensions

## Product Design Philosophy

**This is critical and applies to all code changes.**

- **Never build client-specific features** — features are designed to serve all clients, not just the one requesting them. When a client needs something, design a general solution that accommodates that client AND all existing ones
- **Features must be composable** — clients should be free to select any combination of features. If certain combinations are incompatible, handle it at the design level: clearly communicate constraints to the client (e.g., via validation, warnings, or documentation), never silently break
- **Policies use enums, not booleans** — when modeling policies or configuration options, always use enums (in both Rust and TypeScript). Booleans don't scale — a policy that starts as on/off almost always grows into multiple modes

## Code Quality Standards

These are enforced during code review. Follow them to avoid revision requests.

### Rust
- **Handle `Option`/`Result` properly** — don't unwrap carelessly; return errors or use `?` operator. Handle `None` cases explicitly rather than assuming values exist
- **Reuse types from `sequent-core::types`** — never duplicate types that already exist in sequent-core (e.g., use `sequent_core::types::ceremonies::CountingAlgType` instead of creating local `const &str` equivalents)
- **Use constants, not magic strings** — extract repeated string literals into named constants (e.g., `const APP_VERSION_DEV: &str = "dev"`)
- **Use Rust enums** — prefer enums with `Display`/`FromStr` over string constants when representing fixed sets of values

### TypeScript
- **No `any` types** — always use proper types. If the type is known, use it; if not, define one
- **Async error handling** — always `await` async calls and wrap them in `try/catch` where errors can occur
- **Environment variables** — prefix with the service name (e.g., `B4_` for B4, `HASURA_` for Hasura). When adding new env vars, also add them to the relevant `docker-compose` files

### General
- **No AI artifacts** — remove any AI-generated comments, explanatory notes, or boilerplate that wouldn't be written by a human developer. Reviewers actively flag these. However, do NOT remove existing useful comments written by developers — only remove clearly AI-generated or stale ones
- **No dead code or leftover comments** — remove commented-out code, unused imports, and stale comments before submitting. Preserve existing comments that explain non-obvious logic
- **No unresolved TODOs** — address or remove TODO comments before merge. If work is deferred, create a ticket and reference it
- **Backwards compatibility** — when changing data structures or configs, consider how this affects existing data. Make new fields optional or provide defaults to avoid breaking older election events on import

## PR & Branch Conventions

- **Branch naming**: `feat/meta-<ticket>-<description>/main` or `fix/meta-<ticket>/release/X.Y`
- **PR body**: must include `Parent issue: https://github.com/sequentech/meta/issues/<number>`
- **Every PR must link to a meta issue** — create one first if it doesn't exist
- **Support PRs** (targeting release branches) should not include release notes changes — release notes are auto-generated from the ticket description
- **Release branches**: `release/9.0`, `release/9.1`, `release/9.2`, `release/9.3`, `release/9.4` — same fix is cherry-picked across all applicable branches

## Development Workflow

**Use test-driven development (TDD):**
1. Write tests that define the expected behavior
2. Run the tests — they should fail (confirms the test is meaningful)
3. Implement the code
4. Run the tests — they should now pass

## New Feature Checklist

Before submitting a feature PR, ensure:
- **Docusaurus documentation** — new features require docs in `docs/docusaurus/`. The PR Preview bot auto-generates a preview at `docs.sequentech.io`
- **Keycloak permissions** — if adding new permissions, add them to the default tenant template in `keycloak.ts`
- **Tests** — add unit tests for new functions, including negative/edge-case tests (e.g., invalid input, `None` values, parse errors)
- **Responsive UI** — test frontend changes on different screen sizes

## Dev Environment

Recommended: VS Code Dev Containers or GitHub Codespaces. Services auto-start via Docker Compose with Nix/devenv.

Dev service URLs (inside dev container):
- Keycloak: http://127.0.0.1:8090 (admin/admin)
- Hasura: http://127.0.0.1:8080 (admin secret: "admin")
- Voting Portal: http://127.0.0.1:3000
- Admin Portal: http://127.0.0.1:3002
- ImmuDB: http://127.0.0.1:3325 (immudb/immudb)
- MinIO: http://127.0.0.1:9001
- RabbitMQ: http://127.0.0.1:15672

**Dev container tips**: When editing Rust code in harvest, windmill, or sequent-core, don't run `cargo build` to verify it — check the container logs (`docker logs windmill` / `docker logs harvest`) instead, since those services auto-rebuild on changes inside the dev container. See the note under Build Commands → Rust.
