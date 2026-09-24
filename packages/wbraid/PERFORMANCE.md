<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# Performance — braid v0.6

The performance record for the shuffle and threshold-decryption cryptography
(`vsc`) and its `braid` callers. It describes **what is implemented now** —
the numbers, the techniques and where each is applied, the invariants they
must respect and how those are verified, what remains, and how to measure —
and keeps the **history of how it got there** in the Log at the end.
Performance is a first-class *future* concern for v0.6 (large ballot sets,
browser-hosted trustees), which prioritized correctness and clarity.

## Status

Authoritative numbers come from the reference machine (BENCH-EC2.md): a fresh
**`c7i.4xlarge`** — Intel Xeon Platinum 8488C, 16 vCPU (8 cores × 2 threads),
32 GiB; Ubuntu 24.04, rustc 1.96.0, `eu-west-1` — quiesced by construction,
with both commits built and run **interleaved**, three reps per cell, medians.
Measured 2026-09-22, fork point `657cb05c20` (the parent branch before the
optimization work) → merged milestone `185dbbede2`, N = 10⁵:

| Target | W = 2: before → after | | W = 5: before → after | |
|---|---|---|---|---|
| ① shuffle prove | 13.85 s → 4.92 s | **2.8×** | 23.5 s → 8.76 s | **2.7×** |
| ② shuffle verify | 10.26 s → 2.23 s | **4.6×** | 19.8 s → 3.57 s | **5.6×** |
| ③ partial decryption | 15.3 s → 1.19 s | **12.8×** | 38.2 s → 2.98 s | **12.8×** |
| ④ combine | 45.9 s → 1.99 s | **23×** | 114.8 s → 4.94 s | **23×** |
| ⑤ Naor-Yung verify-and-strip | 3.64 s → 3.70 s | flat | 8.82 s → 8.87 s | flat |

**Follow-up, 2026-09-23** — milestone `185dbbede2` → `009b443add` (batched
Naor-Yung verification), same machine and method, N = 10⁵:

| Target | W = 2: before → after | | W = 5: before → after | |
|---|---|---|---|---|
| ⑤ Naor-Yung verify-and-strip | 3.74 s → 1.00 s | **3.8×** | 8.96 s → 2.26 s | **4.0×** |
| ①–④ (controls) | unchanged within 0.5% | | unchanged within 0.5% | |

Strip is paid once per tally, in the first mix, by its producer and by everyone
who checks it, so its weight is read against those totals (same session, same
reps):

| First mix, N = 10⁵ | W = 2 | | W = 5 | |
|---|---|---|---|---|
| as verified: strip + shuffle verify | 6.01 s → 3.27 s | **1.84×**; strip 62% → 30% | 12.61 s → 5.89 s | **2.14×**; strip 71% → 38% |
| as produced: strip + shuffle prove | 8.76 s → 6.01 s | **1.46×**; strip 43% → 17% | 17.96 s → 11.22 s | **1.60×**; strip 50% → 20% |

**Follow-up, 2026-09-24** — `2d23452f05` → `6f4c995c22` (the shuffle's second
challenge serializes its two commitment lists in parallel; found by the stage
breakdown below), same machine and method, N = 10⁵:

| Target | W = 2: before → after | | W = 5: before → after | |
|---|---|---|---|---|
| ① shuffle prove | 4.97 s → 4.27 s | **1.17×** | 8.93 s → 8.21 s | **1.09×** |
| ② shuffle verify | 2.26 s → 1.53 s | **1.48×** | 3.62 s → 2.88 s | **1.26×** |
| ③ ④ ⑤ (controls) | unchanged within 1% | | unchanged within 1% | |
| T(3), V(3) at W2 | 27.07 s → 22.53 s, 9.88 s → 7.63 s | **1.20×**, **1.30×** | | |

**Follow-up, 2026-09-24 (lever 2)** — `0ae4a48ea3` → `81c3a768a5` (the prover's
permutation commitments and re-encryption legs as fixed-base batches,
`cae05fe816`), same machine and method, N = 10⁵:

| Target | W = 2: before → after | | W = 5: before → after | |
|---|---|---|---|---|
| ① shuffle prove | 4.22 s → 3.05 s | **1.38×** | 8.13 s → 5.53 s | **1.47×** |
| ② ③ ④ ⑤ (controls) | unchanged within 1% | | unchanged within 1% | |
| T(3), V(3) at W2 | 22.57 s → 18.83 s, 7.69 s → 7.67 s | **1.20×**, 1.00× | | |

Where the tip stands at N = 10⁵, same machine at reference speed (median of 3,
ms, the "after" column of that session; the smaller cells scale linearly and are
on record in the raw files):

| N : W | prove | verify | partial_decrypt | combine | ny_strip |
|---|---|---|---|---|---|
| 10⁵ : 2 | 3 053 | 1 532 | 1 205 | 2 029 | 998 |
| 10⁵ : 5 | 5 527 | 2 929 | 3 025 | 5 029 | 2 301 |

Resolution: reps agree within ~1%, and the same binary run standalone vs
interleaved agrees within 0.6–0.7% — differences below that are noise **within
a session**. Across sessions the host varies: sessions on the same instance
type hours apart (2026-09-23, 2026-09-24) differed by ~20–25% on every stage,
so a before/after is only trustworthy interleaved in one session — which is
how the differential runs — and snapshots from different sessions compare
only to that tolerance (each session's 10³ cell is its calibration). Raw
files: `bench-results/ec2-20260922-012110-185dbbede2/` (fork → milestone),
`bench-results/ec2-20260923-155859-009b443add/` (milestone → batched NY) and
`bench-results/ec2-20260924-015429-6f4c995c22/` (→ the second-challenge fix,
with the tally before/after and the `--ser` cell),
`bench-results/ec2-20260924-025652-0ae4a48ea3/` (→ parallel lists, with and
without `--ser`) and `bench-results/ec2-20260924-031018-81c3a768a5/` (→ the
fixed-base batches, with the breakdown), each with `SUMMARY.md`, the
differential CSVs, the grid and `machine.txt`.

Reading it: the **decryption path is the headline** (~13× and ~23×) — it had
the most headroom and every technique below applies to it; the **shuffle**
gains more on verify than prove because the verifier may use variable-time
multi-exponentiation and batch V2 while the prover's remaining cost is
constant-time by necessity; and **⑤ was flat at the milestone because nothing
had touched vsc's `NYStrip` crypto** (only braid's loop around it was
parallelized) — the follow-up batched it, and ⑤ went from the slowest target
at both widths to the fastest at 10⁵.

