<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Verificatum interoperability

braid and [Verificatum](https://www.verificatum.org) implement the same mix-net:
El Gamal with re-encryption, the Terelius–Wikström proof of shuffle, and
threshold decryption from a Shamir sharing. This document records what that
buys us, what has been built on it, and what a reader working in this area
needs to know that is not in either system's documentation.

Section references of the form "VMNV §N" point into
`crates/verificatum/papers/vmnv-3.1.0.md`, the stand-alone verifier
specification. The Verificatum source and jars are under
`crates/verificatum/`, which is assembled by hand and not part of this
repository — see [`crates/v2v/TESTING.md`](crates/v2v/TESTING.md).

---

## Status

**Interop works in both directions, with unmodified Verificatum.**

| direction | what it means | where |
| --- | --- | --- |
| Verificatum verifies ours | `vmnv -mix` accepts a complete braid session — DKG, shuffle chain, threshold decryption | `crates/v2v/tests/they_verify_ours.rs` |
| we verify Verificatum's | our verifier accepts sessions VMN's own prover produced | `crates/v2v/tests/we_verify_theirs.rs` |

Both are exercised over a range of session shapes rather than one. Emitting
sweeps party counts 1–4 with every threshold within them — ten shuffling shapes
and seven mixing ones, at width 2, with the active set spread so the
non-participant path is covered. Ingesting covers four generated sessions,
including one at width 3 and one where a party takes no part.

Nothing is checked in. Every test runs whatever Verificatum is installed and
compares against what *that* produced, because a pinned corpus keeps passing
against a future VMN that changed a derivation — it would succeed exactly when
it should have failed.

All of it lives in **`crates/v2v`**. braid itself has no dependency on any of
it, and nothing depends on `v2v`.

The two braid improvements this work produced — completing the P-256 backend
and replacing the per-ciphertext decryption proofs with a batched one — were
separated onto their own branch, since they stand on their own merits.

### emit, reprove, convert, export

How much of a production system survives into an interop path is decided by
what the *other* side's verifier is rigid about, in decreasing order of what
has to be rebuilt:

| | the verifier demands | what you keep | what you write |
|---|---|---|---|
| **emit** | its own statement *and* transcript | nothing | a second prover |
| **reprove** | its own transcript; the statement agrees | the production prover | a foreign transcript, run after the fact |
| **convert** | *the same as reprove* | the production **proofs** | the same transcript, run **in production**, plus a serializer |
| **export** | nothing — it parses the foreign format | the proofs themselves | nothing at proving time |

**Convert and reprove make identical demands of the verifier.** They differ in
where the foreign transcript runs: in a side path, so a second proof is
produced from the witness — or in production, so the proofs are born
conforming and only their encoding has to change. That is why convert is
reachable without any upstream agreement wherever reprove already is, and it
is the reason it matters: converting needs no witness and no proving, so
**anyone holding the published proofs can do it**, not only the party that
made them.

Export is not hypothetical, and its cost is known, because **`v2v verify` is
export, inbound**: proofs VMN's own prover produced pass through unmodified.
`v2v::wire` is the foreign transcript layer that makes that possible, and it is
most of the crate. What makes the outbound direction hard is not that the
arrangement is unproven but that we do not control `vmnv`.

Which arrangement applies is a property of an *artifact*, not of a session.
Today:

- the shuffle chain is **reprove** — braid's own `Shuffler` with
  `VmnChallenges` plugged in through the `ShuffleChallenges` trait;
- the decryption is **emit** — `v2v::decrypt::prove_decryption` is a separate
  implementation, because the statement itself differs (VMN's is joint over all
  `k` parties, braid's is per party).

So one `v2v generate --kind mixing` run uses two of the four at once. Convert
is not in use, and export is used only inbound, as above.

Whether the input is **real or synthetic is an independent axis**, and both our
paths currently run on synthetic input. Reproving the shuffle over a real
session needs no new mechanism — only the permutation and re-encryption
randomizers, which the mixer holds — and is simply not built. Emitting over a
real session is a different matter: a statement shape cannot be retrofitted
onto data already produced, so it would take a fresh proving protocol among the
trustees after the fact. That, and not the challenge derivation, is why
decryption is the half that is stuck.

### What is not covered

- **P-256 at widths 1–3 only.** Inherent: `vmnv` supports no group braid also
  supports beyond the standard curves. **braid's native Ristretto255 path
  cannot be verified this way at all**, so this is an assurance tool for a
  specific configuration, not for production as currently deployed.
- **Version 3.1.0 only.** `vmnv` rejects unless `version` matches exactly
  (VMNV §9.3 step 2b), so any VMN upgrade is a re-validation event.
- **Input is synthetic.** `v2v generate` runs our cryptography on generated
  ciphertexts; it does not take a real braid session. Reproving a real shuffle
  is available and unbuilt; the decryption cannot follow — see
  [What can be taken from a real session](#what-can-be-taken-from-a-real-session).
- **The live protocol is not wired in.** The tests drive the DKG, shuffle and
  decryption directly rather than through `Trustee::step` and the board.

---

## Using the tools

`crates/v2v` is a library and a binary with two subcommands:

```text
v2v generate [--kind mixing|shuffling] -k -t -w -n [--active 1,3] DIR
v2v verify PROTINFO DIR
```

`generate` writes a session in Verificatum's format for `vmnv` to check.
`verify` checks a session Verificatum produced, needing no JVM.

Full usage, including how to obtain a Verificatum session to point `verify` at,
is in [`crates/v2v/README.md`](crates/v2v/README.md). How the tests are run,
and what they cover, is in [`crates/v2v/TESTING.md`](crates/v2v/TESTING.md).

**Never treat `vmnv`'s exit code as its verdict on a shuffling proof** — see
[the defect below](#vmnv-exits-0-on-shuffling-proofs-it-rejected). Anything
automating this must ask through a predicate that also reads the output;
`vmnv_accepts()` in `they_verify_ours.rs` is that predicate.

---

## How the two systems correspond

### The shuffle proof is the same object

Not merely similar — term for term, with the same commitments in the same order
and the same five verification equations.

| Verificatum (VMNV §8.3) | braid (`vsc/src/zkp/shuffle.rs`) |
|---|---|
| `μ` — permutation commitment, array `u` | `ShuffleCommitments.u_n` |
| `τ^pos = node(B, A', B', C', D', F')` | `ShuffleCommitments { big_b_n, big_a_prime, big_b_prime_n, big_c_prime, big_d_prime, big_f_prime }` — same order |
| `σ^pos = node(k_A, k_B, k_C, k_D, k_E, k_F)` | `Responses { k_a, k_b_n, k_c, k_d, k_e_n, k_f }` — same order |
| `C = ∏u_i / ∏h_i` | `big_c = u_n_fold · h_n_fold⁻¹` |
| `D = B_{N-1}·h_0^{−∏e_i}` | `big_d = big_b_last · (h[0]^{∏e_i})⁻¹` |

El Gamal matches directly too: braid's ciphertexts are `[[C::Element; W]; 2]`
over a single public-key element, which is Verificatum's **κ = 1, ω = W**.

### The key is the same object viewed differently

Verificatum models the joint key as a Shamir polynomial in the exponent
`Γ = (Γ_0, …, Γ_{λ−1})`, with joint public key `y = Γ_0` and per-party key
`y_l = ∏_s Γ_s^{l^s}` (VMNV §2.1), read from
`proofs/PolynomialInExponent.bt`.

braid runs a Joint-Feldman/Pedersen DKG, where each dealer publishes
commitments to its own polynomial (`Shares.commitments`). The joint polynomial
is the sum of the per-dealer polynomials, so **Γ is the component-wise product
of the dealers' commitment vectors**, `Γ_s = ∏_d C_{d,s}` — derived by
`v2v::decrypt::polynomial_in_exponent` and cross-checked against the DKG's own
joint key.

### Transcript (challenges) must be produced VMN's way at proving time

Every challenge in both systems is derived by hashing a transcript, and the
verifier *recomputes* it. So the transcripts must agree byte for byte.
A proof's challenges are fixed when the proof is created. Translating braid's
serialization afterwards changes the bytes `vmnv` will hash, so the challenges
it recomputes are not the ones braid used, and every equation fails. **The
transcript has to be produced VMN's way at proving time**.

This is why the interop layer reimplements Verificatum's serialization and
random oracles outright, in `v2v::wire`, rather than adapting braid's. There is
no part of the transcript the two systems share:

| | Verificatum | braid |
|---|---|---|
| Serialization | byte trees (§4) | VSer |
| Hash | SHA-256/384/512; §5.1 calls SHA-3 "future" | SHA3-512 |
| PRG | `r_i = H(s ‖ bytes₄(i))` (§5.2) | hash and counter, different framing |
| Random oracle | output length prefixed, expanded, masked (§5.3) | `hash_to_scalar` with domain-separation tags |
| Batching exponents `e_i` | `n_e`-bit integers, not reduced mod `q` | full-width scalars |
| Challenge `v` | `n_v`-bit integer | full-width scalar |
| Global prefix | `ρ` over version, sid, the three bit lengths, PRG, group and hash (§9.3 step 4) | none — nothing binds the protocol parameters |
| Independent generators | quadratic-residue walk over derived x-coordinates (§6.8) | hash-to-curve |

The bit-length rows are subtler than they look, and it is worth being precise
about which half matters. Reducing into the scalar field is *not* itself a
divergence: exponents act on a group of prime order `q`, so `g^t = g^(t mod q)`.
Verificatum carries an unreduced `BigInteger` and we reduce only because
`P256Scalar` is a field element — the group element is the same either way.

What has to match exactly is the **integer**: the PRG stream it is taken from,
and the truncation to `n_e` bits (`challenges::scalar_from_bits`). braid's
`hash_to_scalar` fails on that count and not on the reduction — a different
hash, a different framing, and a full-width output where VMN wants a
fixed-bit-length one.

`wire` is therefore built to depend on **nothing from `vsc`** — it is byte
trees, hashes and integers, with no group or scalar types in its signatures.
That is what lets it be checked directly against `vmnv -t`, which dumps the
intermediate values VMN computed for a session: `der.rho`, `bas.h`, the shuffle
and decryption seeds and challenges. Each layer is confirmed against
Verificatum's own numbers before any braid cryptography is involved, so a
mismatch is localised to the transcript rather than discovered later as a proof
that will not verify.

---

## Findings

Everything below was established by reading Verificatum's Java or by running it
at a shape its own documentation does not exercise. A reader building on this
work should expect more of them.

### `vmnv` exits 0 on shuffling proofs it rejected

Replace a mixer's output list with its input, so the proof no longer holds.
`vmnv` **detects** this and, under `-v`, says so — then exits **0**.
Verificatum's own proofs behave identically, so this is `vmnv`, not our emitter.

`MixNetElGamalVerifyFiatShamirSession` reaches the right conclusion and routes
it to the wrong handler:

```java
if (validProofs < v.threshold) {
    v.failInfo("Too few proofs are valid! (" + validProofs + ")");
}
```

`failInfo` only prints, and only when verbose. Its sibling `failStop` prints a
banner and throws. `validProofs < threshold` is exactly VMNV §2.3's reject
condition — *"If less than λ proofs are valid, then reject"* — so the condition
is evaluated correctly and then not enforced.

| Invocation | Result on a proof `vmnv` internally rejected |
|---|---|
| `-shuffle` with `-v` | exits 0, prints the failure |
| `-shuffle` without `-v` | **exits 0 with no output at all** |
| `-mix -nodec` | exits 0 |
| `-mix` (full) | exits 1 — rejected |

The full mixing path is safe only *incidentally*: the downstream plaintext
comparison catches it, not the shuffle check. `-mix -nodec` is affected exactly
like `-shuffle` and is not a safe way to check the mixing phase alone.

This matters because a shuffling session is VMN's documented mode for
re-randomising without decrypting (§2.4), and §10.1 designates the exit code as
*the* accept/reject signal. Accepting a session in which no mixing occurred
means accepting a mix-net that provided no privacy, silently, with a zero exit.

Pinned by `vmnv_exit_code_alone_is_not_sufficient` and
`vmnv_is_silent_about_a_failed_shuffle`, which fail if it is fixed upstream.
(Separately, `vmnv` exits 1 on rejection, not the `-1`/255 §10.1 specifies.)

### A mixer is verified only if its proof file exists

Mixer slots are numbered by **party index**, not sequentially. A party that
takes no part leaves a gap: with active set `{1,3}` the directory holds
`PermutationCommitment01` and `03` but no `02`, and `activethreshold` is `3` —
the highest active index, *not* the count. A verifier cannot iterate
`1..=activethreshold` and assume a proof for each.

How `vmnv` decides which to skip:

```java
public boolean getPoSCActive(final int l) {
    final File file = PermutationCommitment.PCfile(proofs, l);
    return file.exists();
}
```

The whole body of the loop sits inside that condition, so a slot with no proof
file is passed over without comment. Remove a mixer's files and `vmnv` verifies
a shorter chain and reports success.

Not a soundness break — the remaining shuffles are still checked — but the
*privacy* claim weakens, since a session presented as an `n`-mixer mix may have
had its output produced by fewer, and nothing in the transcript binds the
number of mixers. Skipping absent slots is necessary, or valid VMN proofs get
rejected; so we match the behaviour, count what was actually verified, and
require it to meet the threshold.

### The specification and the implementation disagree about where α goes

The most expensive finding here, and both sides of the disagreement are
Verificatum's own: the `vmnv-3.1.0` document, and the Java in
`DistrElGamalSessionBasic`. That one class serves *both* roles — VMN's prover
calls it to write `DecrFactReply<l>.bt`, and `vmnv` calls it to check them —
which is why the discrepancy is invisible from inside Verificatum. Only a third
implementation, written from the document, finds them wrong.

**They agree on the verification equations**, which contain no α at all:

```text
Γ_0^{−v} · y' = g^{k_x}          B^v · B' = A^{k_x}
```

By the time `B` is formed the α has cancelled, since
`B` batches `f_i = ∏_l f_{l,i}^{α c_l} = u_i^{−x}`.

**They disagree one level down**, on how the combined values are built:

```text
specification    y' = ∏ (y'_l)^{c_l}       B' = ∏ (B'_l)^{c_l}       k_x = Σ c_l · k_{x,l}
implementation   y' = ∏ (y'_l)^{α c_l}     B' = ∏ (B'_l)^{α c_l}     k_x = Σ α c_l · k_{x,l}
```

(The *factors* are combined by `α c_l` in both; that is not in dispute.) Since
the combined `k_x` must come out as `R − v·x` either way, the two rules demand
different replies from the prover: `k_{x,l} = r_l − v·x_l` for the
specification, `r_l − v·(x_l/α)` for the implementation.

Both are sound. The only constraint the equations impose is `Σ_l γ_l · w_l = x`:

| | `γ_l` | `w_l` |
|---|---|---|
| specification | `c_l` | `x_l` |
| implementation | `α c_l` | `x_l/α` |

Only the **factor** combination is genuinely forced to `α c_l` — that is the
unique exponent set undoing the `1/α` the factors were computed with, and the
whole reason α exists. The proof pieces are under no such constraint;
Verificatum reused the coefficient array it already had, and *that reuse*, not
any requirement of the scheme, is what forces its witness to be scaled.

The two coincide only when `α = 1`, i.e. `k = 1` — which is why a single-party
session cannot tell them apart. braid follows the implementation
(`v2v::decrypt::prove_decryption` takes `x_l/α`).

**Why it matters beyond us.** A third implementation written strictly from VMNV
§8.6 would reject every genuine Verificatum mixing proof with more than one
party. `vmnv-3.1.0` exists precisely so that stand-alone verifiers can be
written from it, and on this point it is not sufficient for that.

### The DKG is out of scope for the verifier

VMNV says so outright (§2.2): *"The details of the verifiable secret sharing
scheme are not important in this document."*

The only thing `vmnv` does with the DKG's output is Algorithm 24 — read
`FullPublicKey.bt`, read `PolynomialInExponent.bt`, **reject if `Γ_0 ≠ y`**.
That is a consistency check between two files the prover wrote. It does not
establish that a distributed key generation happened.

`Γ_1 … Γ_{λ−1}` are never checked against anything, and never reach a
verification equation — only `Γ_0` does. They enter the transcript solely
through the decryption seed, which binds them without verifying them.
(`v2v::emit` fills them with random group elements for shuffling sessions, and
`vmnv` accepts.)

Compounding this, VMN's decryption proof is batched over *parties* as well as
ciphertexts, so there is no per-party statement to check `y_l` against even if
Γ were verified. The aggregate is sound for the outcome — both equations bind
the combined factor, so a passing transcript implies correct plaintexts — but
it cannot attribute a failure to a party, and `Δ` is read from
`CorrectIndices.bt` on the prover's word.

**braid's own model is stronger here**, and the difference is what gets
batched: braid batches over ciphertexts only, keeping one proof per party.
`Recipient::verification_key` derives `y_l` from the published dealer
commitments, board messages are signature-bound to their sender, and
`AttributedDecryption` pairs a contribution with the key belonging to *that*
sender — while `PartialDecryption` deliberately carries no position, so a
trustee cannot claim another's. A party is therefore pinned to a key it did not
supply, which is what makes a per-party proof meaningful.

### What can be taken from a real session

`v2v generate` runs on synthetic input, but the two halves of a real session
are not equally out of reach. **The shuffle can be reproved; the decryption
cannot be reproved or exported.**

**The shuffle can**, because a shuffle proof needs only one party's own secrets.
Not by translating braid's existing proof — that is the transcript problem
above, and it is impossible — but by producing a *second* proof, in VMN's
transcript, over the real lists. The mixer holds the witness the statement is
about, so the same real shuffle can be proved twice in two formats.

The practical condition is that the witness is still available: re-proving
needs the permutation and the re-encryption randomizers, not just the published
lists. So this has to happen at shuffle time, or the mixer has to retain them
deliberately.

**The decryption cannot**, because VMN's transcript is joint over **all `k`**
parties' factors. The batching seed commits to every party's factor array;
every party's commitment is then hashed into a single challenge; only then can
any party reply. No trustee can produce its piece alone, so this is not a
serialization step but a fresh two-round proving protocol among the trustees,
run after the factors already exist.

braid publishes factors and proof together in one round, and keeps proofs per
party so a failure names its author — which the board needs in order to make
progress. That is a deliberate difference, not a gap, and it is why decryption
sits in **emit** while the shuffle sits in **reprove**. Only moving it to
**export** — a verifier that accepts braid's statement — removes the obstacle
rather than working around it; see [Outstanding](#outstandingfuture-work).

### Smaller things that will cost time

- **Δ is the first λ true entries** of `CorrectIndices.bt`, not an arbitrary
  subset — the loops in `modifiedLagrangeCoefficients` stop at `threshold`.
  Marking more than λ silently selects a prefix.
- **A non-participant's placeholder values are load-bearing.**
  `DecryptionFactors<l>.bt` must be an all-identity array of the right shape —
  the file cannot be omitted, since `readArray` calls `failStop` on a missing
  one. `DecrFactCommitment<l>.bt` and `DecrFactReply<l>.bt` must be the
  identity and zero respectively, because the commitment container is built
  over **all `k`** parties and hashed into the decryption challenge: a
  different placeholder moves `v` and breaks the *participants'* proofs.
- **`vmnv` refuses to start without a random source**, even though verification
  consumes none.
- **VMN's prover-side tooling is Unix-bound** — `/dev/urandom` and a
  lowercase-only hostname regex, both evaluated before argument parsing. Only
  the verifier runs natively on Windows.
- **Modified Lagrange coefficients may be negative.** The integer is reduced to
  the representative of smallest absolute value, choosing between `res` and
  `res − q`.
- **P-256 coordinates occupy 33 bytes, not 32.** The integer encoding is signed
  (§6.1) and `p` has its top bit set, so every coordinate carries a leading
  `0x00`. `FullPublicKey.bt` is 167 bytes, not the 163 a 32-byte assumption
  predicts.
- **Arrays of product-group elements transpose.** An array of ω-wide
  ciphertexts is stored as ω arrays, not N tuples (§6.6).
- **The §8.6 equations in the specification are garbled by OCR.** Take them
  from `DistrElGamalSessionBasic` instead.

### One pattern worth internalising

Three of the findings above are the same shape: **a condition evaluated, its
conclusion not enforced.**

- `validProofs < threshold` routed to a print-only handler;
- a mixer omitted because its file is missing;
- Algorithm 24's `Γ` check, which `vmnv -shuffle` does not perform — the same
  instinct in the specification rather than the code.

A verifier built on this work should treat that as the failure mode to look for
first, and should never inherit a silence: match VMN where a valid proof
depends on it, and report everything else.

---

## Outstanding/future work

- **Report the `vmnv` findings upstream.** The exit-code defect, the
  file-existence mixer skip, and the α discrepancy between VMNV §8.6 and
  `DistrElGamalSessionBasic`. None has been reported yet.
- **Get to convert, so real proofs are what gets verified.** The assurance this
  work delivers today is about braid's *cryptography*: an independently written
  verifier accepts sessions our code produced. It says nothing about any
  particular election, because those artifacts are never shown to `vmnv`.
  **Convert** changes the object of the claim from the implementation to the
  run, and that is a different statement rather than a stronger version of the
  same one. It is the outcome worth aiming at.

  The two halves are shaped very differently.

  **The shuffle needs nothing from Verificatum.** The statement already agrees
  term for term, so a production mixer using `VmnChallenges` and
  `vmn_generators` against a fixed parameter identity would emit proofs
  `vmnv` accepts once serialized — which is what the tests already demonstrate,
  the only difference being that their input is synthetic. This is a unilateral
  decision, available now.

  **The decryption needs `vmnv` to accept a per-party, one-round statement.**
  That is the minimum ask, *given that braid will not adopt a three-round
  decryption*: the zero-ask option is braid taking VMN's joint protocol
  wholesale, which is rejected on its merits, since per-party proofs are what
  let a failure name its author and let the board make progress. Beyond the
  transcript, four things would change on our side: `partial_decrypt` has no
  strategy parameter, so the seam `ShuffleChallenges` provides for the shuffle
  has to be built; that seam needs *two* derivations, the batching exponents
  and the challenge; VMN's decryption seed commits to `Γ` and the mixed
  ciphertext list, which braid's proof does not bind today, so what the proof
  asserts changes and not merely how it is hashed; and byte-tree encoding moves
  into the proving path. The proof objects themselves already correspond field
  for field — `DlogEqProof { big_a_0, big_a_1, k }` against
  `BatchedDecryptionProof { y_prime, b_prime, k_x }` — so there is nothing
  structural to map.

  **What it costs.** braid's production transcript *becomes* Verificatum's:
  their serialization, their generator derivation, their bit lengths, and
  **SHA-256 where braid uses SHA3-512 today**. That last one is sound at the
  security level but would arrive as a side effect of an interop decision, and
  should be signed off on its own terms. Production soundness then leans on
  VMN's specification rather than only the interop path doing so — the verifier
  stays an independent implementation and still catches braid bugs, but a
  specification flaw would no longer be confined to a side path. Set against
  that: one proof path instead of two, so the drift risk the tests currently
  hold back disappears. And braid gains a global prefix binding version, sid,
  the bit lengths, PRG, group and hash — which its own transcripts bind
  nowhere today.

  **Export** remains the ideal, and would ask Verificatum to parse braid's
  format outright. Our half of it exists: `v2v verify` is export, inbound.

- **Make the interop generic over the curve.** Everything in `v2v` is P-256
  only — `wire::marshal::p256`, and `encode.rs` says as much in its module
  docs — while `vsc` and braid are generic over `Context`. Lifting that would
  cover the rest of the curve list both sides support (P-224, P-384, P-521,
  the brainpool and `secp*` families). It would **not** make braid's native
  Ristretto255 path verifiable: `vmnv` has no such group, and that limit is
  permanent regardless.

  Ciphertext width is the same problem, smaller: widths are const generic, so
  each is a separate monomorphisation and `emit::generate` dispatches over a
  fixed set of arms. That bound is a compile-time cost, not a design limit.
- **α in braid's native path.** Deferred, not rejected. It would live entirely
  inside `combine` and is worth roughly a fifth of it, but realising that needs
  a small-exponent entry point in the group API — `C::Scalar` is fixed-width,
  so `α·c_l = 108` at `k = 3` costs what a 256-bit scalar costs. Verificatum
  has the equivalent: `combineDecryptionFactors` passes the coefficients'
  maximum bit length into `expProd` so the ladder runs 7 iterations instead of
  256. A decision to take against a profile.
