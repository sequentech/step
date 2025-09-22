// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Strand is a cryptographic library for use in secure online voting protocols.
//!
//! ## Primitives
//!
//! The following primitives are implemented
//!
//! * ElGamal and exponential ElGamal encryption.
//!
//! * Fixed distributed and [threshold distributed ElGamal].
//!
//! * [Wikstrom] [shuffle] [proofs].
//!
//! * Schnorr and Chaum-Pedersen zero knowledge proofs.
//!
//! Shuffle proofs have been independently verified
//!
//! * [Did you mix me? - Formally Verifying Verifiable Mix Nets in Electronic
//!   Voting] using [OCamlBraid].
//!
//! ## Group backends
//!
//! The library supports pluggable [discrete log] backends, there are currently
//! three:
//!
//! * Curve25519 using the [ristretto group] via the [curve25519-dalek] library.
//! * [Standard multiplicative groups] via the [rug] arbitrary-precision
//!   library, backed by [gmp].
//! * [Standard multiplicative groups] via the [num-bigint] arbitrary-precision
//!   library, in pure rust.
//!
//! ## Significant dependencies
//!
//! * Compute intensive portions are parallelized using [rayon].
//! * Symmetric encryption using [RustCrypto](https://github.com/RustCrypto/block-ciphers).
//! * Serialization for transport and hashing using [borsh](https://crates.io/crates/borsh).
//! * Randomness is sourced from [rand::rngs::OsRng], in wasm builds [getrandom]
//!   is backed by [Crypto.getRandomValues].
//!
//! ## Development environment
//!
//! Strand uses [Github dev containers] to facilitate development. To start
//! developing strand, clone the github repo locally, and open the folder in
//! Visual Studio Code in a container. This will configure the same environment
//! that strand developers use, including installing required packages and VS
//! Code plugins.
//!
//! We've tested this dev container for Linux x86_64 and Mac Os arch64
//! architectures. Unfortunately at the moment it doesn't work with Github
//! Codespaces as nix doesn't work on Github Codespaces yet. Also the current
//! dev container configuration for strand doesn't allow commiting to the git
//! repo from the dev container, you should use git on a local terminal.
//!
//! ## building
//!
//! ```cargo build```
//! 
//! ### Build with parallelism
//!
//! Uses rayon's parallel collections for compute intensive operations
//! ```cargo build --features=rayon```
//!
//! ## unit tests
//!
//! ```cargo test```
//! 
//! because strand is a cryptographic library with compute intensive functionality, you may want to
//! run the tests in release mode
//! ```cargo test --release```
//!
//! ## wasm test
//!
//! See [here](https://github.com/sequentech/strand/tree/main/src/wasm/test).
//!
//! ## benchmarks
//!
//! See [here](https://github.com/sequentech/strand/tree/main/benches).
//!
//! ## Continuous Integration
//!
//! There are multiple checks executed through the usage of Github Actions to
//! verify the health of the code when pushed:
//! 1. **Compiler warning/errors**: checked using `cargo check` and
//! `cargo check ---tests`. Use `cargo fix` and `cargo fix --tests` to fix the
//! issues that appear.
//! 2. **Unit tests**: check that all unit tests pass using `cargo test`.
//! 3. **Code style**: check that the code style follows standard Rust format,
//!    using
//! `cargo fmt -- --check`. Fix it using `cargo fmt`.
//! 4. **Code linting**: Lint that checks for common Rust mistakes using
//! `cargo clippy`. You can try to fix automatically most of those mistakes
//! using `cargo clippy --fix -Z unstable-options`.
//! 5. **Code coverage**: Detects code coverage with [cargo-tarpaulin] and
//!    pushes
//! the information (in master branch) to [codecov].
//! 6. **License compliance**: Check using [REUSE] for license compliance within
//! the project, verifying that every file is REUSE-compliant and thus has a
//! copyright notice header. Try fixing it with `reuse lint`.
//! 7. **Dependencies scan**: Audit dependencies for security vulnerabilities in
//!    the
//! [RustSec Advisory Database], unmaintained dependencies, incompatible
//! licenses and banned packages using [cargo-deny]. Use `cargo deny fix` or
//! `cargo deny --allow-incompatible` to try to solve the detected issues. We
//! also have configured [dependabot] to notify and create PRs on version
//! updates.
//! 8. **Benchmark performance**: Check benchmark performance and alert on
//! regressions using `cargo bench` and [github-action-benchmark].
//! 9. **CLA compliance**: Check that all committers have signed the
//! [Contributor License Agreement] using [CLA Assistant bot].
//! 10. **Browser testing**: Check the library works on different browsers and
//!     operating
//! systems using [browserstack](https://www.browserstack.com/). Run `npm run local`
//! on the `browserstack` folder to try it locally. You'll need to configure the
//! env variables `GIT_COMMIT_SHA`, `BROWSERSTACK_USERNAME`,
//! `BROWSERSTACK_ACCESS_KEY`.
//!
//! [cargo-deny]: https://github.com/EmbarkStudios/cargo-deny
//! [cargo-edit]: https://crates.io/crates/cargo-edit
//! [codecov]: https://codecov.io/
//! [REUSE]: https://reuse.software/
//! [cargo-tarpaulin]: https://github.com/xd009642/tarpaulin
//! [github-action-benchmark]: https://github.com/benchmark-action/github-action-benchmark
//! [Contributor License Agreement]: https://cla-assistant.io/sequentech/strand?pullRequest=27
//! [CLA Assistant bot]: https://github.com/cla-assistant/cla-assistant
//! [dependabot]:https://docs.github.com/en/code-security/dependabot/dependabot-version-updates/configuring-dependabot-version-updates
//! [RustSec Advisory Database]: https://github.com/RustSec/advisory-db/
//! [rayon]: https://github.com/rayon-rs/rayon
//! [threshold distributed ElGamal]: https://members.loria.fr/VCortier/files/Papers/WPES2013.pdf
//! [Wikstrom]: https://www.csc.kth.se/~dog/research/papers/TW10Conf.pdf
//! [shuffle]: https://eprint.iacr.org/2011/168.pdf
//! [proofs]: https://www.ifca.ai/fc17/voting/papers/voting17_HLKD17.pdf
//! [Did you mix me? - Formally Verifying Verifiable Mix Nets in Electronic Voting]: https://eprint.iacr.org/2020/1114.pdf
//! [OCamlBraid]: https://github.com/nvotes/secure-e-voting-with-coq/tree/master/OCamlBraid
//! [discrete log]: https://en.wikipedia.org/wiki/Decisional_Diffie%E2%80%93Hellman_assumption
//! [ristretto group]: https://ristretto.group/
//! [curve25519-dalek]: https://github.com/dalek-cryptography/curve25519-dalek
//! [Standard multiplicative groups]: https://en.wikipedia.org/wiki/Schnorr_group
//! [rug]: https://crates.io/crates/rug
//! [gmp]: https://gmplib.org/
//! [num-bigint]: https://crates.io/crates/num-bigint
//! [rand::rngs::OsRng]: https://docs.rs/rand/latest/rand/rngs/struct.OsRng.html
//! [getrandom]: https://crates.io/crates/getrandom
//! [Crypto.getRandomValues]: https://www.w3.org/TR/WebCryptoAPI/#Crypto-method-getRandomValues
//! [Nix Package Manager]: https://nixos.org/
//! [install Nix]: https://nixos.org/
//! [Github dev containers]: https://docs.github.com/en/codespaces/setting-up-your-project-for-codespaces/introduction-to-dev-containers

