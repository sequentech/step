<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# Protocol description ↔ implementation alignment

An evaluation of how closely this repository's implementation (`crates/braid`,
`crates/vsc`) matches `PROTOCOL.md` — the mathematical description of the voting
protocol — at the level of exact formulas, hash-transcript component lists, and
byte conventions. **Evaluation only**: this document records findings and
proposes scheduling; it changes no code.

Scope: the sections of `PROTOCOL.md` that bind this repository — §2
(preliminaries), §3 (primitives), §4 (DKG), §5.5 (tally input), §6 (mixing), §7
(decryption), §9.2 (verification). The voting client, ballot box and tracker
sections (§5.1–5.4, §8, §9.1) bind other platform components and are out of
scope here, except where their definitions reach the trustees (noted below).

Each finding carries a verdict:

- **A — aligned**: verified equivalent at the mathematical level.
- **B — planned**: a known gap already scheduled as 0.6.3 work.
- **C — new gap**: discovered by this evaluation; needs scheduling.
- **D — description fix**: the implementation is reasonable and the *document*
  should be made precise to match it.
- **E — out of braid scope**: binds platform components outside this repo.

## Summary

The implementation matches the description remarkably closely — all shuffle and
decryption **algebra** (commitments, responses, all verifier equations), the
DKG derivations, the hash/transcript machinery, domain labels, encodings and
completion rules are equation-for-equation aligned. The three known 0.6.3 tasks
are confirmed as exactly the planned gaps. The evaluation found **one
significant new gap**: the shuffle proof's Fiat-Shamir challenges bind less
data than the description specifies — in particular the prover-chosen
permutation commitments are not hashed into either challenge (C1 below). Four
small description-precision fixes round out the list.

## A — verified aligned

