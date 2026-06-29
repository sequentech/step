<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Braid

[![Chat][discord-badge]][discord-link]
[![Build Status][build-badge]][build-link]
[![codecov][codecov-badge]][codecov-link]
[![Dependency status][dependencies-badge]][dependencies-link]
[![License][license-badge]][license-link]
[![REUSE][reuse-badge]][reuse-link]

## Sequent verifiable re-encryption mixnet written in Rust

![Demo](https://raw.githubusercontent.com/sequentech/braid/v2/resources/demo.png)

Braid is a verifiable re-encryption mixnet written in Rust that can serve as the 
cryptographic core of secure voting systems. 

## Development environment

Braid uses [Github dev containers] to facilitate development. To start developing braid,
clone the github repo locally, and open the folder in Visual Studio Code in a container. This
will configure the same environment that braid developers use, including installing required
packages and VS Code plugins.

We've tested this dev container for Linux x86_64 and Mac Os arch64 architectures. Unfortunately
at the moment it doesn't work with Github Codespaces as nix doesn't work on Github Codespaces yet.
Also the current dev container configuration for braid doesn't allow commiting to the git repo
from the dev container, you should use git on a local terminal.

## Nix reproducible builds

braid uses the [Nix Package Manager] as its package
builder. To build braid, **first [install Nix]** correctly
in your system. If you're running the project on a dev container,
you shouldn't need to install it.

After you have installed Nix, enter the development environment with:

```bash
nix develop
```

## Updating Cargo.toml

Use the following [cargo-edit] command to upgrade dependencies to latest
available version. This can be done within the `nix develop` environment:

```bash
cargo upgrade -Z preserve-precision
```

This repository doesn´t include a `Cargo.lock` file as it is intended to work as a library. However for Wasm tests we keep a copy of the file on `Cargo.lock.copy`. If you update Cargo.toml, keep the lock copy file in sync by generating the lock file with `cargo generate-lockfile`, then `mv Cargo.lock Cargo.lock.copy` and commit the changes.

## Build

This is a project written in [Rust] and uses `cargo`. It uses [nix](https://nixos.org/) to create reproducible builds. In order to build the project as a library for the host system, run:

```nix build```

If you don't want to use nix, you can build the project with:

```cargo build```

## Tests

* Repl

`cargo run --bin test_repl --features=repl`

* Full cycle run

`cargo test --release`

## Status

Prototype. Do not use in production.

## Dependencies

The mixnet supports pluggable [discrete log](https://en.wikipedia.org/wiki/Decisional_Diffie%E2%80%93Hellman_assumption) backends, there are currently two:

* Curve25519 using the [ristretto group](https://ristretto.group/) via the [curve25519-dalek](https://github.com/dalek-cryptography/curve25519-dalek) library.
* [Standard multiplicative groups](https://en.wikipedia.org/wiki/Schnorr_group) via the [rug](https://crates.io/crates/rug) arbitrary-precision library, backed by [gmp](https://gmplib.org/).

Other significant dependencies:

* Compute intensive portions are parallelized using [rayon](https://github.com/rayon-rs/rayon).
* The protocol is declaratively expressed in a [datalog](https://en.wikipedia.org/wiki/Datalog) variant using [crepe](https://github.com/ekzhang/crepe).
* Message signatures are provided by [ed25519-zebra](https://crates.io/crates/ed25519-zebra).

Previous versions of the mixnet have been analyzed with [clingo](https://github.com/potassco/clingo-rs).

## Continuous Integration

There are multiple checks executed through the usage of Github Actions to verify
the health of the code when pushed:
1. **Compiler warning/errors**: checked using `cargo check` and 
`cargo check ---tests`. Use `cargo fix` and `cargo fix --tests` to fix the 
issues that appear.
2. **Unit tests**: check that all unit tests pass using `cargo test`.
3. **Code style**: check that the code style follows standard Rust format, using
`cargo fmt -- --check`. Fix it using `cargo fmt`.
4. **Code linting**: Lint that checks for common Rust mistakes using 
`cargo clippy`. You can try to fix automatically most of those mistakes using
`cargo clippy --fix -Z unstable-options`.
5. **Code coverage**: Detects code coverage with [grcov] and pushes the 
information (in master branch) to [codecov].
6. **License compliance**: Check using [REUSE] for license compliance within
the project, verifying that every file is REUSE-compliant and thus has a 
copyright notice header.
7. **Dependencies scan**: Audit dependencies for security vulnerabilities in the
[RustSec Advisory Database], unmaintained dependencies, incompatible licenses 
and banned packages using [cargo-deny]. Use `cargo deny fix` or 
`cargo deny --allow-incompatible` to try to solve the detected issues.
8. **Benchmark performance**: Check benchmark performance and alert on
regressions using `cargo bench` and [github-action-benchmark].
9. **CLA compliance**: Check that all committers have signed the 
[Contributor License Agreement] using [CLA Assistant bot].

## Papers

Braid uses standard crytpographic techniques, most significantly

* [Proofs of Restricted Shuffles](https://www.csc.kth.se/~dog/research/papers/TW10Conf.pdf)

* [A Commitment-Consistent Proof of a Shuffle](https://eprint.iacr.org/2011/168.pdf)

* [Pseudo-Code Algorithms for Verifiable Re-Encryption Mix-Nets](https://www.ifca.ai/fc17/voting/papers/voting17_HLKD17.pdf)

Shuffle proofs have been independently verified

* [Did you mix me? Formally Verifying Verifiable Mix Nets in Electronic Voting](https://eprint.iacr.org/2020/1114.pdf) using [this](https://github.com/nvotes/secure-e-voting-with-coq/tree/master/OCamlBraid).

[Sequent]: https://sequentech.io
[Rust]: https://www.rust-lang.org/
[grcov]: https://crates.io/crates/grcov

[cargo-deny]: https://github.com/EmbarkStudios/cargo-deny
[cargo-edit]: https://crates.io/crates/cargo-edit
[codecov]: https://codecov.io/
[REUSE]: https://reuse.software/
[cargo-tarpaulin]: https://github.com/xd009642/tarpaulin
[github-action-benchmark]: https://github.com/benchmark-action/github-action-benchmark
[Github dev containers]: https://docs.github.com/en/codespaces/setting-up-your-project-for-codespaces/introduction-to-dev-containers

[Contributor License Agreement]: https://cla-assistant.io/sequentech/braid?pullRequest=27
[CLA Assistant bot]: https://github.com/cla-assistant/cla-assistant
[dependabot]:https://docs.github.com/en/code-security/dependabot/dependabot-version-updates/configuring-dependabot-version-updates
[RustSec Advisory Database]: https://github.com/RustSec/advisory-db/
[Nix Package Manager]: https://nixos.org/
[install Nix]: https://nixos.org/

[discord-badge]: https://img.shields.io/discord/1006401206782001273?style=plastic
[discord-link]: https://discord.gg/WfvSTmcdY8

[build-badge]: https://github.com/sequentech/braid/workflows/CI/badge.svg?branch=main&event=push
[build-link]: https://github.com/sequentech/braid/actions?query=workflow%3ACI

[codecov-badge]: https://codecov.io/gh/sequentech/braid/branch/main/graph/badge.svg?token=W5QNYDEJCX
[codecov-link]: https://codecov.io/gh/sequentech/braid

[dependencies-badge]: https://deps.rs/repo/github/sequentech/braid/status.svg
[dependencies-link]: https://deps.rs/repo/github/sequentech/braid

[license-badge]: https://img.shields.io/github/license/sequentech/braid?label=license
[license-link]: https://github.com/sequentech/braid/blob/master/LICENSE

[reuse-badge]: https://api.reuse.software/badge/github.com/sequentech/braid
[reuse-link]: https://api.reuse.software/info/github.com/sequentech/braid