**Where the time goes** (`vsc --features profile`, reference machine,
2026-09-24, after both levers — tip `81c3a768a5`, N = 10⁵; share of each
stage's wall-clock, W2 / W5). Categories are wall-clock at the stages' outer
call sites, so a category's share is what removing it would save:

| stage | MSM | per-element exps | fixed-base | transcript ser | hashing | generators | unattributed |
|---|---|---|---|---|---|---|---|
| prove | 24% / 29% (constant-time) | — | **41% / 37%** | 17% / 18% | 6% / 7% | 3% / 2% | 9% / 7% |
| verify | **41% / 42%** | | | **32% / 34%** | 13% / 13% | 6% / 3% | 9% / 8% |
| partial_decrypt | 16% / 16% | **64% / 64%** | | 14% / 13% | 6% / 5% | | 1% / 1% |
| combine | **65% / 65%** | | | 24% / 24% | 11% / 10% | | 0% / 1% |
| ny_strip | 39% / 44% | | | | **56% / 52%** † | | 5% / 5% |

† the per-ballot challenge derivations, which include each ballot's
serialization. Over one mix (prove + verify) at W2: MSM 30%, fixed-base
batches 27%, transcript serialization 22%, hashing 9%, generators 4%,
unattributed 9%. Before the levers (tip `3e5c25e86c`, same method) the
prover's *unbatched* exponentiations were the largest single item at 42–48%
of prove; they are gone, the fixed-base batches that replaced them cost about
half as much, and the constant-time MSMs `A′`/`F′` are now the prover's
largest item after them (the accepted premium, Constraints). The analytic
estimates that preceded the measurement (MSM ~20%, serialization ~20% of a
mix) were right for what they could see; what they could not see was the
unbatched exponentiation, and that message encoding/decoding dwarfed all of
it until `0ae4a48ea3`.

**The global target** (`examples/tally.rs`; same machine, 2026-09-23, tip
`2d23452f05`): one tally's **critical-path latency** `T(Q)` for a quorum of Q
acting in turn, and the external verifier's `V(Q)`, N = 10⁵, median of 3, each
stage with its share of T:

| N : W : Q | T | V | strip ×2 | prove ×Q | verify ×Q | partial | combine |
|---|---|---|---|---|---|---|---|
| 10⁵ : 2 : 3 | **27.2 s** | 10.0 s | 2.0 s (7%) | 15.0 s (55%) | 6.9 s (25%) | 1.2 s (4%) | 2.0 s (7%) |
| 10⁵ : 5 : 3 | **50.5 s** | 18.3 s | 4.6 s (9%) | 26.9 s (53%) | 11.0 s (22%) | 3.0 s (6%) | 5.0 s (10%) |
| 10⁵ : 2 : 5 † | **34.8 s** | 12.9 s | 1.7 s (5%) | 20.2 s (58%) | 9.7 s (28%) | 0.9 s (3%) | 2.4 s (7%) |
| 10⁵ : 2 : 7 † | **47.6 s** | 17.6 s | 1.6 s (3%) | 28.3 s (59%) | 13.6 s (28%) | 1.0 s (2%) | 3.2 s (7%) |
| 10⁶ : 1 : 2 | **138.3 s** | 51.0 s | 11.5 s (8%) | 75.2 s (54%) | 37.4 s (27%) | 6.2 s (5%) | 7.9 s (6%) |

The replay agrees with the formula over the isolated targets above —
`2·strip + Q·(prove + verify) + partial + combine` gives 27.0 s and 50.2 s —
within 0.6%, the machine's resolution: the five targets compose additively, and
the path is what the formula says it is. More than half of it is shuffle
proving; strip, the slowest single target before batching, is under a tenth.

† The Q = 5 and Q = 7 rows come from a second session an hour later whose host
ran **~20% faster on every stage** — same instance type, AZ, CPU model and
kernel; the 10³ calibration cell 50 ms vs 62 ms for prove — so their absolute
values are not comparable with the rows above (see Resolution). Read the
Q-scaling *within* that session: every mix cost the same (prove 4.03 s, verify
1.94 s, all 12 within 1%), each extra partial's verification 0.42 s, and the
two rows fit `T(Q) ≈ 2.8 s + 6.4 s·Q` to 0.2% — linear in the quorum size, with
the intercept the two strips, the partial and the Lagrange step. On the
reference-speed host of the first rows the slope is ~7.9 s per trustee.

These rows predate the day's levers. On the tally at 10⁵/W2/Q3, measured
interleaved step by step: T(3) 27.07 s → 22.53 s (the second-challenge fix) →
18.83 s (the fixed-base batches), **1.44× in all**, and V(3) 9.88 s → 7.63 s;
with message encoding/decoding on the path, 56.9 s → 23.7 s after the parallel
lists, before the fixed-base batches. The other cells shift by the same
per-stage ratios.

**Message encoding and decoding, measured and fixed** (`tally --ser`,
10⁵/W2/Q3): with each posted message encoded by its producer and decoded by
its consumer, T was **62.4 s, of which 39.8 s (64%) was encoding/decoding**
and 22.5 s the cryptography — both directions ran sequentially, `Vec::ser`
when a list was posted and `deser` when it was read, about seven million
point compressions and decompressions per tally on one core. `0ae4a48ea3`
(parallel lists behind the unchanged encoding, Design) measured interleaved
against `6f4c995c22` on the same machine: **encoding/decoding 36.9 s → 3.7 s
(10.0×)**, T with messages on the path 56.9 s → 23.7 s (**2.40×**); every
crypto stage, and T without `--ser`, unchanged within 1%
(`bench-results/ec2-20260924-025652-0ae4a48ea3/`). Message handling is now
~16% of the path instead of 64%; it is not in T's default composition.

The 10⁶ : 1 : 2 row is the scenario measured on older implementations, for
comparison with them. Its session ran the 10⁵ : 2 : 3 cell alongside as an
anchor and reproduced 27.1 s, so that host was at the reference speed and the
row is comparable with the Q = 3 rows. Per mix at a million ballots of width
1: prove 37.6 s, verify 18.7 s (reps within 0.1%); strip 5.7 s per party;
combine over two partials 7.9 s. Against 10⁵/W2 that is 7.5× for 10× the
ballots at half the width — linear in N, with the per-ciphertext costs
(permutation commitments, bridging, transcript) outweighing the per-component
ones. Raw files: `bench-results/ec2-20260923-234858-2d23452f05/`,
`bench-results/ec2-20260923-235827-a3a46a5640/` and
`bench-results/ec2-20260924-005233-196bc5c947/`.

## Design, as implemented

Five operator-facing targets, each a site where the techniques apply:
**shuffle prover** and **verifier** (`vsc::zkp::shuffle`), **partial
decryption** and **combine** (`vsc::dkgd`), and the first mix's **Naor-Yung
verify-and-strip** (`vsc::cryptosystem::naoryung`, driven by
`braid::trustee::mix`). What goes where:

| technique | shuffle prover | shuffle verifier | partial_decrypt | combine | NY strip |
|---|---|---|---|---|---|
| chunked constant-time MSM (secret scalars) | A′, F′ | | | | |
| chunked variable-time MSM (public data) | | A, F, V1, V5, V2 | a, b | a, b; Lagrange F_j | the batched PlEq check (size 4WN) |
| fixed-base batch (`exp_many`) | uᵢ, re-encryption legs, bridging B_i, B′ | | | | |
| closed-form bridging chain | ✓ | | | | |
| small-exponent batching (verifier-local weights) | | V2 | | | all N well-formedness proofs |
| parallel serialization (transcripts `par_ser`; message lists `Vec<T>`) | seed, posting | seed, reading | seed, posting | seed (×T), reading | reading |
| rayon per-element loops | e_n, generators | generators | factors u^{xᵢ} | per ciphertext | challenges vᵢ, strip |

Read the columns and the Status table follows: `combine` and `partial_decrypt`
collect the most entries; the strip column was filled last (the batched check
of `009b443add`), which is why ⑤ was flat at the milestone.

### Chunked multi-exponentiation, constant-time and variable-time

Products `∏ bases_i^{e_i}` run through a small seam on the group traits:
`GroupElement::multi_exp` (constant-time, for **secret** scalars),
`GroupElement::vartime_multi_exp` (variable-time, **public data only**), and
their componentwise counterparts `DistGroupOps::dist_multi_exp` /
`dist_vartime_multi_exp` for width-`W` ciphertext columns. P-256 and the
product group inherit naive defaults; ristretto255 overrides all of them
(`groups/ristretto255/element.rs`).

**Chunked, not a single dalek call.** curve25519-dalek's `multiscalar_mul` /
`vartime_multiscalar_mul` are single-threaded, and a single call *loses* to the
naive `map(exp).reduce(mul)` product already spread over the rayon pool. The
overrides split the input into `num_threads` chunks (floor 64), run one dalek
MSM per chunk on the pool and sum the partials. Chunk boundaries depend only on
input length, never on the scalars, so the constant-time path stays
constant-time. On the reference machine at N = 10⁵ (`benches/msm_strategy.rs`),
relative to the naive parallel product: a single constant-time call is **4.3×
slower**, chunked constant-time 2.6× faster, chunked variable-time **8.0×**
faster (47.8 ms).

**Stateless by necessity: no prepared bases, no offline precomputation.** The
independent generators `h` are derived per mix from the input ciphertext list
(PROTOCOL.md §2.5/§6.2; `braid::trustee::mix` re-derives them for every link).
Within one verification every base vector is used in exactly one MSM, and
across mixes the bases differ — so there is nothing for a precomputed table to
amortize across calls, and commitments cannot be computed before the ballots
exist. That is why the seam is plain methods on the group traits rather than
a stateful backend object.

**Constant-time vs variable-time is a per-call-site property, not a mode.**
The prover's MSMs consume the secret blinders `ε`, `β` — leaking them damages
zero-knowledge — so `A′ = g^α·∏ hᵢ^{εᵢ}` and `F′` over the 2W output columns
use the constant-time variants. The verifier's `A = ∏ uᵢ^{eᵢ}`, `F`, V1's
`∏ hᵢ^{k_Eᵢ}`, V5 and the batched V2 consume only hash-derived batching values
and published responses, so they use the variable-time variants. In
decryption the same rule gives: the batched-statement rebuild `A = ∏ uᵢ^{eᵢ}`,
`B = ∏ fᵢ^{eᵢ}` is public in both `partial_decrypt` and `combine`
(variable-time), and `combine`'s Lagrange combination `F_j = ∏_i f_{i,j}^{λ_i}`
is a size-`T` variable-time multi-exp per ciphertext over published factors;
only the trustee's own factors `u_j^{x_i}` involve the secret share and stay
constant-time. Every call site is annotated with why its timing class is
correct; getting it wrong in either direction is a bug.

### Fixed-base batches

`GroupElement::exp_many` computes `self^{s_i}` for many `s_i` from one
precomputed basepoint table, constant-time per multiply, in parallel. The
prover's bridging commitments are two such batches (below). The adjacent
prover products with the same shape — the Pedersen commitments `uᵢ = g^{rᵢ}·hᵢ`
and the re-encryption legs `(g^s, y^s)` — are fixed-base work and must never be
routed through `multi_exp`; they are not yet batched (Remaining levers).

### Closed-form bridging chain

The recurrence `B₀ = h₁, Bᵢ = g^{bᵢ}·B_{i−1}^{e′ᵢ}` is inherently serial — the
one unparallelizable stretch of the prover. `bridging_commitments` computes the
equivalent closed form `Bᵢ = g^{dᵢ}·h₁^{pᵢ}` as two `exp_many` batches, with

```
dᵢ = bᵢ + e′ᵢ·d_{i−1}   (d₁ = b₁)   -- also the Step-4 response d = d_N
pᵢ = ∏_{k≤i} e′_k        (p₁ = e′₁)   -- prefix products; reused for B′
```

from two cheap sequential scalar scans; `B′` has its own closed form on `p`.
Bit-identity with the recurrence is pinned by `test_bridging_closed_form_*`
(N ∈ {1,2,5,10,65}) and by V2/V4, which uniquely determine `B`/`B′` given the
rest, so a passing roundtrip implies byte-identical commitments.

### Small-exponent batching: V2 and the ballot proofs

Two sites check `N` independent equations over public data with the same
fixed bases, and both collapse to one random-weighted check
(Bellare–Garay–Rabin small-exponent batching) whose weights are the
**verifier's own randomness, drawn after the proofs are fixed** — never
hashed, never in a transcript, so nothing the prover sees changes.

**Shuffle V2.** The `N` per-index checks `Bᵢ^v·B′ᵢ = g^{k_Bᵢ}·B_{i−1}^{k_Eᵢ}`
(with `B₀ = h₁`) become two variable-time multi-exps of sizes 2N and N:

```
∏ Bᵢ^{v·tᵢ} · ∏ B′ᵢ^{tᵢ} == g^{Σ tᵢ·k_Bᵢ} · ∏ B_{i−1}^{tᵢ·k_Eᵢ}
```

No prover coordination, no change to `ShuffleChallenges`, Verificatum interop
untouched. A proof with any failing instance passes with probability exactly
1/q; PROTOCOL.md §6.4 states the batched form as a permitted check with that
bound, keeping the per-index equations normative (PROTOCOL-alignment.md D7).
`test_shuffle_batched_v2_rejects_*` tampers `k_B`, which appears only in V2 and
does not feed the challenge, isolating the batch.

**Naor-Yung well-formedness** (`PlEqProof::verify_batch`, reached through
`naoryung::PublicKey::strip_all`). Each ballot's PlEq proof states, per
component `w`, `g^{k} = A_g·u_b^{v}` and `z^{k} = A_z·u_a^{v}` with its own
challenge `vᵢ`; per item that is `4W` exponentiations. With independent
weights `t`, `s` per `(i, w)` the `2WN` equations become one check:

```
g^{Σ t·k} · z^{Σ s·k} == ∏ A_g^{t} · ∏ u_b^{t·vᵢ} · ∏ A_z^{s} · ∏ u_a^{s·vᵢ}
```

— two fixed-base exponentiations and **one variable-time multi-exp of size
4WN**, so the exponentiation cost of the whole list is one MSM. What is
hashed does not change: every `vᵢ` is recomputed by the same
`challenge_input`/`hash_to_scalar` as the per-item `verify`, over the same
inputs and tags, and these `N` hashes (in parallel) are the floor the target
cannot go below. On rejection the failing ballots are attributed by per-item
verification; a batch that rejects while every proof verifies individually
cannot happen for correct arithmetic and fails closed
(`Error::BatchVerificationInconsistent`). PROTOCOL.md §3.5 states the form
as permitted with its 1/q bound, `PlEqVerify` normative, referenced from §5.5
and §9.2 step 3 (PROTOCOL-alignment.md D8). Tests: batch == per-item
acceptance across sizes and widths; a tampered response, a swapped pair of
proofs and a foreign context attributed to exactly the per-item failures;
`strip_all` equals per-item `strip`; braid halts the tally on a tampered
ballot as before.

### Parallel serialization: transcripts and messages

Ristretto point compression (one inverse square root per point) and
decompression dominate two kinds of work: the Fiat-Shamir seeds, which hash the
N-element generator, commitment and ciphertext lists, and the posted messages,
which every party encodes once and every reader decodes. Both run on the rayon
pool behind the unchanged encoding (SERIALIZATION.md §2, §5):

- **Lists** — `Vec<T>::write` encodes the elements in parallel from
  `PAR_MIN_ELEMENTS` (1024) up, for any element type; `Vec<T>::read` decodes
  them in parallel when the element width is known, which `Deserializable`'s
  `FIXED_WIDTH` hint states for the group leaves, arrays and structs of them
  (`Ciphertext`, via the derive). The bytes produced and accepted are the
  sequential loops' exactly — pinned against a sequential reference on valid,
  corrupted, truncated, extended and mis-counted lists — and each element
  still goes through the same strict `read`. braid's `Mix`, `Ballots` and
  `PartialDecryption` bodies inherit it with no change.
- **Transcripts** — `par_ser` is that list encoding for a borrowed slice,
  used by the shuffle's `batching_challenges` and second `challenge` (the
  `B_n`/`B′_n` lists, the last sequential site, found by the stage breakdown:
  `6f4c995c22`) and dkgd's `batching_exponents`.

