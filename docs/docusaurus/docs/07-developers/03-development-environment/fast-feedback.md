---
id: fast-feedback
title: Fast feedback loops
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

The development loop is edit, see the result, run the relevant test. Commands in
this guide run from the repository root inside the devcontainer unless stated;
`scripts/dev/step-dev <command> --help` describes each one.

## Devcontainer modes

## Shared UI hot reload

## Screens, workbench and scenarios

## Incremental WASM

After editing `sequent-core` or a crate it depends on, run from the devenv shell:

```sh
scripts/dev/step-dev wasm            # rebuild and publish when inputs changed
scripts/dev/step-dev wasm --status   # does the published build match the sources?
scripts/dev/step-dev wasm --clean    # drop the development package and its cache
```

The command fingerprints the path crates Cargo resolves for the wasm32 build
(their library sources, manifests and files named by `include_str!`,
`include_bytes!` or `#[path]`), the resolved dependency versions, sources and
features, the workspace profiles, Cargo configuration, `rust-toolchain.toml`,
the build recipe, the rustc, Cargo and wasm-bindgen versions, C compiler settings
and the command itself. Tests, benches, examples, other workspace members,
unrelated `Cargo.lock` entries, hidden files and editor backups are not inputs.
Without changes it only prints `up to date`. Otherwise it compiles incrementally
in `packages/rust-local-target/sequent-core-wasm/` and publishes one development
package there; node_modules, Yarn caches and dist trees are untouched.

Portal dev servers (`start:*`) and Storybook load that package instead of the
installed tgz while it exists, and reload the open page when a new build is
published; restart them after the first publish or after `--clean`. Production
builds always use the installed tgz. A failed build exits non-zero, keeps the
previous build and logs `development WASM build failed` in the browser console
until a build succeeds or the sources match the published build again.

The committed `packages/*/rust/sequent-core-0.1.0.tgz` files are the release
package. It records its source fingerprint; regenerate it with wasm-opt from the
sequent-core flake, then reinstall and commit the four tgz files and `yarn.lock`:

```sh
nix develop ./packages/sequent-core --command scripts/dev/step-dev wasm --release-package
yarn --cwd packages install --frozen-lockfile
scripts/dev/step-dev wasm --check-package
```

`--check-package` fails, naming the changed inputs, when the committed package was
not built from the checked-out sources; CI runs it in `build_wasm.yml`.
For recovery, `.devcontainer/scripts/rebuild-sequent-core-full.sh` also removes
the installed copies so that the next install extracts them again.

## Focused tests

## Benchmarks

## Incremental CI
