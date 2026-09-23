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

Where the tip (`009b443add`) stands, same machine (median of 3, ms):

| N : W | prove | verify | partial_decrypt | combine | ny_strip |
|---|---|---|---|---|---|
| 10³ : 2 | 61 | 39 | 18 | 37 | 14 |
| 10⁴ : 2 | 503 | 256 | 131 | 231 | 104 |
| 10⁴ : 5 | 892 | 419 | 323 | 565 | 229 |
| 10⁵ : 2 | 5 009 | 2 265 | 1 208 | 2 016 | 988 |
| 10⁵ : 5 | 8 957 | 3 616 | 3 008 | 4 989 | 2 258 |

Resolution: reps agree within ~1%, and the same binary run standalone vs
interleaved agrees within 0.6–0.7% — differences below that are noise. Raw
files: `bench-results/ec2-20260922-012110-185dbbede2/` (fork → milestone) and
`bench-results/ec2-20260923-155859-009b443add/` (milestone → tip), each with
`SUMMARY.md`, the differential CSV, the full grid and `machine.txt`.

Reading it: the **decryption path is the headline** (~13× and ~23×) — it had
the most headroom and every technique below applies to it; the **shuffle**
gains more on verify than prove because the verifier may use variable-time
multi-exponentiation and batch V2 while the prover's remaining cost is
constant-time by necessity; and **⑤ was flat at the milestone because nothing
had touched vsc's `NYStrip` crypto** (only braid's loop around it was
parallelized) — the follow-up batched it, and ⑤ went from the slowest target
at both widths to the fastest at 10⁵.

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
| fixed-base batch (`exp_many`) | bridging B_i, B′ | | | | |
| closed-form bridging chain | ✓ | | | | |
| small-exponent batching (verifier-local weights) | | V2 | | | all N well-formedness proofs |
| parallel transcript serialization (`par_ser`) | seed | seed | seed | seed (×T) | |
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

### Parallel transcript serialization

The Fiat-Shamir seeds hash the N-element generator, commitment and ciphertext
lists, and ristretto point compression (one inverse square root per point)
dominates that work. `FixedWidth` (a `const WIDTH` on the group leaves, arrays
and `Ciphertext`) marks the encodings with computable element boundaries, and
`par_ser` encodes such a slice on the rayon pool, byte-identical to `Vec::ser`
(pinned by `test_par_ser_matches_sequential_*`). It is wired into the shuffle's
`batching_challenges` and dkgd's `batching_exponents`. The wire format itself
requires only *self-delimitation*, which variable-width types (`String`,
nested `Vec`, `Option`) also satisfy on the sequential path; fixed width is the
special case that enables parallel boundaries (SERIALIZATION.md §10).
Deserialization is not parallelized (Remaining levers).

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
| **Accept/reject is unchanged**, except V2's documented 1/q | batched-V2 negative tests; PROTOCOL.md §6.4 permits the batched form (D7) |
| **Constant-time wherever a secret enters** (`ε`, `β`, `bᵢ`; `u^{xᵢ}`); variable-time only on public data | no test proves timing: the evidence is the per-site classification — the trait contract, the call-site annotations, and that every variable-time site consumes only hash-derived or published values |
| **Parallelism only in the crypto/action layer; identical behaviour on the wasm pool** | production `wasm` (wasm-bindgen-rayon, atomics) and `wasm-core` both compile; headless IndexedDB test, the interactive emulator (full protocol under wasm) and the live-b4 protocol tests pass |
| **`vsc` lint posture** (`unsafe_code = forbid`; `unwrap_used`, `panic`, `arithmetic_side_effects`, pedantic/complexity denied) | CI clippy with `-D warnings`; new curve arithmetic and indexing carry justified, localized `#[allow]`s |
| **CI gates** — `fmt -- --check`, workspace clippy (`--all-targets -D warnings`), vsc clippy, `cargo test --release --features sqlite,postgres`, the wasm-core build | all green on the milestone tree (Log, 2026-09-21) |

## Remaining levers

In priority order; nothing here is done.

1. **Prover fixed-base batches.** `apply_permutation`'s `uᵢ = g^{rᵢ}·hᵢ` and
   the re-encryption `(g^s, y^s)` legs still use per-element `exp`/`repl_exp`;
   `exp_many` applies (constant-time). Prove is the least-improved target.