// #![doc = include_str!("../README.md")]
extern crate cfg_if;
/// Provides implementation of modular arithmetic backends based on discrete log
/// assumptions.
pub mod backend;
/// Defines a generic interface to concrete modular arithmetic backends based on
/// discrete log assumptions.
pub mod context;

/// ElGamal encryption.
pub mod elgamal;
/// Wikstrom proof of shuffle following [TW10](https://www.csc.kth.se/~dog/research/papers/TW10Conf.pdf). See also [HLDK17](https://arbor.bfh.ch/8269/1/HLKD17.pdf).
#[cfg(any(test, not(feature = "wasm")))]
pub mod shuffler;
#[cfg(any(test, not(feature = "wasm")))]
pub mod shuffler_product;
/// Distributed ElGamal threshold cryptosystem following [Pedersen91](https://link.springer.com/chapter/10.1007/3-540-46766-1_9).
/// See also [CGGI13](https://members.loria.fr/VCortier/files/Papers/WPES2013.pdf).
#[cfg(any(test, not(feature = "wasm")))]
pub mod threshold;
/// Schnorr and Chaum-Pedersen zero knowledge proofs.
pub mod zkp;

/// Hashing.
mod hashing;
/// Random number generation.
mod random;
/// Signature frontend.
mod signatures;
/// Symmetric encryption frontend.
#[cfg(not(feature = "wasm"))]
mod symmetric;