| # | Item | Description | Implementation | Notes |
|---|---|---|---|---|
| A1 | Group | §2.1 ristretto255, prime order, canonical 32-byte encodings | `vsc::groups::ristretto255` | The code is group-generic (a P-256 instantiation exists for tests); the description documents the deployed instantiation |
| A2 | Element encoding | §2.2: 30-byte payload at bytes 1..30, byte 0 = 2i, byte 31 = j, first valid encoding; scalar → 2 elements | `encode_30_bytes`, `encode_scalar` | Trial order nit → D3 |
| A3 | Transcript hashing | §2.3: SHA3-512 over value‖tag pairs; H2S = 64-byte wide reduction; H2G = ristretto one-way map; vector challenges seed-then-counter with 64-bit big-endian indices | `utils::hash::update_hasher`, `Scalar::from_hash`, `RistrettoPoint::from_hash` | Exact match, including the value-then-tag order |
| A4 | Domain labels | §2.4: `label(P) = cfg ‖ len(P) ‖ P`, `ctx(P, input) = label(P) ‖ H(input)`; purposes `"shuffle"`, `"shuffle_generators"`, `"decryption proof"` | `braid::trustee::domain_label`, `shuffle_proof_label`, `shuffle_generators_seed` | Purpose strings verbatim; len endianness nit → D2 |
| A5 | Independent generators | §2.5: `h_i = H2G(seed, i)`, seed = `ctx("shuffle_generators", input)` | `ind_generators` | Tag string nit → D1 |
| A6 | Randomness | §2.6: OS CSPRNG; Fisher-Yates permutations | `Context::get_rng` (OS-seeded), `Permutation::shuffle` (classic downward Fisher-Yates) | |
| A7 | ElGamal, width-W | §3.1–3.2: componentwise with independent randomizers | `elgamal::Ciphertext<C, W>` | |
| A8 | Schnorr proof | §3.3: `v = H2S(b, Y, A, ctx)`, `k = v·x + a`, check `b^k = Y^v·A` | `zkp::schnorr`, tags `g/public_y/big_a/schnorr_context` | In vsc; braid's DKG use of it is task 1 (B1) |
| A9 | DLEQ proof | §3.4: `v = H2S(b₀, b₁, Y₀, Y₁, A₀, A₁, ctx)` | `zkp::dlogeq`, tags in that order | |
| A10 | Plaintext-equality proof | §3.5: `v = H2S(g, y, z, u_b, v_b, u_a, A, ctx)`; binds `v_b` | `zkp::pleq`, tags in that order | |
| A11 | Naor-Yung | §3.6: `z = H2G(ctx_enc, "naor_yung_public_key_a")` — no trapdoor; verify-before-strip | `naoryung::KeyPair::augment` (tag string verbatim), `strip` verifies first | In vsc; braid's use is task 2 (B2) |
| A12 | Signatures | §3.7: Ed25519; signed statement = head (context hashes) + H(body) | `utils::signatures::Ed25519`; braid `wire.rs` statement layout | |
| A13 | Configuration | §4.1 field list; detect-and-halt semantics | `braid::messages::artifact::Configuration`; datalog error relations | |
| A14 | Bulletin-board rules | §4.2: signature checks, one-slot/equivocation-halt, digest persistence + anti-rewrite on reconnect | `BoardClient` (§6.2/§6.3 of the braid spec) | Model-checked (see `STATERIGHT.md`) |
| A15 | DKG round 1 | §4.3: degree-(t−1) polynomial; Feldman checking values; per-recipient ElGamal-encrypted shares via 2-element scalar encoding | `dkgd::Dealer`, `braid::trustee::dkg::compute_shares` (`encrypt_scalar` to `share_encryption_keys[i]`) | Checking-value *proofs* are task 1 (B1) |
| A16 | DKG round 2 | §4.3: share check `g^s = ∏ A^{iʲ}`; derive `x_i`, `y = ∏A_{d,0}`, `vk_m`; post `(y, vk₁..vkₙ)`; all-identical completion | `Recipient::verify_share`/`from_shares`; braid `compute_public_key` posts joint pk **and** all verification keys; datalog `pk mismatch` halt rule | |
| A17 | Mix chain rules | §6.5: counter-sign by all t before the next mix; own mix counts as signature; consecutive positions; length exactly t; halts on violations | datalog `mix.rs` rules and error relations | Model-checked |
| A18 | Shuffle algebra | §6.2–6.4: permutation commitments, bridging commitments (B₀ = h₁), all proof commitments, all responses, verifier equations V1–V5, `e′ = π⁻¹(e)` | `zkp::shuffle` prover and verifier | Equation-for-equation match; the challenge *transcripts* are C1 |
| A19 | Threshold decryption | §7: factors `u^{x_i}`; batched proof with `seed = H(vk, u-list, factor-list, ctx)`, `e_j = H2S(seed, j)`, `A = ∏u^e`, `B = ∏f^e`, DLEQ; position from the signed envelope, never from prover data; Lagrange `λ_i = ∏ k/(k−i)`; `m = v·F⁻¹` | `Recipient::partial_decrypt`, `batching_exponents`, `lagrange`, `combine` | Exact match including seed component order |
| A20 | Decryption completion | §7.2: contributions from t distinct quorum members; all post identical plaintext lists, halt otherwise; plaintexts must cite the chain end | datalog: per-sender decryption slots; `plaintexts mismatch`, `unexpected input ciphertexts` error rules | |

## B — planned 0.6.3 work, confirmed

| # | Item | Description | State | Task |
|---|---|---|---|---|
| B1 | DKG checking-value Schnorr proofs | §4.3 round 1 step 2 (prove), round 2 step 1 (verify); motivated by [BNP24] | vsc has the full machinery (`CheckingValue { value, proof }`, `get_checking_values_proofs`); braid's `Shares` artifact carries raw checking values, nothing verifies proofs | 0.6.3 task 1 |
| B2 | Naor-Yung input ballots | §3.6, §5.5: tally input is NY ciphertexts; every trustee runs `NYVerify` and strips to ElGamal before mixing | vsc NY is complete (A11); braid's ballots are plain ElGamal today | 0.6.3 task 2 |
| B3 | Per-tally domain identifier | §2.4 (marked TO BE CONFIRMED) | `domain_label` is keyed on `cfg` only | 0.6.3 task 2 rider |

## C — new gaps

