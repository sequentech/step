<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
# strand

[![Chat][discord-badge]][discord-link]
[![Build Status][build-badge]][build-link]
[![codecov][codecov-badge]][codecov-link]
[![Dependency status][dependencies-badge]][dependencies-link]
[![License][license-badge]][license-link]
[![REUSE][reuse-badge]][reuse-link]

Strand is a cryptographic library for use in secure online voting protocols. 

## Primitives

The following primitives are implemented

* ElGamal and exponential ElGamal encryption.

* Fixed distributed and [threshold distributed ElGamal].

* [Wikstrom] [shuffle] [proofs].

* Schnorr and Chaum-Pedersen zero knowledge proofs.

Shuffle proofs have been independently verified

* [Did you mix me? - Formally Verifying Verifiable Mix Nets in Electronic Voting] using [OCamlBraid].

## Group backends

The library supports pluggable [discrete log] backends, there are currently three:

* Curve25519 using the [ristretto group] via the [curve25519-dalek] library.
* [Standard multiplicative groups] via the [rug] arbitrary-precision library, backed by [gmp].
* [Standard multiplicative groups] via the [num-bigint] arbitrary-precision library, in pure rust.

## Significant dependencies

* Compute intensive portions are parallelized using [rayon].
* Symmetric encryption using [RustCrypto](https://github.com/RustCrypto/block-ciphers).
* Serialization for transport and hashing using [borsh](https://crates.io/crates/borsh).
* Randomness is sourced from [rand::rngs::OsRng], in wasm builds [getrandom] is backed by [Crypto.getRandomValues].

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
5. **Code coverage**: Detects code coverage with [cargo-tarpaulin] and pushes
the information (in master branch) to [codecov].
6. **License compliance**: Check using [REUSE] for license compliance within
the project, verifying that every file is REUSE-compliant and thus has a 
copyright notice header. Try fixing it with `reuse lint`.
7. **Dependencies scan**: Audit dependencies for security vulnerabilities in the
[RustSec Advisory Database], unmaintained dependencies, incompatible licenses
and banned packages using [cargo-deny]. Use `cargo deny fix` or
`cargo deny --allow-incompatible` to try to solve the detected issues. We also
have configured [dependabot] to notify and create PRs on version updates.
8. **Benchmark performance**: Check benchmark performance and alert on
regressions using `cargo bench` and [github-action-benchmark].
9. **CLA compliance**: Check that all committers have signed the 
[Contributor License Agreement] using [CLA Assistant bot].
10. **Browser testing**: Check the library works on different browsers and operating
systems using [browserstack](https://www.browserstack.com/). Run `npm run local`
on the `browserstack` folder to try it locally. You'll need to configure the env variables 
`GIT_COMMIT_SHA`, `BROWSERSTACK_USERNAME`, `BROWSERSTACK_ACCESS_KEY`.

## Development environment

Strand uses [Github dev containers] to facilitate development. To start developing strand,
clone the github repo locally, and open the folder in Visual Studio Code in a container. This
will configure the same environment that strand developers use, including installing required
packages and VS Code plugins.

We've tested this dev container for Linux x86_64 and Mac Os arch64 architectures. Unfortunately
at the moment it doesn't work with Github Codespaces as nix doesn't work on Github Codespaces yet.
Also the current dev container configuration for strand doesn't allow commiting to the git repo
from the dev container, you should use git on a local terminal.

## Nix reproducible builds

strand uses the [Nix Package Manager] as its package
builder. To build strand, **first [install Nix]** correctly
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

## building

This project uses [nix](https://nixos.org/) to create reproducible builds. In order to build the project as a library for the host system, run:

```nix build```

You can build the project as a WASM library with:

```nix build .#strand-wasm```

If you don't want to use nix, you can build the project with:

```cargo build```

### Build with parallelism

Uses rayon's parallel collections for compute intensive operations

```cargo build --features=rayon```

## unit tests

```cargo test```

## wasm test

See [here](https://github.com/sequentech/strand/tree/main/src/wasm/test).

## benchmarks

See [here](https://github.com/sequentech/strand/tree/main/benches).

[cargo-deny]: https://github.com/EmbarkStudios/cargo-deny
[cargo-edit]: https://crates.io/crates/cargo-edit
[codecov]: https://codecov.io/
[REUSE]: https://reuse.software/
[cargo-tarpaulin]: https://github.com/xd009642/tarpaulin
[github-action-benchmark]: https://github.com/benchmark-action/github-action-benchmark
[Contributor License Agreement]: https://cla-assistant.io/sequentech/strand?pullRequest=27
[CLA Assistant bot]: https://github.com/cla-assistant/cla-assistant
[dependabot]:https://docs.github.com/en/code-security/dependabot/dependabot-version-updates/configuring-dependabot-version-updates
[RustSec Advisory Database]: https://github.com/RustSec/advisory-db/
[rayon]: https://github.com/rayon-rs/rayon
[threshold distributed ElGamal]: https://members.loria.fr/VCortier/files/Papers/WPES2013.pdf
[Wikstrom]: https://www.csc.kth.se/~dog/research/papers/TW10Conf.pdf
[shuffle]: https://eprint.iacr.org/2011/168.pdf
[proofs]: https://www.ifca.ai/fc17/voting/papers/voting17_HLKD17.pdf
[Did you mix me? - Formally Verifying Verifiable Mix Nets in Electronic Voting]: https://eprint.iacr.org/2020/1114.pdf
[OCamlBraid]: https://github.com/nvotes/secure-e-voting-with-coq/tree/master/OCamlBraid
[discrete log]: https://en.wikipedia.org/wiki/Decisional_Diffie%E2%80%93Hellman_assumption
[ristretto group]: https://ristretto.group/
[curve25519-dalek]: https://github.com/dalek-cryptography/curve25519-dalek
[Standard multiplicative groups]: https://en.wikipedia.org/wiki/Schnorr_group
[rug]: https://crates.io/crates/rug
[gmp]: https://gmplib.org/
[num-bigint]: https://crates.io/crates/num-bigint
[rand::rngs::OsRng]: https://docs.rs/rand/latest/rand/rngs/struct.OsRng.html
[getrandom]: https://crates.io/crates/getrandom
[Crypto.getRandomValues]: https://www.w3.org/TR/WebCryptoAPI/#Crypto-method-getRandomValues
[Nix Package Manager]: https://nixos.org/
[install Nix]: https://nixos.org/
[Github dev containers]: https://docs.github.com/en/codespaces/setting-up-your-project-for-codespaces/introduction-to-dev-containers

[discord-badge]: https://img.shields.io/discord/1006401206782001273?style=plastic
[discord-link]: https://discord.gg/WfvSTmcdY8

[build-badge]: https://github.com/sequentech/strand/workflows/CI/badge.svg?branch=main&event=push
[build-link]: https://github.com/sequentech/strand/actions?query=workflow%3ACI

[codecov-badge]: https://codecov.io/gh/sequentech/strand/branch/main/graph/badge.svg?token=W5QNYDEJCX
[codecov-link]: https://codecov.io/gh/sequentech/strand

[dependencies-badge]: https://deps.rs/repo/github/sequentech/strand/status.svg
[dependencies-link]: https://deps.rs/repo/github/sequentech/strand

[license-badge]: https://img.shields.io/github/license/sequentech/strand?label=license
[license-link]: https://github.com/sequentech/strand/blob/master/LICENSE

[reuse-badge]: https://api.reuse.software/badge/github.com/sequentech/strand
[reuse-link]: https://api.reuse.software/info/github.com/sequentech/strand
