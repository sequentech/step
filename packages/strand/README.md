<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
# strand

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

### Build with parallelism

Uses rayon's parallel collections for compute intensive operations

```cargo build --features=rayon```

## unit tests

```cargo test```

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