### C1 — Shuffle Fiat-Shamir challenges bind less than the description (significant)

`PROTOCOL.md` §6.3 specifies strong Fiat-Shamir for the shuffle proof:

- batching seed: `seed = H(g, h, u, y, w, w′, ctx)` — binding the generators,
  the **permutation commitments `u`**, the key, and both ciphertext lists;
- challenge: `v = H2S(seed, B, A′, B′, C′, D′, F′, ctx)` — chaining the seed
  (and everything it binds) into the second challenge.

The implementation (`zkp::shuffle::NativeChallenges`) hashes:

- batching seed: `(pk, w, w′, ctx)` — the generators and permutation
  commitments are passed in but **ignored**;
- challenge `v`: `(pk, B, A′, B′, C′, D′, F′, ctx)` — the seed is not chained,
  so `v` binds neither `u` nor the ciphertext lists.

Assessment. Omitting `g` (a fixed constant) and `h` (a deterministic function
of `ctx`, which *is* bound) is benign. Omitting **`u` from both challenges is
not**: `u` is prover-chosen, and the Terelius-Wikström soundness argument
requires the permutation commitment to be fixed *before* the batching vector
`e` is drawn — that ordering is exactly what hashing `u` into the seed
enforces in the non-interactive setting. As implemented, a prover can compute
`e` first and choose `u` afterwards, which voids the commit-then-challenge
structure the proof's extraction argument rests on (the class of weakness
described in [BPW12], which `PROTOCOL.md` Appendix A cites as the reason for
hashing "the complete preceding transcript"). No concrete forgery is exhibited
here; the finding is a deviation from the specified strong-FS transcripts, and
that alone warrants alignment — the description is the sound design.

Provenance: this is a **known, tracked deficiency**, not a new discovery — the
tag constants carry `#[crate::warning("Challenge inputs are incomplete. …")]`
in the current tree, and the protocol-description work left implementation
notes with drop-in target code (this evaluation reached the same conclusion
independently before reading them). The notes' essentials, recorded here so
they survive the notes' deletion:

- Batching seed transcript becomes `(g, h, u, pk, w, w′, ctx)` — the ignored
  `generators`/`pedersen_commitments` parameters are already passed in — with
  the tag array extended to match; the seed is then chained into `v` in place
  of `pk` (binding the seed transitively binds the statement *and* `e`; this
  is Verificatum's structure, `v = RO(ρ ‖ seed ‖ τ)`).
- *Ordering dependency*: `challenge` needs the seed `batching_challenges`
  produced, and the trait doesn't hand it over. Two options were considered:
  cache it (v2v's `RefCell` pattern — makes the challenge object single-use
  per proof and not `Sync`, so call sites need per-call instances) or extend
  the trait so the dependency is explicit in the signatures.
  **Decided 2026-08-27: extend the trait** — `batching_challenges` returns
  the seed alongside `e`, and `challenge` takes it as a parameter. Both
  prover and verifier already invoke the two methods in the required order,
  so callers just pass along a value they now receive. `VmnChallenges`
  adjusts to the new signatures and can drop its internal `RefCell` cache.
- *Doc-vs-code choice*: retaining `pk` in `v`'s transcript alongside the seed
  would be a harmless superset of §6.3's `v = H2S(seed, B, A′, B′, C′, D′,
  F′, ctx)`. **Decided 2026-08-27: follow §6.3 verbatim** — the seed replaces
  `pk` as the first component (the seed already binds it); one canonical
  form, no divergence for future verifier implementors to reconcile.
- Working reference: `v2v::challenges::VmnChallenges` implements the
  seed-chained structure through the same trait.

Remediation is therefore contained and pre-designed: the `ShuffleChallenges`
trait signatures, its two implementors, and the two call sites (prover and
verifier both derive through it); no wire-format change; invalidates any
previously recorded transcripts (none shipped).
**Decided 2026-08-27: first item of 0.6.3** — small, security-relevant,
independent of the other tasks. **Implemented 2026-08-27**: trait extended
(seed returned by `batching_challenges`, passed into `challenge`), both
transcripts completed per §6.3, `VmnChallenges` stateless; the
`#[crate::warning]`s are gone, and the vsc, v2v and braid suites pass.