cfg_if::cfg_if! {
    if #[cfg(feature = "openssl_core")] {
        /// Random number generation backed by [OpenSSL](https://crates.io/crates/openssl).
        pub use random::openssl as rng;
        /// SHA-2 hashing backed by [OpenSSL](https://crates.io/crates/openssl).
        pub use hashing::openssl as hash;
        /// AES-GCM backed by [OpenSSL](https://crates.io/crates/openssl).
        pub use symmetric::openssl as symm;
        /// Ed25519 digital signatures backed by [dalek](https://github.com/dalek-cryptography/curve25519-dalek/tree/main/ed25519-dalek).
        pub use signatures::dalek as signature;
    }
    else if #[cfg(feature = "openssl_full")] {
        pub use random::openssl as rng;
        /// SHA-2 hashing backed by [OpenSSL](https://crates.io/crates/openssl).
        pub use hashing::openssl as hash;
        /// AES-GCM backed by [OpenSSL](https://crates.io/crates/openssl).
        pub use symmetric::openssl as symm;
        /// EcDSA digital signatures backed by [OpenSSL](https://crates.io/crates/openssl).
        pub use signatures::openssl as signature;
    }
    else if #[cfg(feature = "wasm")] {
        /// Webassembly API.
        pub mod wasm;
        /// Ed25519 digital signatures backed by [dalek](https://github.com/dalek-cryptography/curve25519-dalek/tree/main/ed25519-dalek).
        pub use signatures::dalek as signature;
        /// Random number generation backed by [rand](https://crates.io/crates/rand).
        pub use random::rand as rng;
        /// SHA-2 hashing backed by [rustcrypto](https://crates.io/crates/sha2).
        pub use hashing::rustcrypto as hash;
    }
    else {
        /// Ed25519 digital signatures backed by [dalek](https://github.com/dalek-cryptography/curve25519-dalek/tree/main/ed25519-dalek).
        pub use signatures::dalek as signature;
        /// Random number generation backed by [rand](https://crates.io/crates/rand).
        pub use random::rand as rng;
        /// SHA-2 hashing backed by [rustcrypto](https://crates.io/crates/sha2).
        pub use hashing::rustcrypto as hash;
        /// Chacha20poly1305 backed by [rustcrypto](https://docs.rs/chacha20poly1305/latest/chacha20poly1305/).
        pub use symmetric::rustcrypto as symm;
    }
}

/// Miscellaneous functions.
#[doc(hidden)]
pub mod util;

/// Serialization frontend. StrandVectors for parallel serialization.
#[doc(hidden)]
pub mod serialization;

/// Support for distributed Elgamal.
#[allow(dead_code)]
#[cfg(test)]
mod keymaker;

use std::collections::HashMap;

pub fn info() -> HashMap<&'static str, String> {
    let mut info = HashMap::new();
    let version = option_env!("CARGO_PKG_VERSION").unwrap_or("unknown");
    let hash = crate::hash::info();
    let random = crate::rng::info();
    let signature = crate::signature::info();
    #[cfg(not(feature = "wasm"))]
    let symmetric = crate::symm::info();

    info.insert("VERSION", version.to_string());
    info.insert("HASH", hash);
    info.insert("RNG", random);
    info.insert("SIGNATURE", signature);
    #[cfg(not(feature = "wasm"))]
    info.insert("SYMMETRIC", symmetric);

    info
}

pub fn info_string() -> String {
    let info = info();
    let ret = format!(
        "
===============================================================================
strand

Version:    {}
HASH:       {}
RNG:        {}
SIGNATURE:  {}
SYMMETRIC:  {}
===============================================================================
",
        info.get("VERSION").unwrap(),
        info.get("HASH").unwrap(),
        info.get("RNG").unwrap(),
        info.get("SIGNATURE").unwrap(),
        info.get("SYMMETRIC").unwrap()
    );

    ret
}
