<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# MSM in the Terelius–Wikström shuffle — feasibility assessment and design

**Status:** assessed design, ready to schedule.
**Scope:** `vsc::zkp::shuffle`, `vsc::traits::groups`, `vsc::groups::ristretto255`.
**Supersedes:** the original MSM refactor proposal (this file's previous
content), which was written without access to this source tree or to
`PROTOCOL.md`. This revision checks every idea in it against both, keeps what
survives, corrects what does not, and answers its two motivating questions.

The two questions, answered up front:

- **(a) Can plain MSM accelerate our algorithms?** Yes — an estimated **6–10×
  on shuffle verification and 4–6× on proving** at production sizes, entirely
  on CPU, with no transcript, wire-format, or proof-byte change, and a
  contained diff (two new trait methods plus rewiring inside `shuffle.rs`).
  Measured baseline and per-change accounting below.
- **(b) Can they be further accelerated on GPUs?** Technically yes, but it is
  **not recommended now**. Two of the original note's premises fail against
  this codebase (§2.1, §2.2), which shrinks the GPU's marginal value. The one
  existing GPU MSM for curve25519 — Anza's sppark-based `curve25519-cuda`
  (§2.5) — is unpublished and unaudited, so today the choice is between an
  immature dependency and hand-written kernels, either way with a real
  assurance and reproducibility cost. The CPU design below keeps every call
  site MSM-shaped, so a GPU backend remains a drop-in *if* a concrete
  throughput requirement ever justifies it (§7).

---

## 1. Verdict summary

Every idea from the original note, against the implementation and the
protocol:

| Original idea | Verdict | Why (section) |
|---|---|---|
| §2 inventory of six MSM sites, CT/vartime split | **Confirmed** — verified line by line | §3 |
| §3 `MsmBackend` trait with `prepare()`/`Prepared` | **Rejected** — the protocol makes prepared bases worthless (per-mix generators); extend the existing `multi_exp` seam instead | §2.1, §4 |
| §3.3 delegate to dalek `MultiscalarMul`/`VartimeMultiscalarMul` | **Confirmed**, with a correction: must be *chunk-parallelized*, or it loses to the current rayon-parallel baseline | §2.2, §4.2 |
| §3.2 `vartime_msm_shared` (shared digit decomposition) | **Deferred** — per-column calls recompute only cheap scalar digits; a CPU non-issue | §4.4 |
| §4.1 fold `v` into scalars, merge all equations into one MSM | **Rejected for CPU** — dalek's bucket overhead is negligible (w ≤ 8), so merging buys ~nothing; revisit only for GPU | §2.3 |
| §4.2 batch Verification 2 with random `tᵢ` | **Confirmed** — the largest single verifier win; math checked against PROTOCOL.md §6.4 | §5.2 |
| §4.3 closed form for the `big_b_n` chain | **Confirmed** — the recurrence is literally already in the code (`d_n`, Step 4); kills the prover's only serial stretch | §5.3 |
| §4.4 `big_b_prime_n` follow-on | **Confirmed** | §5.3 |
| §4.5 structure-of-arrays ciphertext layout | **Rejected** — column gathers are pointer-cheap on CPU; if a GPU backend ever exists, transpose at its boundary, not in the type system | §2.4 |
| §5 smaller items | **Mostly confirmed**, triaged individually | §5.5 |
| §6 GPU backend | **Deferred** with a concrete go/no-go rule; the security-posture and ops-hygiene guidance is kept | §7 |
| §9 128-bit `e_n` | **Kept as an open question** — genuine ~1.6–2× verifier upside, but it is a transcript + spec + soundness-argument change | §8 |
| §9 offline/online split (commitments before ballots) | **Rejected — impossible under the protocol**: the generators `h` are derived from the input ciphertext list, so nothing that depends on `h` can precede the ballots (PROTOCOL.md §2.5, §6.2) | §2.1 |

---

## 2. What the original note got wrong

These corrections drive the design, so they come first.

### 2.1 The generators are per-mix, not per-run

The note assumed "the `h_generators` are fixed for the entire mixnet run", and
built its `prepare()`/`Prepared` abstraction (and much of the GPU case) on
re-using device-resident or precomputed bases across proofs.

PROTOCOL.md §2.5/§6.2 says otherwise: `h = IndGenerators(N,
ctx("shuffle_generators", input))` where the instance input is (a hash of) the
mix's **input ciphertext list**. braid derives them exactly so
(`braid::trustee::mix.rs` — `shuffle_generators_seed` includes the input
hash), fresh for every mix link, and constructs a fresh `Shuffler` per prove
and per verify call.

Consequences:

- Within one `verify_with` call, **every base vector is used in exactly one
  MSM**: `u_n` in A, `h_n` in V1, `w` in F, `w'` in V5, `B/B'` in (batched)
  V2. Across calls, the bases differ. There is *no* reuse for `prepare()` to
  amortize — on CPU or GPU.
- The only bases reused within a proof are `g`, `pk.y` and `h₁ =
  h_generators[0]` — the *fixed-base-many-scalars* pattern, which is served by
  a precomputed table built per call (§4.3), not by an MSM `prepare()`.
- The note's open question about exposing TW's offline/online split (computing
  the permutation commitment before ballots arrive) is dead on arrival: `u_i =
  g^{r_i}·h_{π(i)}` depends on `h`, which depends on the ballots. Changing
  that is a protocol change, out of scope.

### 2.2 The baseline is already parallel — a bare dalek call would *lose*

The note's "~20× naive → Pippenger" compares sequential naive against
single-threaded Pippenger. But the current `map(exp).reduce(mul)` sites run
under rayon across all cores. A single `vartime_multiscalar_mul` call is
**single-threaded**: at N = 10⁵ on a 16-thread machine, replacing a parallel
naive product (~320 point-ops/exp ÷ 16 threads ≈ 20 N wall-units) with one
serial Pippenger call (~32–35 N) would be a **~1.7× slowdown**.

The fix is structural and cheap: **chunk the input, run one dalek MSM per
chunk on rayon, multiply the partial results.** Group associativity makes this
trivially correct. Per-chunk bucket overhead is small (dalek's Pippenger uses
w ∈ {6,7,8}, so ≤ 33 windows × 128 buckets ≈ 4–8k additions per chunk) and
chunk sizes stay above dalek's own Straus/Pippenger switch (190) and window
thresholds (500/800) for any interesting N. Net effect at N = 10⁵, 16
threads: **~9–10× over the parallel naive baseline** for vartime sites, ~5×
for constant-time sites (CT Straus: ~64 additions per point + 256 doublings
per chunk).

Two related facts checked against the pinned `curve25519-dalek 5.0.0-pre.6`:

- The vartime path dispatches Straus below 190 points and Pippenger above,
  with window size capped at **8** (6/7/8 by size) — not the c ≈ 16 the note
  assumed. Per-point cost ≈ 253/8 ≈ 32 additions.
- The **SIMD (AVX2) backend is already the default** on x86_64 (`build.rs`
  auto-selects it; runtime CPU detection picks AVX2 per call). The measured
  baseline below already includes it; there is no free SIMD multiplier left,
  and wasm builds use the serial backend regardless.

### 2.3 Merging equations into one giant MSM buys ~nothing on CPU

The note's motivation for folding `v` into the scalars and merging V1–V5 into
one size-(4+6W)N MSM was the per-MSM bucket-reduction term `d·2^c`. With
dalek's w ≤ 8 that term is ~10⁴ additions per MSM — noise against 32·N ≥
3×10⁶ at N = 10⁵. Total per-point work is unchanged by merging. Keeping the
five equations separate preserves the 1:1 correspondence with PROTOCOL.md
§6.4 (worth real review money in this codebase) at no measurable cost. The
merge is a GPU-era idea (launch amortization) and goes back on the shelf with
the GPU.

### 2.4 SoA layout: solving a non-problem