### C2 — The encryption context `ctx_enc` must be defined before task 2 completes

The NY auxiliary key is `z = H2G(ctx_enc, …)` (§3.6/§5.2), and trustees need
`ctx_enc` to derive `z` when they verify-and-strip the tally input (§5.5). The
component list of `ctx_enc` is a `PROTOCOL.md` TO-BE-CONFIRMED item and is
co-owned with the platform (the voting client encrypts under it). Task 2
cannot finish without this definition — schedule the decision as part of
task 2's design.

### C3 — Integer power in share verification wraps for large committees (minor)

`Recipient::vk_factor` computes the Shamir evaluation exponent `iʲ` in `u32`
before converting to a scalar; for large committee sizes this silently wraps in
release builds. Harmless at braid's current `MAX_TRUSTEES = 8` (max 8⁷);
convert to scalar arithmetic if the committee bound ever grows. No action for
0.6.3 beyond this note.

## D — description precision fixes (edits to `PROTOCOL.md`)

| # | Where | Fix |
|---|---|---|
| D1 | §2.5 | The generators tag is group-qualified: `"independent_generators_ristretto"`, not `"independent_generators"` |
| D2 | §2.4 | State the byte encoding of `len(P)` in the domain label. **Reversed 2026-08-27 from a doc-only fix to a code change**: the little-endian encoding in `domain_label` (`trustee/mod.rs`) is the *only* little-endian integer entering any hash transcript in braid/vsc — VSer lengths/integers and both hash counters are all big-endian — an inherited anomaly, not a convention. Decided: normalize the code to big-endian, riding along with C1 (which already invalidates all transcripts; none shipped), so the documented rule becomes uniform: *every 64-bit integer entering a hash transcript is big-endian*. §2.4 gets that sentence when the code lands. Verificatum interop is unaffected: `VmnChallenges` ignores the braid `context` parameter entirely and salts with VMN's own `rho` prefix (`v2v/src/challenges.rs`, `session.rs`) — the two derivations share no bytes |
| D3 | §2.2 | Specify the encode trial order: the implementation searches `j` (byte 31) outer, `i` (byte 0) inner |
| D4 | §1.5/§2.1 (optional) | Note the implementation is group-generic and ristretto255 is the deployed instantiation |

## E — out of braid scope (platform components)

Ballot encoding (§5.1), client-side encryption (§5.2 — except `ctx_enc`, C2),
the Benaloh challenge (§5.3), trackers and the ballot locator (§5.4 and its
TO-BE-CONFIRMED items), result rules (§8), voter-facing verification (§9.1),
and export signing. The independent election verifier (§9.2) is scheduled
separately (0.6.4+, the `v2v` crate).

## Proposed effect on the 0.6.3 plan

1. **Add C1 (shuffle Fiat-Shamir alignment) to 0.6.3**, ordered first or
   alongside task 1 — small, contained, and the only security-relevant code
   divergence found.
2. Tasks 1 and 2 proceed as planned (this evaluation confirms their scope);
   task 2 absorbs C2 (define `ctx_enc`) and B3 (per-tally identifier).
3. Apply the D fixes to `PROTOCOL.md` — cheap, can be done immediately.
4. C3 is a note, not a task.
5. The VSer canonicality audit remains the capstone, unaffected.

## How this was evaluated

By reading `PROTOCOL.md` against the source, at the level of formulas and
transcript component lists: `vsc::zkp::{schnorr, dlogeq, pleq, shuffle}`,
`vsc::cryptosystem::{elgamal, naoryung}`, `vsc::dkgd::{dealer, recipient}`,
`vsc::groups::ristretto255::{group, element, scalar}`, `vsc::utils::hash`,
and `braid::trustee::{mod, dkg, mix, decrypt}`, `braid::datalog::{dkg, mix,
decrypt}`, `braid::messages::wire`. Board-client behavior (A14, A17) is
additionally covered by the model-checking harnesses (`STATERIGHT.md`).
