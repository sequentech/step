<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# strand threat model

strand is the platform's cryptographic library: Ristretto255 group operations (curve25519-dalek), ElGamal, Schnorr and Chaum–Pedersen proofs, the Terelius–Wikström shuffle proof, threshold key generation with Feldman commitments, Ed25519 signatures (ECDSA P-384 under `fips_full`), AEAD (ChaCha20-Poly1305 by default, AES-256-GCM under `fips_core`/`fips_full`, none under `wasm`) and SHA-512/SHAKE256. `src/lib.rs` selects the backends by Cargo feature. It has no network, storage or logging of its own. It is linked into the voter and admin browsers (through the sequent-core WASM build), braid trustee machines, the b3 board, the windmill, harvest and electoral-log containers, and operator tools (velvet, step-cli). Production code uses `src/backend/ristretto.rs` `RistrettoCtx`; the multiplicative-group backends (`num_bigint.rs`, `rug.rs`, `malachite.rs`) serve only tests and benchmarks. Ballot secrecy and tally integrity depend on its proofs being sound and its secrets staying secret. See the [system threat model](../../THREAT_MODEL.md).

## Assets

- **Trustee key shares**: ElGamal private keys, DKG polynomial coefficients and the shares trustees send each other (`src/elgamal.rs` `PrivateKey`, `src/threshold.rs`, `RistrettoCtx::encrypt_exp`). Confidentiality: enough shares decrypt every ballot. Integrity: a wrong share gives a wrong joint key or a wrong decryption.
- **Trustee channel key**: a `PrivateKey` that braid AEAD-encrypts and posts to the board, and that windmill exports to the admin portal as ciphertext. Confidentiality: it protects every DKG share sent to that trustee.
- **Election public key** (`src/elgamal.rs` `PublicKey`). Integrity: encrypting to a substituted or degenerate key loses ballot secrecy.
- **Ballot plaintext and encryption randomness**: `PublicKey::encrypt_and_pok` creates them in the voter's browser and returns the randomness for ballot auditing. Confidentiality: either one reveals the vote.
- **Mix secrets**: the permutation, re-encryption factors and commitment randomness from `src/shuffler.rs` `Shuffler::gen_shuffle` and `gen_proof`. Confidentiality: disclosure links ballots to voters.
- **Proof soundness**: the shuffle, decryption, plaintext-knowledge and Feldman checks. Integrity: they make the tally verifiable.
- **Signing keys**: Ed25519 keys of trustees, the protocol manager, the electoral log and the ephemeral per-ballot voter key (`src/signatures/dalek.rs` `StrandSignatureSk`). Confidentiality and integrity: they authenticate board messages, audit-log entries and ballots.
- **Symmetric keys and AEAD ciphertexts** (`src/symmetric/`): they protect trustee channel keys, windmill vault secrets and voter secret attributes. Confidentiality and integrity.
- **Randomness** (`src/random/` `StrandRng`): the single source for keys, nonces, ballot randomness and permutations. Unpredictability.

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| Borsh decoding through `src/serialization.rs` `StrandDeserialize`: `Ciphertext`, `PublicKey`, `PrivateKey`, `Schnorr`, `ChaumPedersen`, `ShuffleProof`, `StrandVector`, `StrandRectangle` | braid, b3, windmill, sequent-core and velvet, on bytes from the board, the database or a voter | untrusted | `BorshDeserialize for RistrettoPointS` (the point must decompress) and `for ScalarS` (canonical scalar) in `src/backend/ristretto.rs`; `try_from_slice` consumes every byte; `StrandRectangle::new` rejects non-rectangular input |
| `src/zkp.rs` `Zkp::encryption_popk_verify` | windmill cast-vote path; sequent-core ballot verification | authenticated voter | Schnorr check with a recomputed challenge |
| `src/shuffler.rs` `Shuffler::check_proof`; `src/shuffler_product.rs` `Shuffler::check_proof` | braid trustees checking another trustee's mix; braid `verify` | trustee (any one may be malicious) | TW10/HLKD17 proof verification; challenge recomputed from the statement and the label |
| `src/threshold.rs` `verify_share`, `verification_key_factor`, `verify_decryption_factor`, `lagrange`; `RistrettoCtx::decrypt_exp` | braid trustees during DKG and tally | trustee | Feldman check `g^share == ∏ C_i^(t^i)`; Chaum–Pedersen check; share decryption requires two ciphertexts and a canonical scalar |
| `src/signatures/dalek.rs` `StrandSignaturePk::verify`, `from_bytes`, `from_der_b64_string`; `StrandSignatureSk::from_der_b64_string` | b3, braid, electoral-log, windmill, sequent-core | untrusted (messages); operator (keys from configuration or the vault) | ed25519-dalek verification; SPKI and PKCS#8 parsing |
| `src/symmetric/` `decrypt` | braid (channel key); windmill (vault, voter secret attributes) | operator / internal service | AEAD tag verification |
| `src/elgamal.rs` `PublicKey::encrypt_and_pok`; `src/backend/ristretto.rs` `RistrettoCtx::encode` (WASM) | voter browser through sequent-core | authenticated voter | Fresh randomness from `StrandRng` (`crypto.getRandomValues`) |
| `src/backend/ristretto.rs` `RistrettoCtx::decode` | braid, windmill and velvet, on decrypted ballots | untrusted (any element a voter encrypted) | Returns 30 bytes for any element; format checks belong to the caller |
| `src/backend/ristretto.rs` `RistrettoCtx::generators` | braid, with a seed built from the configuration, batch and mix number | trustee / internal service | SHAKE256 of the seed; no secret input |
| `src/wasm/test/` `#[wasm_bindgen]` test and bench exports | page JavaScript, in WASM builds that include them | untrusted | Feature-gated; no secret input |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| strand-T1 | Tampering | Invalid, non-canonical or degenerate elements and scalars in serialized ciphertexts, keys or proofs | `src/backend/ristretto.rs` `BorshDeserialize for RistrettoPointS` (`CompressedRistretto::decompress`), `BorshDeserialize for ScalarS` (`Scalar::from_canonical_bytes`), `RistrettoCtx::element_from_bytes`, `exp_from_bytes`; Ristretto has prime order. The bigint backends check range and quadratic residuosity (`src/backend/num_bigint.rs` `BigintCtx::element_from_biguint`). Rejecting degenerate values where the protocol forbids them is a caller duty | Partial |
| strand-T2 | Tampering | Weak Fiat–Shamir: a challenge that leaves out part of the statement, an ambiguous encoding, or a proof accepted in another protocol | `src/zkp.rs` `ChallengeInput` (Borsh map with named keys, sorted and length-prefixed); the Schnorr, Chaum–Pedersen and shuffle challenges hash the proof inputs, commitments and label; `RistrettoCtx::hash_to_exp` reduces 64 bytes of SHA-512 with `Scalar::from_bytes_mod_order_wide` | Not verified |
| strand-T3 | Tampering / Information disclosure | A malicious mixing trustee posts a shuffle that alters, drops or links ballots, with a proof that verifies | TW10/HLKD17 shuffle proof verification; generators as in strand-T8; each trustee verifies a mix before signing it | Not verified |
| strand-T4 | Denial of service | A malformed proof, artifact or key makes a caller panic, so the protocol stalls | Borsh decoding returns errors (`src/serialization.rs` `StrandDeserialize`); verifiers report failure as `bool` or `Result`; `RistrettoCtx::decrypt_exp` checks the ciphertext count | Not verified |
| strand-T5 | Spoofing / Tampering | A ballot is accepted from a submitter who did not create it, such as a copy or re-randomization of another voter's ballot | `src/elgamal.rs` `PublicKey::encrypt_and_pok`; `src/zkp.rs` `Zkp::encryption_popk`, `encryption_popk_verify` (Schnorr proof of knowledge of the encryption randomness) | Not verified |
| strand-T6 | Tampering | A trustee publishes a wrong partial decryption | `src/threshold.rs` `decryption_factor`, `verify_decryption_factor`; `src/zkp.rs` `Zkp::decryption_proof`, `verify_decryption` (Chaum–Pedersen over the generator, `gr`, the trustee's verification key and the factor, bound to `mhr` and the label) | Mitigated |
| strand-T7 | Tampering | A dealer's DKG shares disagree with its commitments, giving a wrong joint key or decryption | `src/threshold.rs` `gen_coefficients` (Feldman commitments), `verification_key_factor`, `verify_share`; shares travel ElGamal-encrypted to the recipient (`RistrettoCtx::encrypt_exp`, `decrypt_exp`). The counts of commitments and trustee positions are validated by braid | Partial |
| strand-T8 | Tampering | Shuffle generators with a known relation make the permutation commitment non-binding | `src/backend/ristretto.rs` `RistrettoCtx::generators`: SHAKE256 of the seed, mapped to points with `RistrettoPoint::from_uniform_bytes`; braid supplies the seeds | Not verified |
| strand-T9 | Information disclosure | Weak or failing randomness exposes keys, ballot randomness, proof nonces or the permutation | `src/random/rand.rs` `StrandRng` (`OsRng`; in WASM, getrandom `js` uses `crypto.getRandomValues`) and `src/random/openssl.rs` `StrandRng` (`rand_bytes`), both panicking on failure; `RistrettoCtx::rnd_exp` (64-byte wide reduction); `gen_permutation` (`SliceRandom::shuffle` over `StrandRng`) | Mitigated |
| strand-T10 | Spoofing / Repudiation | Forged or malleable signatures on board messages, audit-log entries or ballots | `src/signatures/dalek.rs` `StrandSignaturePk::verify` (ed25519-dalek). Ordering and replay protection belong to b3 and braid | Not verified |
| strand-T11 | Tampering | Encrypted secrets (channel key, vault entries, voter attributes) are modified, swapped or replayed | `src/symmetric/` `encrypt` / `decrypt`: AEAD tag verification; random 96-bit nonce from `StrandRng` | Not verified |
| strand-T12 | Information disclosure | Timing side channels leak secret scalars or the ballot plaintext | curve25519-dalek constant-time scalar multiplication (`RistrettoCtx::gmod_pow` with `RISTRETTO_BASEPOINT_TABLE`, `emod_pow`) and constant-time point and scalar equality. Not every operation on secret data is confirmed constant time; the bigint backends are variable time and test-only | Partial |
| strand-T13 | Information disclosure | Secrets (key shares, coefficients, mix secrets, symmetric keys, ballot randomness) stay in memory or reach logs | `src/elgamal.rs` `PrivateKey`, `src/symmetric/` `EncryptionData` and `src/signatures/dalek.rs` `StrandSignatureSk` have no `Debug`; `StrandSignatureSk` has no Borsh or serde, and its `SigningKey` is zeroized on drop; no logging dependency | Partial |
| strand-T14 | Tampering | A flawed or misconfigured crypto dependency ships | `Cargo.toml` pins `curve25519-dalek =4.1` and `ed25519-dalek =2.1` (`packages/Cargo.lock`: 4.1.3 and 2.1.1) | Not verified |
| strand-T15 | Tampering | Test-only or unreviewed code reaches a shipped artifact | Cargo feature gating (`src/lib.rs`); no workspace crate enables `certs` | Partial |
| strand-T16 | Tampering / Denial of service | A voter encrypts a group element that is not a valid ballot encoding, so decryption yields arbitrary bytes | `src/backend/ristretto.rs` `RistrettoCtx::encode` (try-and-increment into bytes 1..30); `RistrettoCtx::decode` returns 30 bytes for any element without panicking. strand cannot know the ballot format, so validating the decoded bytes is the ballot codec's job | Accepted (by design: format checks live in sequent-core) |

## Assumptions

- **braid** validates every artifact and the threshold before it calls strand, and aborts when a verifier returns false or an error. It derives generator seeds and proof labels from the configuration, the batch and the mix number. Its DKG hash numbering after dropping NULL entries (`../braid/src/protocol/action/dkg.rs`), the verifier underflow with fewer mixes than the threshold (`../braid/src/verify/verifier.rs`) and the missing shape check on `Shares` artifacts (braid-T23): Open on release/10.0 (fix in sequentech/step#3522).
- **b3 and braid** authenticate, order and deduplicate board messages, so every strand artifact comes from the trustee it names.
- **sequent-core and windmill** validate ballots before they are stored or tallied, and encrypt only to the published election key.
- **sequent-core** validates decoded ballot bytes. Its panic on an out-of-range length byte (`../sequent-core/src/ballot_codec/vec.rs` `decode_array_to_vec`): Open on release/10.0 (fix in sequentech/step#3522).
- **braid, windmill, electoral-log and sequent-core** keep secrets out of logs and tracing spans, and manage the keys and ciphertexts they store.
- **The host** provides a working CSPRNG: the OS RNG on servers, `crypto.getRandomValues` in the browser.
- **Production builds** use the Ristretto backend and the default feature set. Operators who enable `fips_core`/`fips_full` configure the OpenSSL FIPS provider.

## Review focus

1. Shuffle proof verification, the only check that a mixing trustee did not alter or link ballots.
2. Handling of malformed input by verifiers and decoders.
3. The feature-selected backends.
4. How sequent-core, windmill and braid call the proof and encryption APIs.
5. Signature verification.
6. The lifecycle of secrets in memory and in logs.
7. Constant-time handling of secret data.
8. Threshold arithmetic and Lagrange interpolation (`src/threshold.rs`).
9. Build configuration and dependency management of shipped artifacts.
10. The `certs` feature, before any crate enables it.