2. **Parallel deserialization** — the other half of SERIALIZATION.md §10, on
   `FixedWidth`'s computable boundaries. A colder site (message loading, not
   the transcript) and safety-sensitive: only if a profile of the loading path
   warrants it.
3. **`jemalloc`** is available behind a `braid` feature as a higher-performance
   allocator and profiling aid; not wired into the runtime.
4. **Marked unoptimized paths.** `--features custom-warnings` surfaces the
   `#[crate::warning("…")]` annotations on known-unoptimized code as compiler
   warnings — the in-code map of what is left.
5. **GPU — assessed, not justified.** Rule: adopt a GPU MSM only if MSM holds
   ≥ 70% of verifier wall-clock at the deployment's real N *and* a latency
   requirement CPU scaling cannot meet exists. On the reference machine at
   10⁵/W2 the verifier's ~dozen MSM-equivalents (~48 ms each) are ~0.6 s of
   2.2 s — a quarter to a third — so even a free GPU MSM buys ≤ ~1.4× on
   verify. If it is ever revisited: Anza's
   `curve25519-cuda` (sppark-based, in `anza-xyz/cryptography`) is the one
   candidate for this curve — variable-time, GPU→CPU fallback — but unpublished
   and unaudited as of 2026-09; the posture would be GPU on the verifier only,
   CPU prover (secret `ε` never reaches VRAM), feature-gated with silent CPU
   fallback, CPU path normative for Verificatum interop. The EC2 *G and VT*
   quota is granted, so a feasibility session is possible.
6. **A global target, then two scheduling levers it would measure — recorded,
   not decided.** The five targets are stages; the sixth measurement is the
   **critical-path latency of one tally** for a quorum of N, each party acting
   in turn and concurrent work off the path:

   ```
   T(N) = 2·Strip + N·(Prove + Verify) + PartialDecrypt + N·PartialDecryptVerify
   V(N) =   Strip + N·Verify                            + N·PartialDecryptVerify   (external verifier)
   ```

   The first mix costs its producer Strip + Prove and its verifier Strip +
   Verify (the verifier recomputes L₀ itself); every later mix adds Prove +
   Verify; the partials are computed concurrently but verified in series by
   whoever combines. To be implemented as a *replay* — the stages run with
   real data flow, N a parameter, each stage timed and the totals composed,
   stage shares printed so the formula is checked rather than assumed; a
   separate example with its own CSV; network out, serialization behind a
   flag that is off. On the tip at N = 3, 10⁵/W2, T ≈ 27 s, of which N·Prove
   is ~55% and strip ~7% (before batching: 32 s and 23%). Two levers change
   no single target and register only on T(N), so they wait for it:
   - **Eager strip.** The trustee verifying the first mix strips the ballot
     list only when that mix arrives (`mix_input_ciphertexts`), which is the
     second Strip on the path; stripping when the ballots arrive removes it
     (~1.0 s, ~4% at N = 3 — batching already took most of this lever's
     value).
   - **Eager partial-decryption verification.** `ComputePlaintexts` runs
     `combine` once all N partials are posted, and `combine` verifies them in
     series (`recipient.rs`, the contribution loop); verifying each on
     arrival, as mixes are, takes N − 1 verifications off the path (~1.2 s,
     ~4% at N = 3, growing with N).

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
| `examples/targets.rs` | snapshot | one `(N, W)` cell of the five targets in production form — shuffle prove and verify (both incl. `ind_generators`), `partial_decrypt`, `combine`, Naor-Yung verify-and-strip — as a CSV line, in production form — so it uses `strip_all` and builds against commits at or after `009b443add`; the fork-point differential is on record from `185dbbede2` (whose `targets.rs` built against the fork-point API), and older baselines chain through it; T = 3, P = 5 |
| `bench.ps1` / `bench.sh` | local | turnkey local run: build untimed, then the guidance benches (`GUIDANCE`, default on) and the whole grid (`CELLS`, `REPS`) to a timestamped `bench-results/` file |
| `bench-ec2.sh` + `bench-ec2/remote-bench.sh` | reference | the snapshot grid — and, given a baseline commit, an interleaved before/after — on a temporary EC2 instance of a fixed type (BENCH-EC2.md), which cannot outlive the session; the remote script owns its grid loops, so it measures any commit that builds `targets`; `collect` renders `SUMMARY.md` (`bench-ec2/summarize.sh`) beside the raw files. The authoritative layer |

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
- **Serialization** (`e01343dbaf` `FixedWidth` + `par_ser`; `2e3407606f`
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
