<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Braid

## Sequent verifiable re-encryption mixnet written in Rust

![Demo](https://raw.githubusercontent.com/sequentech/braid/v2/resources/demo.png)

Braid is a verifiable re-encryption mixnet written in Rust that can serve as the 
cryptographic core of secure voting systems. 

## Dependencies

The mixnet supports pluggable [discrete log](https://en.wikipedia.org/wiki/Decisional_Diffie%E2%80%93Hellman_assumption) backends, there are currently two:

* Curve25519 using the [ristretto group](https://ristretto.group/) via the [curve25519-dalek](https://github.com/dalek-cryptography/curve25519-dalek) library.
* [Standard multiplicative groups](https://en.wikipedia.org/wiki/Schnorr_group) via the [rug](https://crates.io/crates/rug) arbitrary-precision library, backed by [gmp](https://gmplib.org/).

Other significant dependencies:

* Compute intensive portions are parallelized using [rayon](https://github.com/rayon-rs/rayon).
* The protocol is declaratively expressed in a [datalog](https://en.wikipedia.org/wiki/Datalog) variant using [crepe](https://github.com/ekzhang/crepe).
* Message signatures are provided by [ed25519-zebra](https://crates.io/crates/ed25519-zebra).

Previous versions of the mixnet have been analyzed with [clingo](https://github.com/potassco/clingo-rs).

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