`Ciphertext<C, W>` is `[[C::Element; W]; 2]`, and `GroupElement::multi_exp`
takes `bases: &[&Self]` **specifically so a column can be gathered out of an
array-of-structs without copying** (see the trait's own doc comment). A
column gather is N pointer pushes — sub-millisecond at N = 10⁵ against
hundreds of ms of point arithmetic. Re-laying-out the ciphertext type would
touch serialization, the wire format, and every consumer, for no CPU benefit.
If a GPU backend ever exists, transpose in its upload path.

### 2.5 Minor GPU-note corrections (recorded so they don't resurface)

- "Scalar field ℓ … needs Montgomery or Barrett" — moot: MSM needs no mod-ℓ
  arithmetic on the device at all. Signed-digit decomposition consumes the
  scalar *bytes*; only the F_p coordinate arithmetic (fast reduction mod
  2²⁵⁵−19, no Montgomery needed) runs on the GPU.
- sppark/ICICLE/ec-gpu do not support curve25519/ristretto255 upstream —
  they target SNARK curves. The one known exception is Anza's
  `curve25519-cuda` (in `anza-xyz/cryptography`, companion to
  `solana-ed25519`, their 2026 curve25519-dalek fork): a **variable-time**
  GPU MSM (`msm_curve25519`) that maps curve25519 points onto an isomorphic
  short-Weierstrass model to reuse sppark's Pippenger, with a built-in
  GPU→CPU fallback. As of 2026-09 it exists at v0.2.0 in the repo but is
  unpublished (the crates.io name is a v0.0.0 "reserved for future use"
  placeholder), is unaudited, and backs their fork's types rather than our
  pinned dalek. It moves the GPU effort term from write-kernels to
  integrate-and-validate (§7). (Solana's older `solana-perf-libs`
  `cuda-ecc-ed25519` is batch signature verification, not MSM.)

---

## 3. Confirmed inventory (against `zkp/shuffle.rs` @ 0.6.5)

`P` = prover `shuffle_with` (`shuffle.rs:298`), `V` = verifier `verify_with`
(`shuffle.rs:564`). All verified against PROTOCOL.md §6.3–6.4; the CT column
is a per-site correctness property (ε, β blind the permutation; leaking them
breaks zero-knowledge).

| Site | Code | Lines | Size | Scalars | CT? |
|---|---|---|---|---|---|
| P: A′ | `h_n_epsilon_n_fold` | 367–378 | N | secret ε | **yes** |
| P: F′ | `w_prime_n_epsilon_n` + `fold_values` | 405–417 | N × 2W | secret ε, shared | **yes** |
| V: A | `big_a` | 615–617 | N | public e | no |
| V: F | `big_f` + `fold_values` | 620–626 | N × 2W | public e, shared | no |
| V1: | `h_n_k_e_n_fold` | 655–657 | N | public k_E | no |
| V5: | `w_prime_n_k_e_n_fold` | 713–718 | N × 2W | public k_E, shared | no |

Adjacent non-MSM hot spots, also verified present:

- the serial `big_b_n` chain, `shuffle.rs:345–353` (N dependent variable-base
  exps — the only unparallelizable stretch, and at N = 10⁵ roughly a third of
  prove wall-clock);
- Verification 2 as N independent equations, `shuffle.rs:662–692` (3N exps:
  `g^{k_B}`, `B_{i−1}^{k_E}`, `B_i^v`);
- sequential `u_n_fold`/`h_n_fold`, `shuffle.rs:629–636`;
- the sequential `e_n` derivation with a per-iteration digest clone,
  `shuffle.rs:186–194` (`NativeChallenges::batching_challenges`), run by both
  prover and verifier;
- `b_n` sampled with a plain `.map()` (`shuffle.rs:343`) beside
  `par_iter` neighbours;
- `apply_permutation`'s `g.exp(r)` and `re_encrypt`'s `[g, y].repl_exp(r)` —
  N and 2WN exponentiations of **fixed bases** through the variable-base
  path (`g_exp`, which uses dalek's precomputed basepoint table, exists at
  `traits/groups.rs:51` but the shuffle never calls it).

The "not MSM" list from the original note (per-item results: `u_n`, `g_b_n`,
re-encryption, `B′`, V2's `lhs/rhs` vectors) is confirmed — those are
fixed-base batches, served by §4.3, and must not be routed through an MSM.

What already exists as a seam (predates this work, do not duplicate):

- `GroupElement::multi_exp(bases: &[&Self], exps)` — CT contract documented,
  naive default, **overridden for ristretto only** with dalek's CT Straus
  (`groups/ristretto255/element.rs:81–92`). The doc comment already mandates
  that a vartime variant be a separate method.
- `DistGroupOps::dist_multi_exp` — broadcast form, delegates per column.
  Used today at exactly four sites, all in batched decryption
  (`dkgd/recipient.rs:438–439, 776–777`).
- The differential-test template:
  `productgroup/tests.rs::test_multi_exp_override_matches_the_naive_default`.

---

## 4. Design: extend the existing seam, no backend object

### 4.1 Two new trait methods (plus one optional)

```rust
// traits::groups::GroupElement

/// Multi-exponentiation, variable-time. ∏ bases[i]^{exponents[i]}.
///
/// PRECONDITION: all exponents are PUBLIC values (verifier-side batching
/// exponents, published responses). Timing may depend on them. For secret
/// exponents use `multi_exp`.
fn vartime_multi_exp(bases: &[&Self], exponents: &[Self::Scalar])
    -> Result<Self, Error> {
    Self::multi_exp(bases, exponents) // sound (CT is a valid vartime), slow
}

/// Fixed-base batch: self^{s} for each s, constant-time per element.
/// Implementations may precompute a table over `self` and must amortize it
/// across the batch. Must be constant-time in the exponents.
fn exp_many(&self, exponents: &[Self::Scalar]) -> Vec<Self> {
    exponents.iter().map(|s| self.exp(s)).collect()
}
```

Ristretto overrides:

- `vartime_multi_exp`: chunk into `max(N / rayon::current_num_threads(),
  ~2·10³)`-sized pieces, one `RistrettoPoint::vartime_multiscalar_mul` per
  chunk on rayon, multiply partials. (Below one chunk's worth, a single
  direct call — dalek already dispatches Straus/Pippenger by size.)
- `exp_many`: `RistrettoBasepointTable::create(&self.0)` (CT radix-16 table,
  ~256 point-ops to build, ~30 KB), then `par_iter` table-mults (~64
  additions each vs ~320 for variable-base). Special-case `self ==
  basepoint` to reuse the compiled-in table.
- Also chunk-parallelize the existing **CT** `multi_exp` override the same
  way (partial CT Straus per chunk; each chunk pays its own 256 doublings —
  still ~5× over the per-exp baseline).

P-256 keeps the defaults (correct, slower — it is a test/interop context;
`p256` exposes no public multi-exp API worth chasing now).

Optional, later: `GroupScalar::random_128(rng)` (default: full-width
`random`) so batched V2's `tᵢ` can be small exponents — dalek's vartime paths
skip zero digits, so 128-bit `tᵢ` roughly halve their MSM terms. Not needed
for v1: full-width random `tᵢ` are sound (soundness error ~1/q) and cost only
~2× on 3 of ~13 MSMs.

**Why no `MsmBackend` object:** with §2.1, `prepare()` has nothing to
amortize; without `prepare()` the trait collapses to exactly the two methods
above, which belong on the existing `GroupElement` seam where a CT/vartime
pair is already documented policy. Differential testing needs no backend
object either — the naive default *is* the reference implementation, and the
existing test template pins overrides against it. `Shuffler::with_backend`
falls away with the trait. If a GPU ever arrives, it slots in as an
alternative implementation *inside* the ristretto overrides (feature-gated,
CPU fallback on any error), or a backend enum is introduced then — deciding
that today would be speculation.

### 4.2 Call-site pattern for the W dimension

Verifier F / V5 and prover F′ gather each of the 2W columns as `Vec<&Element>`
straight out of `&[Ciphertext<C, W>]` (this is what the `&[&Self]` signature
is for) and make 2W independent `(vartime_)multi_exp` calls, themselves under
a rayon scope. `dist_multi_exp`'s `&[Self]` signature doesn't fit
`&[Ciphertext]` without copying halves; the column-gather form does, and the
decryption sites that already use `dist_multi_exp` stay as they are.

### 4.3 Annotation discipline

Every rewired site gets a one-line comment stating *why* its timing class is
correct (`// vartime OK: e_n / k_e_n / t_n are public` — or `// CT required:
epsilon is secret blinding`). The original note was right that getting this
backwards in either direction is a real bug; it stays right.

### 4.4 Deliberately not building

- `vartime_msm_shared`: sharing digit decomposition across the 2W columns
  saves only scalar-byte processing (trivial next to point work on CPU).
  Reconsider only inside a GPU backend, where it becomes one radix sort
  feeding 2W accumulations.
- Serializable prepared tables: moot per §2.1.

---

## 5. The shuffle changes

All of these preserve **bit-identical proofs and identical accept/reject
behaviour** (except V2's documented 2⁻ᵏ batching error, §5.2): they change how
values are computed, never what they are. The existing `test_shuffle_*` suites
must pass unchanged, under both `NativeChallenges` and `VmnChallenges`.

### 5.1 Verifier: wire the four public MSM sites

`big_a`, V1's `h^{k_E}` fold, and the 2W-column products for `big_f` and V5
go through `vartime_multi_exp`. `u_n_fold`/`h_n_fold` (unit exponents — a
multi-add, not an MSM) become parallel rayon reductions; caching `h_n_fold`
in `Shuffler` is pointless in braid's usage (fresh `Shuffler` per call) and
is dropped from the plan.

### 5.2 Verifier: batch Verification 2

Replace the N elementwise checks `B_i^v·B'_i = g^{k_{B,i}}·B_{i−1}^{k_{E,i}}`
(PROTOCOL.md §6.4 V2; `shuffle.rs:662–692`) with one small-exponent batch
(Bellare–Garay–Rabin): draw verifier-local random `t₁..t_N`, accept iff

```text
∏ B_i^{v·t_i} · ∏ B'_i^{t_i} · ∏ B_{i−1}^{−t_i·k_{E,i}} · g^{−Σ t_i·k_{B,i}} == 1
```

with `B₀ = h₁`. One vartime MSM of size 2N+2 (collect per-base exponents:
`B_i` gets `v·t_i − t_{i+1}·k_{E,i+1}` for i < N and `v·t_N` for i = N),
replacing 3N full exponentiations — the single largest verifier win.

Soundness: if any equation i fails, acceptance probability over `t_i` is
≤ 1/|T| (2⁻²⁵² full-width; 2⁻¹²⁸ if `random_128` lands). The `t_i` are the
**verifier's own randomness, drawn after the proof is fixed** — no transcript
involvement, no prover coordination, no effect on `ShuffleChallenges` or on
the Verificatum path (`v2v` verifies VMN sessions through this same verifier;
a randomized-batch V2 still accepts exactly the valid ones).

Two consequences to document:

- `verify_with` becomes internally randomized (`C::get_rng()`; works under
  wasm via `getrandom`/`wasm_js`). Its signature and result are unchanged.
- PROTOCOL.md needs a D-series-style precision note on §6.4/§9.2: a verifier
  MAY check V2 (and in principle any of V1–V5) via random-weight batching
  with the stated error bound; the equations as written remain the normative
  statement. Independent verifiers doing the exact check accept a superset
  of what we accept, differing with probability ≤ 2⁻¹²⁸ — never the reverse
  direction on honest proofs.

### 5.3 Prover: closed-form bridging chain, and its follow-on

Verified against the code: the chain `B_i = g^{b_i}·B_{i−1}^{e'_i}`
(`shuffle.rs:345–353`) and the response recurrence `d₁ = b₁, d_i = b_i +
e'_i·d_{i−1}` (`shuffle.rs:462–473`) are the same recurrence computed twice —
`d_i` **is** the discrete log of `B_i` base g up to the `h₁` component. With
`p_i = ∏_{k≤i} e'_k` (prefix product, cheap field scan; `p₀ = 1, d₀ = 0`):

```text
B_i  = g^{d_i}  · h₁^{p_i}
B'_i = g^{β_i + d_{i−1}·ε_i} · h₁^{p_{i−1}·ε_i}
```

Base case and induction check out (B₁ = g^{b₁}h₁^{e'₁} ✓), and the verifier's
D = B_N·h₁^{−∏e} = g^{d_N} matches V4 exactly (∏e' = ∏e since e' is a
permutation of e). So:

- move the `d_n` computation before commitment generation and reuse it in
  Step 4 (dedupe);
- `big_b_n` becomes `g.exp_many(&d_n)` ⊙ `h₁.exp_many(&p_n)` — 2N
  constant-time table exponentiations, fully parallel, and the N
  `g^{b_i}` exps (`g_b_n`) disappear entirely;
- `big_b_prime_n` becomes two more `exp_many` batches (§4.4 of the original
  note, confirmed), replacing N variable-base + N fixed-base exps.

Constant-time notes: `d_i`, `p_i`, and `e'_i` are secret (e' reveals the
permutation), so the scans stay in (constant-time) field ops and the
exponentiations stay on the CT table path. This removes the prover's only
serial stretch — at N = 10⁵ roughly 6 s of the measured 18.5 s prove.

Migration safety: land the closed form alongside the loop first, with an
equality test over several (N, W) cells (including N = 1), then delete the
loop — plus a fixed regression: proofs from before/after on a seeded input
must be byte-identical (they are the same group elements).

### 5.4 Prover: remaining wiring

- A′ through CT `multi_exp` (secret ε); F′ through 2W CT column MSMs
  (§4.2).
- `apply_permutation`: `u_n = g^{r}·h_π` via `g.exp_many(&r_permuted)`;
  re-encryption's `(g^s, y^s)` legs via `exp_many` on `g` and on `pk.y`
  (gather the W columns of `s_n`). Same for `encrypt_with_r` if touched.
- Singleton `g.exp(..)` calls (`A′`, `C′`, `D′`, verifier RHS 3/4, `g^{k_a}`)
  switch to the existing `C::G::g_exp` (basepoint table; free ~4×, N-independent
  but tidy).

### 5.5 Small-item triage (original §5)

| Item | Verdict |
|---|---|
| Parallelize `e_n` derivation (borrowed prefix, no per-iteration digest clone) | **Do** — N independent `hash_to_scalar` calls, byte-identical output, runs in both prove and verify; at N = 10⁵ ~100 ms serial today |
| `b_n` plain `.map()` | **Do** — `into_par_iter`, trivial |
| `gen_private_exponents` RNG-in-closure | **Demote** — `C::get_rng()` is a thread-local handle fetch, not a reseed; `map_init` is a micro-opt, measure later |
| `shuffle_slice_random` dead code | **Do** — delete; keep the Fisher–Yates and add the unbiasedness comment (`random_range(0..=i)`) |
| `fold_values` obsolescence | **Confirmed** — after §5.1/§5.4 the group-element folds are MSM calls; `fold_values` remains for scalar folds only, which shrinks the `bounded-combine` decision (PERFORMANCE.md §1) to the scalar case |

### 5.6 Parallelism-completeness audit (pre-MSM stage)

Rayon is applied unevenly today, and that threatens measurement validity as
much as performance: gains measured against an under-parallelized baseline
would be attributed to MSM when they are really just missing `par_iter`. So
the parallelization fixes land **first, as their own stage, re-baselined
with `shuffle_scaling` before any MSM work** — and the audit is
redesign-aware: sites the MSM/algebra work deletes get no effort.

Sequential sites found (systematic sweep, not only the
`#[crate::warning]`-annotated ones):

| # | Site | Verdict |
|---|---|---|
| S1 | `NativeChallenges::batching_challenges` e_n loop (`shuffle.rs:186–194`) — N sequential `hash_to_scalar`, runs in prove *and* verify | **Parallelize** (borrowed prefix, byte-identical output) |
| S2 | `b_n` sampling (`shuffle.rs:343`) — plain `.map()` | **Parallelize** |
| S3 | `u_n_fold`/`h_n_fold` (`shuffle.rs:629–636`) — sequential point products | **Parallelize** (rayon reduce) |
| S4 | `ser()` of N-sized lists inside challenge derivation — sequential ristretto compressions | **Measure first** — this is PERFORMANCE.md §3 (parallel serialization behind the unchanged encoding); do when profiles show it, likely right after MSM lands |
| S5 | `big_b_n` chain (`shuffle.rs:345–353`) | **No rayon effort** — inherently sequential; §5.3's closed form is its fix |
| S6 | V2 `lhs_2`/`rhs_2` (already parallel) | **No effort** — deleted by §5.2 |
| S7 | `c` fold, `d_n` recurrence (`shuffle.rs:449, 462–473`) — scalar ops | **Leave** — cheap; `d_n` inherently sequential |
| D1 | `partial_decrypt` factor computation (`dkgd/recipient.rs:425–428`) — N×W CT exponentiations, sequential | **Parallelize** — the trustee decryption hot path |
| D2 | `batching_exponents` (`recipient.rs:651–656`) — N sequential `hash_to_scalar`, runs in `partial_decrypt` and per contribution in `combine` | **Parallelize** |
| D3 | `combine` Lagrange accumulation (`recipient.rs:793–796`) — T·N×W exponentiations, sequential; λ public → also vartime-eligible (and the VERIFICATUM.md small-α idea slots here later) | **Parallelize** (per-item results — a fixed-scalar batch, *not* an MSM) |
| D4 | `combine` plaintext extraction (`recipient.rs:799–803`) — N inversions+muls, sequential | **Parallelize** (cheap, free to fix) |
| G1 | P-256 `ind_generators` (`p256/group.rs:107–126`) — sequential, annotated; ristretto's is parallel | **Parallelize** (symmetry, test/interop speed) |
| B1 | braid `mix_input_ciphertexts` Naor-Yung verify+strip (`braid trustee/mix.rs:63–72`) — N sequential PlEq verifications (~6 exps each), run by *every* quorum member for the first link | **Parallelize** (preserve the per-index error attribution) |
| — | `dkgd::dealer` share/checking-value loops — T·P-sized | **Leave** — DKG is small and not hot |
| — | `gen_private_exponents` RNG-in-closure | **Leave/measure** — already `par_iter`; `map_init` is a micro-opt |

All of these preserve outputs bit-for-bit (independent iterations, or
order-preserving folds).

**Stage 0b — the reverse direction.** Parallelism is not free (split/steal
overhead, committed worker stack, visual noise), so the same audit runs
backwards: the `benches/parallel_tradeoff.rs` criterion bench measured every
per-element shape serial vs parallel, and every site whose parallel form
saved < 0.1% of prove/verify in *absolute* wall-clock (not raw speedup ratio)
was reverted to a plain serial loop with a comment citing the bench. That
covered all the scalar loops (`b_n`, `beta`/`epsilon`, `a`, `k_b_n`,
`k_e_n`, `e_n_fold`) and the point-product folds (`u_n_fold`, `h_n_fold`,
`combine`'s plaintext extraction) — S2/S3 above among them. Kept parallel:
everything point-exponentiation or hashing (the MSM sites, `partial_decrypt`
factors, `batching_exponents`, `combine`'s Lagrange step, and S1 `e_n`, which
is hashing and the one cheap-looking site that is really ~0.7%). See the
PERFORMANCE.md stage-0b table.

### 5.7 Rider: the decryption sites

PERFORMANCE.md §2 already queues vartime adoption for the four
`dist_multi_exp` sites in `dkgd/recipient.rs` (bases and exponents all
public). With `vartime_multi_exp` in the trait, `dist_multi_exp` gets a
vartime twin (or those sites gather columns directly) — a few lines each.
Same PR series, near-free.

---

## 6. Measured baseline and expected effect

`examples/shuffle_scaling.rs`, this machine (Windows x64, 16 logical cores,
AVX2 dalek backend, `--release`, fold=`reduce`), 2026-09-20:

| N | W | prove (ms) | verify (ms) |
|---|---|---|---|
| 10⁴ | 2 | 1 355 | 1 058 |
| 10⁵ | 2 | 18 535 | 13 172 |

Accounting at N = 10⁵, W = 2 (per-exp ≈ 320 add-equivalents, 16 threads):

- **Verifier**: ~13N exponentiations today, ~all of wall-clock. After: ~11
  vartime MSMs (A, F×4, V1, V5×4, V2-batch ≈ 2 more N-units) at ~32–35
  additions/point chunk-parallel → **expected ~6–10× (13 s → ~1.5–2.5 s)**,
  at which point challenge derivation and serialization become co-dominant
  (§5.5 e_n item, then PERFORMANCE.md §3).
- **Prover**: serial chain ~6 s → ~0.4 s parallel closed form; fixed-base
  batches (u_n, re-enc, B, B′) ~5×; CT MSMs (A′, F′) ~5× → **expected ~4–6×
  (18.5 s → ~3–4.5 s)**, floor set by RNG, hashing, serialization.

These are estimates to be confirmed with the same tool after each step;
record cells in PERFORMANCE.md as they land.

---

## 7. GPU (question b): assessment and decision rule

**What it would take.** One candidate now exists (§2.5): Anza's
`curve25519-cuda` — vartime `msm_curve25519` over sppark's Pippenger via a
short-Weierstrass mapping of curve25519, GPU→CPU fallback included. Adopting
it would be integrate-and-validate rather than write-kernels: it backs
their dalek fork (`solana-ed25519`) rather than our pinned
`curve25519-dalek 5.0.0-pre.6`, so points cross the boundary either through
the canonical 32-byte encodings (RFC 9496; braid's verifier bases arrive as
those wire bytes anyway) or by coordinate conversion, plus the
Edwards→short-Weierstrass map its API implies; ristretto MSM over curve
representatives is sound. Its vartime orientation fits the verifier-only
GPU posture. Costs that remain regardless: an unpublished, unaudited,
fast-moving dependency in the trust chain of an election verifier; the full
differential-test campaign (dalek as oracle, property tests across
backends, rare-limb-pattern hunting); and the ops rules below. Without it,
a backend is hand-written CUDA (or wgpu) F(2²⁵⁵−19) limb arithmetic,
extended-Edwards point ops, and a bucketed Pippenger — a multi-month
effort carrying the same test campaign.

**What it would buy, corrected.** The original note's GPU case leaned on (a)
prepared device-resident generators — void per §2.1 — and (b) a 5–20× over
multicore CPU on MSM. After the CPU work lands, the verifier at N = 10⁵ is
~1.5–2.5 s with MSM no longer the sole dominant term; a GPU that zeroes MSM
time yields perhaps 2–4× more end-to-end, only after the hash/serialization
residue is also parallelized, and only on machines with the hardware. The
protocol's own structure caps it too: mix links verify serially per trustee
(t verifies of size N per tally per trustee), and browser trustees (wasm)
can never use it.

**Where it could genuinely matter:** a bulk election verifier re-checking
very large elections (N ≥ 10⁶, many contests) as an offline batch job — a
throughput setting, and the one MSM.md's §6.2 posture fits (GPU on the
verifier only; prover secrets never reach VRAM).

**Decision rule:** revisit if and only if, after steps 1–5 below are profiled,
(a) MSM still holds ≥ 70% of verifier wall-clock at the deployment's real N,
**and** (b) a concrete verification-latency requirement exists that CPU
scaling (more cores; the 128-bit `e_n` option in §8) cannot meet. Track the
maturity of Anza's `solana-curve25519-cuda` (§2.5) as an input to that
decision — a shipped, maintained sppark curve25519 backend materially lowers
the cost side of the ledger without changing the value side. The
operational constraints from the original note carry over verbatim as
preconditions: feature-gated build with zero CUDA dependency by default, CPU
path remains normative (and the only one used for Verificatum-interop
claims), silent CPU fallback on any device error, size-based dispatch, and
the decentralization caveat stated in docs.

---

## 8. Open question kept: 128-bit `e_n` (NativeChallenges only)

Unchanged in substance from the original note, with two added facts: (a)
Verificatum itself uses fixed-bit-length batching exponents (n_e; see
VERIFICATUM.md), so there is precedent; (b) with dalek's zero-digit skipping,
128-bit `e_i` would roughly halve the A/F/V1/V5 MSM cost — the estimated
~1.6–2× on the whole verifier stands. It remains a transcript change
(NativeChallenges only, never the VMN convention), a PROTOCOL.md §2.3/§6.3
edit, and a soundness re-derivation (in TW, `e` drives the
permutation-matrix argument itself; the Schwartz–Zippel bound becomes ~N/2¹²⁸
and the extraction argument must be re-checked over the smaller challenge
set). Decide separately; nothing in this design depends on it either way.

---

## 9. Sequencing

Each stage is measured with the same `shuffle_scaling` cells before the next
begins, so every factor in the final result is attributable to its stage —
in particular, MSM gains are measured against the *rayon-complete* baseline
of stage 0, not against today's under-parallelized one.

0. **Parallelism-completeness pass** (§5.6): the "Parallelize" rows (S1–S3,
   D1–D4, G1, B1, plus the §5.5 trivia). Bit-identical outputs;
   re-baseline.
1. **MSM microbenchmark + traits + ristretto overrides + tests** — a
   stable-toolchain criterion bench (`vsc/benches/msm.rs`) comparing
   parallel-naive vs CT Straus vs vartime, single-call vs chunked, plus the
   fixed-base-table pattern — validating §2.2 and fixing chunk sizes
   *before* wiring; then `vartime_multi_exp`, `exp_many`, chunk-parallel CT
   `multi_exp`; differential tests against the naive defaults (template
   exists), edge cases (empty, single, mismatched lengths, identity bases,
   scalars near ℓ), CT/vartime annotations.
2. **Verifier wiring** (§5.1) + **batched V2** (§5.2). Largest win,
   verifier-only. Includes the PROTOCOL.md batching note and negative tests
   (each `test_shuffle_invalid_*` still rejects; add a corrupted-B/B′ case
   aimed specifically at batched V2). **Done** (2026-09-20): 2a wired
   `big_a`/`big_f`/V1/V5 to `vartime_multi_exp`/`dist_vartime_multi_exp`
   (bit-identical accept/reject); 2b batched V2 with verifier-local `t_i`
   (full-width; `random_128` deferred), tested by `test_shuffle_batched_v2_
   rejects_*` (tamper `k_b_n`, which only V2 uses and which does not feed the
   challenge, so it isolates the batch). PROTOCOL.md §6.4/§9.2 batching note
   still to write.
3. **Prover** (§5.3, §5.4): closed form beside the loop → equality test →
   swap; fixed-base batches; CT MSMs. Byte-identity regression on seeded
   inputs. **Done** (2026-09-20): `bridging_commitments` computes
   `B_i = g^{d_i}·h_1^{p_i}` via two `exp_many` batches (d_n reused as the
   Step-4 response `d`, p_n reused for B'), replacing the serial loop; B′ is
   its closed-form follow-on (two more `exp_many` batches); A′ uses CT
   `multi_exp` + `g_exp`, F′ uses CT `dist_multi_exp`. Verified by
   `test_bridging_closed_form_*` (closed form == loop for N ∈ {1,2,5,10,65})
   and the roundtrip (which uniquely pins B/B′ via V2/V4). Still open in the
   prover, deferred as lower-value fixed-base cleanups: `apply_permutation`'s
   `u_n = g^r·h` and the re-encryption `(g^s, y^s)` legs still use per-element
   `exp`/`repl_exp` rather than `exp_many`; the singleton `g.exp` sites in the
   proof commitments were moved to `g_exp` only for A′.
4. **Decryption rider** (§5.7).
5. **Profile** with `shuffle_scaling` across (N, W) cells; serialization
   (S4/PERFORMANCE.md §3) if it now dominates; update PERFORMANCE.md
   (including the now-scalar-only `bounded-combine` decision); apply the §7
   GPU decision rule and record the outcome here. **Done** (controlled run
   2026-09-21, PERFORMANCE.md): MSM is now 8× at the primitive but only ~1.9×
   end-to-end on verify — **serialization (ristretto compression in the
   Fiat-Shamir seeds) now dominates**, not MSM. Next lever is parallel
   serialization + hoisting `combine`'s per-contribution re-serialization
   (PERFORMANCE.md item 3), then the deferred prover fixed-base cleanups. The
   §7 GPU go/no-go rule (≥70% of verifier in MSM) is **not met** — GPU stays
   deferred.

Constraints that bind every step: proof bytes and transcripts unchanged
(`VmnChallenges`/v2v unaffected — verified: batched V2 is verifier-internal);
`vsc` lint levels (`missing_docs`, `unwrap_used`, `arithmetic_side_effects`
etc. are deny — document and justify every new item); wasm builds must keep
compiling (`rayon` is already unconditional in vsc; chunked MSM runs on
wasm-bindgen-rayon's pool exactly like the folds it replaces).