The wire format itself requires only *self-delimitation*, which variable-width
types (`String`, nested `Vec`, `Option`) also satisfy; a known width is the
extra property that makes a list's boundaries computable before decoding.

### Parallel loops, and what deliberately stays serial

Rayon is applied where it earns its overhead, decided by
`benches/parallel_tradeoff.rs` on **absolute wall-clock saved in context**, not
the raw ratio (a 5× speedup on a 10 ms loop is noise on a multi-second
operation). Parallel: every per-element point exponentiation or hash — the
MSM sites, `partial_decrypt`'s factors, `batching_exponents`, `combine`'s
per-ciphertext combination, `ind_generators` (both curves), the shuffle's
`e_n` derivation, and the Naor-Yung batch's challenge hashes and strip
(`strip_all`; braid's own loop around per-item `strip` is gone).
Serial: the scalar loops (`b_n`, `β`/`ε`, `a`, `k_b_n`, `k_e_n`, `e_n_fold`)
and the point-product folds (`u_n_fold`, `h_n_fold`) — each under 0.1% of
prove/verify, so serial is simpler at no measurable cost. Parallelism lives in
the crypto/action layer only: `ascent` is sequential-only (spec §7.8), so the
datalog is not a parallelism site; `rayon` is a non-optional dependency of
`vsc` and `braid`, native and via `wasm-bindgen-rayon` in the browser.

## Constraints and how they are verified

| invariant | enforced / evidenced by |
|---|---|
| **Proofs, transcripts and wire format are unchanged** — the techniques change how values are computed, never what they are | `par_ser` == `Vec::ser` and closed form == recurrence differential tests; shuffle and dkgd round-trips; all 17 model-check configurations; **Verificatum interop 70/70** (`V2V_REQUIRE_VMN=1`) with seed, challenge, decryption transcript and generators byte-identical to `vmnv -t`; PROTOCOL-alignment.md re-verification |
| **Accept/reject is unchanged**, except the documented exactly-1/q of the two batched checks | batched-V2 negative tests and PROTOCOL.md §6.4 (D7); `verify_batch` attribution tests, `strip_all` rejection tests and PROTOCOL.md §3.5 (D8) |
| **Constant-time wherever a secret enters**; variable-time only on public data | no test proves timing: the evidence is the per-site audit below — every exponentiation site, what it raises to, whether that is secret, and the mode it uses — kept current with the code |
| **Parallelism only in the crypto/action layer; identical behaviour on the wasm pool** | production `wasm` (wasm-bindgen-rayon, atomics) and `wasm-core` both compile; headless IndexedDB test, the interactive emulator (full protocol under wasm) and the live-b4 protocol tests pass |
| **`vsc` lint posture** (`unsafe_code = forbid`; `unwrap_used`, `panic`, `arithmetic_side_effects`, pedantic/complexity denied) | CI clippy with `-D warnings`; new curve arithmetic and indexing carry justified, localized `#[allow]`s |
| **CI gates** — `fmt -- --check`, workspace clippy (`--all-targets -D warnings`), vsc clippy, `cargo test --release --features sqlite,postgres`, the wasm-core build | all green on the parking-milestone tree (2026-09-24, `9fdae3ec79`+): fmt, both clippy gates (vsc's after `22eea703bd`), every workspace test suite, the wasm-core wasm32 build; **Verificatum interop 70/70** with `V2V_REQUIRE_VMN=1`. Still to run on that tree: the headless wasm test and the production wasm build (`test-wasm.ps1`, `build-wasm.ps1`), the live-b4 protocol tests and the interactive emulator (TESTING.md), and the fuzz smoke baseline with the fuzz-aware threshold (ASSURANCE.md §3) |

### Constant time or variable time, per site

**The decision (2026-09-24, revisited with the measured breakdown):
constant time wherever the exponent is a secret, variable time wherever every
operand is public.** Nothing in the protocol text speaks to timing; this is an
implementation posture, and it costs something measurable: after the
fixed-base batches, the prover's two constant-time multi-exponentiations
(`A′`, `F′`) are ~27–30% of prove, and a variable-time multi-exponentiation
runs about three times faster (`msm_strategy`), so going variable-time there
would save roughly a fifth of prove and a tenth of T. It is not taken. The
exponents in question are the prover's blinders `ε`: the response
`k_E = v·e′ + ε` hides the permuted challenge `e′` behind them, so a timing
channel on `ε` is a channel on the permutation — the one secret a mixnet
exists to keep. A co-tenant or same-host observer is a realistic adversary for
a trustee running in a cloud, and the library's constant-time primitives are
exactly the defence the deployment would otherwise have to argue away. The
fixed-base batches are constant-time at no extra cost (the table multiply is),
so the prover's remaining constant-time premium is those two MSMs, and it is
accepted.

The audit. "Secret" means the exponent is a value the protocol keeps private;
"public" means every operand is published or hash-derived from published
values, so timing can reveal nothing an observer does not already have.

| site | raises | exponent | mode |
|---|---|---|---|
| shuffle prover: `uᵢ = g^{rᵢ}·h` (`apply_permutation`) | `g` | `rᵢ` commitment randomness — secret | constant-time fixed-base (`exp_many`) |
| shuffle prover: re-encryption `(g^{sᵢ}, y^{sᵢ})` | `g`, `y` | `sᵢ` re-encryption randomness — secret | constant-time fixed-base (`exp_many`) |
| shuffle prover: bridging `Bᵢ = g^{dᵢ} h₁^{pᵢ}`, `B′ᵢ` | `g`, `h₁` | `d`, `p`, `β`, `ε` — secret | constant-time fixed-base (`exp_many`) |
| shuffle prover: `A′ = g^α ∏ hᵢ^{εᵢ}` | `g`, `hᵢ` | `α`, `ε` — secret | constant-time: `g_exp`, `multi_exp` |
| shuffle prover: `F′ = Enc(1; −φ) ∏ w′ᵢ^{εᵢ}` | `w′ᵢ`, `g`, `y` | `ε`, `φ` — secret | constant-time: `dist_multi_exp`, `repl_exp` |
| shuffle prover: `C′ = g^γ`, `D′ = g^δ` | `g` | `γ`, `δ` — secret | constant-time `exp` |
| shuffle verifier: `A`, `F`, `h^{k_E}`, V2 (both sides), V5 | published lists and commitments | `e`, `v`, `k_*`, verifier-local `t` — public | variable-time MSM (`vartime_multi_exp`, `dist_vartime_multi_exp`) |
| shuffle verifier: the nine single exponentiations (`A^v`, `g^{k_A}`, `h₁^{∏e}`, `g^{Σtk_B}`, `C^v`, `g^{k_C}`, `D^v`, `g^{k_D}`, `(g,y)^{−k_F}`) | public | public | constant-time `exp` — harmless and microseconds; not worth a variable-time variant |
| decryption: factors `uᵢ^{x}` (`partial_decrypt`) | `uᵢ` (varying) | `x` the key share — secret | constant-time per-element `dist_exp`; no fixed base, no batch: inherent |
| decryption: the batched statement `a`, `b` (`partial_decrypt`, `combine`) | published `u`, factors | hash-derived `e` — public | variable-time `dist_vartime_multi_exp` |
| decryption: Lagrange `F_j = ∏ f_{i,j}^{λᵢ}` (`combine`) | published factors | `λ` — public | variable-time `dist_vartime_multi_exp` (size T, per ciphertext) |
| decryption proof (`DlogEqProof`): prove / verify | `g`, `u` | prove: blinder — secret; verify: public | constant-time `exp`/`dist_exp` both (verify could be variable-time; it is a handful of exponentiations) |
| Naor-Yung: encrypt, PlEq prove | `g`, `y`, `z` | `r`, `a` — secret | constant-time `repl_exp` |
| Naor-Yung: `verify_batch` | published ballots and proofs | `v`, verifier-local weights — public | variable-time MSM (size 4WN) |
| Naor-Yung: per-item `verify` (the attribution fallback) | published | public | constant-time `repl_exp`/`dist_exp` — harmless; only runs on a rejected list |
| DKG: shares, verification keys (`Dealer`, `Recipient`) | `g` | polynomial coefficients, shares — secret | constant-time `exp` |
| DKG: share verification (`checking_values.exp`) | published commitments | public | constant-time `exp` — P·T exponentiations, harmless |
| generators (`ind_generators`) | — | hash-to-point, no exponent | — |

Every variable-time site consumes only published or hash-derived values, and
every secret exponent is on a constant-time path. Three public sites use
constant-time exponentiation where variable-time would be admissible (the
verifier's single exponentiations, the decryption proof's verify, share
verification); each is a handful of exponentiations and stays as is.

## Remaining levers

In priority order; nothing here is done.

1. **Transcript bytes reused, not recompressed.** Every Fiat–Shamir input is
   the canonical `ser` encoding of group elements (SERIALIZATION.md §6), and
   compressing a point costs a field inversion. The verifier already holds
   those bytes: the posted lists and proofs *are* that encoding, so it decodes
   them to points and then, inside `verify`, recompresses the same points for
   the transcript — measured 32–34% of verify (Status, "transcript ser":
   0.49 s of 1.52 s at W2). The prover compresses its output list and
   commitments twice, once for the transcript and once for posting — 17–18%
   of prove is transcript serialization, about half of it this. Carrying the canonical bytes
   alongside decoded values (a `deser` that returns both, a transcript builder
   that takes bytes when offered, one compression shared with message
   serialization on the prover) removes that work **without changing a single
   hashed byte**. Two conditions become load-bearing, both true today:
   decoding is strict — a non-canonical encoding is rejected, so received
   bytes ≡ re-serialized bytes, which turns from a format property into a
   soundness requirement once relied upon — and the transcript and message
   framings of a list are byte-identical or sliceable. About 0.7 s per mix,
   ~2 s of T(3) = 18.8 s (~11%); the byte-carrying `deser` it needs is the
   plumbing the parallel lists already have. Deferred at the parking
   milestone, by decision. Native transcripts only: the Verificatum-compatible
   challenge derivation encodes ByteTree through `VmnChallenges`
   (SERIALIZATION.md §6) and keeps its own serialization.
2. **`jemalloc`** is available behind a `braid` feature as a higher-performance
   allocator and profiling aid; not wired into the runtime.
3. **Marked unoptimized paths.** `--features custom-warnings` surfaces the
   `#[crate::warning("…")]` annotations on known-unoptimized code as compiler
   warnings — the in-code map of what is left.
4. **GPU — assessed, not justified.** Rule: adopt a GPU MSM only if MSM holds
   ≥ 70% of verifier wall-clock at the deployment's real N *and* a latency
   requirement CPU scaling cannot meet exists. On the reference machine at
   10⁵/W2 the verifier's MSMs are a measured 41% of verify and 30% of a
   whole mix (Status, "Where the time goes"), so even a free GPU MSM buys
   ≤ ~1.7× on verify and ≤ ~1.4× on a mix, and the prover's share is
   constant-time by decision (Constraints). If it is ever revisited: Anza's
   `curve25519-cuda` (sppark-based, in `anza-xyz/cryptography`) is the one
   candidate for this curve — variable-time, GPU→CPU fallback — but unpublished
   and unaudited as of 2026-09; the posture would be GPU on the verifier only,
   CPU prover (secret `ε` never reaches VRAM), feature-gated with silent CPU
   fallback, CPU path normative for Verificatum interop. The EC2 *G and VT*
   quota is granted, so a feasibility session is possible.
5. **Two scheduling levers, measurable only on the global target — recorded,
   not decided.** The global target (`examples/tally.rs`, Tooling and method;
   its numbers in Status) is the **critical-path latency of one tally** for a
   quorum of Q, each party acting in turn and concurrent work off the path:

   ```
   T(Q) = 2·Strip + Q·(Prove + Verify) + PartialDecrypt + Q·PartialDecryptVerify
   V(Q) =   Strip + Q·Verify                            + Q·PartialDecryptVerify   (external verifier)
   ```

   The first mix costs its producer Strip + Prove and its verifier Strip +
   Verify (the verifier recomputes L₀ itself); every later mix adds Prove +
   Verify; the partials are computed concurrently but verified in series by
   whoever combines. It is a replay of braid's *current* schedule over vsc's
   primitives, so a scheduling change in braid changes the composition with
   it. Two levers change no single target and register only on T(Q):
   - **Eager strip.** The trustee verifying the first mix strips the ballot
     list only when that mix arrives (`mix_input_ciphertexts`), which is the
     second Strip on the path; stripping when the ballots arrive removes it
     (~1.0 s, ~5% of T(3) = 18.8 s — batching already took most of this
     lever's value).
   - **Eager partial-decryption verification.** `ComputePlaintexts` runs
     `combine` once all N partials are posted, and `combine` verifies them in
     series (`recipient.rs`, the contribution loop); verifying each on
     arrival, as mixes are, takes N − 1 verifications off the path (~1.2 s,
     ~6% of T(3) = 18.8 s, growing with N).

   Both are braid datalog/action changes with no protocol or transcript
   consequence; whether either is worth its complexity is undecided and will
   be judged on T(N). The replay is a model of braid's schedule over vsc's
   primitives, so when a scheduling lever lands the composition changes with
   it; braid's in-process protocol test at scale would be the empirical
   cross-check, with the caveat that it puts every trustee's work on one box
   and so measures total work, not per-party latency.

Not levers: `ind_generators` (~110 ms at 10⁵, 1–2% of prove/verify, folded
into those targets); `combine`'s per-contribution re-serialization dedup
(~2–4% after `par_ser`); 128-bit `e_n` (halves the MSM window count but alters
the Terelius–Wikström soundness argument — a protocol decision, out of scope).

## Tooling and method

**Optimize from benchmarks, not speculation** — every decision above is backed
by a measurement. Single-shot cross-run comparison is unreliable: sustained
load warms a laptop and inflates later runs (an unchanged control drifted +43%
between two back-to-back sweeps). So a change is measured by running the
before and after binaries **interleaved** within one session, taking the
**median** of reps, and — for authoritative numbers — on the **reference
machine**, which is quiet by construction and the same hardware every time;
`SUMMARY.md` reports the snapshot-vs-interleaved spread of the same binary as
the resolution of the run. The criterion benches self-calibrate with warmup and
sampling.

Three layers:

| Tool | Layer | What it measures |
|---|---|---|
| `benches/msm_strategy.rs` | guidance | naive-parallel vs single/chunked dalek MSM, constant-time and variable-time; selects the override shape |
| `benches/parallel_tradeoff.rs` | guidance | serial vs parallel for each per-element loop shape; decides where rayon earns its keep |
| `benches/shuffle.rs` | guidance | fixed N = 100 / W = 3 prove/verify micro-benchmark; nightly-only libtest harness |
| `vsc --features profile` (`utils/profile.rs`) | breakdown | wall-clock per cost category — constant-time MSM, variable-time MSM, fixed-base batches, per-element exponentiations, transcript serialization, hashing, generators — accumulated at the stages' outer, sequential call sites (no nested regions, never a sum over workers); the identity when the feature is off, so the production build carries no timers; read through `targets.rs`, which prints each target's breakdown and the unattributed remainder, and the rig's `BREAKDOWN=1`. The call sites were removed after the 2026-09-24 measurement so they do not clutter the crypto code; measuring again means wrapping the outer call sites listed in the per-site audit (Constraints) in `timed` |
| `examples/targets.rs` | snapshot | one `(N, W)` cell of the five targets in production form — shuffle prove and verify (both incl. `ind_generators`), `partial_decrypt`, `combine`, Naor-Yung verify-and-strip — as a CSV line, in production form — so it uses `strip_all` and builds against commits at or after `009b443add`; the fork-point differential is on record from `185dbbede2` (whose `targets.rs` built against the fork-point API), and older baselines chain through it; T = 3, P = 5 |
| `examples/tally.rs` | global | the **global target**: one tally's critical-path latency for a quorum of Q, replayed with real data flow — both strips of the first mix, Q rounds of prove and verify, the slowest partial decryption, combine — each stage timed and composed into `T(Q)` and the external verifier's `V(Q)`, with the stage shares on stderr; `--ser` adds the encoding/decoding of each posted message (off by default); the plaintexts are checked against the encrypted messages; N, W, Q parameters (`N:W:Q` cells) |
| `bench.ps1` / `bench.sh` | local | turnkey local run: build untimed, then the guidance benches (`GUIDANCE`, default on), the whole targets grid (`CELLS`, `REPS`) and the tally grid (`TALLY_CELLS`) to a timestamped `bench-results/` file |
| `bench-ec2.sh` + `bench-ec2/remote-bench.sh` | reference | the snapshot grid — and, given a baseline commit, an interleaved before/after — on a temporary EC2 instance of a fixed type (BENCH-EC2.md), which cannot outlive the session; the remote script owns its grid loops — targets, `TALLY_CELLS`, `TALLY_SER_CELLS` (with `--ser`), and with a baseline the interleaved tally before/after over `TALLY_DIFF_CELLS` — and with `BREAKDOWN=1` writes the stage breakdown from a separate `--features profile` build; `collect` renders `SUMMARY.md` (`bench-ec2/summarize.sh`) beside the raw files. The authoritative layer |

## Log

How the above came to be, in order. Laptop numbers are interleaved A/B on a
Windows x64 / 16-thread / AVX2 machine and are directional; the
reference-machine numbers in Status supersede them as the record of where
things stand.

- **Assessment of the original MSM note.** It proposed an `MsmBackend` object
  holding prepared bases, and assumed a serial baseline; both premises were
  wrong — generators are derived per mix (nothing to amortize), and the
  baseline was already the parallel naive product, which a single dalek call
  loses to. The design collapsed to the stateless seam of methods above. GPU
  deferred behind a go/no-go rule.
- **Stage 0 — rayon completeness.** Parallelized the under-parallelized sites:
  `partial_decrypt` factors, `batching_exponents`, `combine`'s Lagrange step,
  P-256 `ind_generators`, and braid's first-mix Naor-Yung strip loop (5.9× at
  10⁵). Removed parallelism where `parallel_tradeoff` showed no absolute gain
  (the scalar loops and point folds), with the policy recorded above.
- **Stage 1 — chunked MSM primitives** (`multi_exp`, `vartime_multi_exp`,
  `exp_many`, `dist_vartime_multi_exp`). Laptop `msm_strategy` at 10⁵ vs the
  naive product: single constant-time 2.3× *slower*, single variable-time
  1.6×, chunked constant-time 2.5×, chunked variable-time 8.2×.
- **Stage 2 — shuffle** (`88af78733e` verifier vartime MSMs, `6b51e641c2`
  batched V2, closed-form bridging and `A′`/`B′`/`F′` via MSM and `exp_many`).
  The old `fold_values`/`bounded-combine` seam — two strategies trading stack
  growth against materialization for large point folds — became moot once the
  point-valued folds were MSMs; only the scalar `f` fold survives, serial, and
  the feature was removed. Laptop controlled run vs the pre-optimization tree
  at 10⁵/W2: verify ~1.9×, prove ~1.5×.
- **The Amdahl finding.** The MSM primitive was 8× faster yet verify only
  ~1.9×: MSM had become a minority of wall-clock and **serialization** — point
  compression inside the Fiat-Shamir seed derivations — dominated (~65% of
  verify); `combine` re-serialized the ciphertext list once per contribution.
- **Serialization** (`e01343dbaf` `FixedWidth` — since replaced by the `FIXED_WIDTH` hint — + `par_ser`; `2e3407606f`
  wired into both transcript sites). Laptop, interleaved pre/post at 10⁵/W2:
  verify 2.8×, prove 1.9×, `partial_decrypt` 2.0×, `combine` 2.0×, strip flat
  (the control). The per-contribution dedup measured 2–4% afterwards and was
  dropped.
- **Decryption vartime** (`0fa48cab26`). `combine`'s `a`/`b` and Lagrange
  combination and `partial_decrypt`'s `a`/`b` moved to variable-time — public
  data that had been on the constant-time path. Laptop, interleaved: `combine`
  2.4× (5.96 → 2.53 s), `partial_decrypt` 1.3×.
- **Tooling** (`dba5a1d84e`). `examples/targets.rs` replaced the two scaling
  tools; `bench.ps1`/`bench.sh` rewired; the fork-point differential method
  (sparse worktree at the fork, same `targets.rs` on both trees). Laptop
  campaign differential after the MSM stages: verify 1.9×, prove 1.5×,
  `partial_decrypt` 4.5×, `combine` 3.7×.
- **Protocol alignment re-verified** (`040356605f`, 2026-09-21). All certified
  items preserved at the value, transcript and byte level; one new
  description-precision item, D7: batched V2 documented in PROTOCOL.md §6.4 and
  referenced from §9.2, per-index equations kept normative. 128-bit `e_n`
  dropped as a soundness matter.
- **Regression verification of the milestone** (2026-09-21, `418cabba92`,
  `59f055e3ed`, `4c70b6bcd9`). Every CI gate with CI's exact commands (fmt,
  workspace clippy `-D warnings --all-targets`, vsc clippy, the
  `sqlite,postgres` workspace test across all five crates, wasm-core build);
  Verificatum interop 70/70 with `V2V_REQUIRE_VMN=1`; both wasm builds; the
  headless IndexedDB test, the live-b4 protocol tests and the interactive
  emulator (DKG → tally → verify plaintexts → second tally → reconnect). Two
  environment traps recorded in TESTING.md (`cargo test -p b4` mis-invocation;
  the Windows `VMN_JAVA` shim). Merged into `feat/meta-13385/main` at
  `185dbbede2`.
- **EC2 reference rig** (`237a2d6004`, 2026-09-22; BENCH-EC2.md). The smoke
  lifecycle caught three defects before any real spend: `read` at EOF under
  `set -e` (`4e48cb147d`), CRLF from `git archive` on Windows for commits
  predating `.gitattributes` (`aa0a387d93`), a rustup download flake in
  user-data (`4d5cdde28c`). **Authoritative run** `185dbbede2` vs `657cb05c20`
  on `c7i.4xlarge`, 45 min, ~$0.60 (`ad700d64bf`) — the Status tables. GPU rule
  evaluated against those numbers: not met (`76cacb49c2`). `SUMMARY.md`
  renderer (`c58e9eb5d1`, `2dfa570a20`); the remote script made to own its
  grids after the milestone's packaged `bench.sh` ignored `GUIDANCE`
  (`762fe5d24f`).
- **Naor-Yung batching** (`009b443add`, 2026-09-23). `PlEqProof::verify_batch`
  behind `naoryung::PublicKey::strip_all`: the first mix's N well-formedness
  proofs checked as one random linear combination with verifier-local weights
  (exactly 1/q), nothing hashed changed; landed in braid's
  `mix_input_ciphertexts`; PROTOCOL.md §3.5 permission note, alignment D8.
  Reference machine, interleaved vs the milestone
  (`ec2-20260923-155859-009b443add`, 13 min): strip 3.8× at W2, 4.0× at W5,
  the other four targets unchanged within 0.5%; first-mix verification
  (strip + verify) 1.8×/2.1×, production (strip + prove) 1.5×/1.6× — the
  Status tables. `targets.rs` follows production form and needs `strip_all`
  from this commit on (Tooling and method).
- **The global target** (`2d23452f05`, 2026-09-23). `examples/tally.rs`
  replays one tally — both strips of the first mix, Q rounds of prove and
  verify, the slowest partial, combine — and composes the stage times into
  the critical-path latency `T(Q)` and the verifier's `V(Q)`; `TALLY_CELLS`
  grids in the remote and local scripts, `SUMMARY.md` renders T and V with
  stage shares. First measurement (`ec2-20260923-234858-2d23452f05`, 7 min):
  T(3) = 27.2 s at W2 and 50.5 s at W5, matching the formula over the
  isolated targets within 0.6% — the Status table. A second session
  (`ec2-20260923-235827-a3a46a5640`, 7 min; the driver had not been
  forwarding `TALLY_CELLS`, fixed in `a3a46a5640`) measured Q = 5 and 7 at
  W2: linear in Q as the formula says (`T ≈ 2.8 + 6.4·Q` s on that host), but
  its host ran ~20% faster than the previous two sessions' on every stage —
  the first observed **host variance** on the rig, recorded in Status and
  BENCH-EC2.md: interleaved within a session is the only exact comparison.
  A third session (`ec2-20260924-005233-196bc5c947`, 12 min) added the
  historical comparison cell 10⁶ : 1 : 2 — T = 138 s, V = 51 s — with the
  10⁵ : 2 : 3 anchor reproducing 27.1 s, host at reference speed.
- **Task 1 of the parking milestone: measurement before levers**
  (2026-09-24). The tally before/after (`TALLY_DIFF_CELLS`), `--ser` cells
  and a `profile` feature (`233f484655`: `vsc::utils::profile`, wall-clock
  per cost category at the stages' outer call sites, printed by `targets`,
  `BREAKDOWN=1` on the rig — after a first session lost it to a knob named
  like the driver's AWS profile, `3e5c25e86c`). The breakdown's first result
  was a fix: the shuffle's second challenge still encoded `B_n`/`B′_n`
  sequentially (`6f4c995c22`, `par_ser`, byte-identical) — verify 1.48×,
  prove 1.17× at 10⁵/W2 on the rig, T(3) 1.20×
  (`ec2-20260924-015429-6f4c995c22`). Its second result re-ranked the
  levers: `--ser` put message encoding/decoding at 64% of the critical path,
  and the measured shares (`ec2-20260924-021429-3e5c25e86c`, a fast host,
  shares only) put the prover's unbatched exponentiations at 42–48% of prove
  and transcript serialization at a third of verify — the Status tables.
- **Parallel lists** (`0ae4a48ea3`, 2026-09-24; lever 1 of the parking
  milestone). `Vec<T>` encodes in parallel from 1024 elements for any element
  type and decodes in parallel when the element width is known — the
  `FIXED_WIDTH` hint on `Deserializable`, summed by the derive, replacing the
  `FixedWidth` trait; `par_ser` became the same encoder for a slice. Bytes
  produced and accepted unchanged, pinned against a sequential reference on
  valid, corrupted, truncated, extended and mis-counted lists; braid's
  message bodies inherit it untouched. Reference machine, interleaved
  (`ec2-20260924-025652-0ae4a48ea3`): message encoding/decoding 36.9 s →
  3.7 s (10.0×), T with messages 56.9 s → 23.7 s (2.40×), crypto stages flat.
- **Prover fixed-base batches** (`cae05fe816`, 2026-09-24; lever 2). The
  permutation commitments and both re-encryption legs through `exp_many`,
  constant-time, pinned to the per-element definition. Reference machine,
  interleaved (`ec2-20260924-031018-81c3a768a5`): prove 4.22 → 3.05 s (1.38×)
  at W2, 8.13 → 5.53 s (1.47×) at W5; T(3) 22.57 → 18.83 s (1.20×); the
  breakdown after both levers in Status.
- **Parking milestone** (2026-09-24). Constant-time decision and per-site
  audit written (`81c3a768a5`); the 35 breakdown call sites removed from the
  crypto code, module and feature kept (`2ad569ecae`); PROTOCOL.md's permitted
  batched verifications moved to Appendix C, main text restored to the
  unoptimized description (`b0a5cc5de2`); implementation re-verified against
  the description and the description against its reference (`66faaab0af`,
  `7c614d2dd2`). Over the day, T(3) at 10⁵/W2 went 27.1 → 18.8 s without
  message handling and 56.9 → 23.7 s with it (before the last lever).
  Transcript-bytes reuse is the one measured lever left on the table.
  Two scheduling levers (eager strip, eager partial verification) recorded
  under Remaining levers as measurable only on the global target, undecided.
