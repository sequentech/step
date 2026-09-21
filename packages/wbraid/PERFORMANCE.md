<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# Performance — braid v0.6

The performance record for the shuffle and threshold-decryption cryptography
(`vsc`) and its `braid` callers: the design and rationale of the
multi-exponentiation and parallelism work, the constraints that bind it, the
measured results, and the levers that remain. Performance is a first-class
*future* concern for v0.6 (large ballot sets, browser-hosted trustees), which
prioritized correctness and clarity; this document is where the optimization
work and its measurements live.

## Ground rules

- **Optimize from benchmarks, not speculation.** Every decision below is
  backed by a measurement in the benchmark inventory.
- **Parallelism lives in the crypto/action layer.** `ascent` runs
  sequential-only (spec §7.8), so the datalog is not a parallelism site; rayon
  is used natively and `wasm-bindgen-rayon` in the browser. `rayon` is a
  non-optional dependency of both `vsc` and `braid` — it is in every build's
  tree regardless.
- **`jemalloc`** is available behind a `braid` feature as a
  higher-performance allocator and profiling tool; not yet wired into the
  runtime.
- **`--features custom-warnings`** surfaces the `#[crate::warning("…")]`
  annotations on known-unoptimized paths as compiler warnings.

## 1. Multi-exponentiation: the design

The shuffle's dominant cost is multi-exponentiation — products `∏ bases_i^{e_i}`.
These run through a small seam on the group traits rather than a backend object.

### The seam

- `GroupElement::multi_exp` — constant-time, for **secret** scalars.
- `GroupElement::vartime_multi_exp` — variable-time, **public scalars only**.
- `GroupElement::exp_many` — fixed-base batch (`self^{s_i}` for many `s_i`),
  constant-time.
- `DistGroupOps::dist_multi_exp` / `dist_vartime_multi_exp` — the broadcast,
  componentwise counterparts for width-`W` ciphertext columns.

P-256 and the product group inherit naive defaults; ristretto255 overrides all
of them (`groups/ristretto255/element.rs`).

**Why a seam of methods and not a `MsmBackend` object with prepared bases.**
The independent generators `h` are derived per mix from the input ciphertext
list (PROTOCOL.md §2.5/§6.2; `braid::trustee::mix` re-derives them for every
link). Within one verification every base vector is used in exactly one MSM,
and across mixes the bases differ — so there is nothing for a "prepared bases"
abstraction to amortize. The design therefore collapses to a constant-time /
variable-time pair on the existing trait, plus a fixed-base batch. (An
offline/online split that precomputes commitments before ballots arrive is
impossible for the same reason: the commitments depend on `h`, which depends
on the ballots.)

### Constant-time vs variable-time is a per-call-site property

Not a global mode. The prover's MSMs consume the secret blinding scalars
(`epsilon`, `beta`) — leaking them damages zero-knowledge — so they use the
constant-time `multi_exp` / `exp_many` / `dist_multi_exp`. The verifier's MSMs
consume only public data (hash-derived batching values, published responses),
so they use the variable-time variants, which are faster. Getting this
backwards in either direction is a real bug; every call site is annotated with
why its timing class is correct.

### Chunked, not single — the load-bearing choice

curve25519-dalek's `multiscalar_mul` / `vartime_multiscalar_mul` are
single-threaded. A single call **loses** to the current `map(exp).reduce(mul)`
product, which is already spread across the rayon pool. The overrides instead
split the input into `num_threads` chunks (floor 64), run one dalek MSM per
chunk on the pool, and sum the partials. Chunk boundaries depend only on input
length, never on the scalars, so the constant-time path stays constant-time.
Measured at N = 10⁵ vs the naive parallel product (`benches/msm_strategy.rs`):

| strategy | vs naive |
|---|---|
| single constant-time call | **2.3× slower** |
| single variable-time call | 1.6× |
| chunked constant-time | 2.5× |
| chunked variable-time | **8.2×** |

`chunk_t` (= thread count) is best for the variable-time path; `chunk_4t` is
~9% better for constant-time at large N but not worth diverging for.

### Where MSM applies in the shuffle

Prover (`shuffle_with`): `A′ = g^α·∏ hᵢ^{εᵢ}` (secret ε), `F′` over the 2W
output columns (secret ε). Verifier (`verify_with`): `big_a = ∏ uᵢ^{eᵢ}`,
`big_f` over the 2W input columns, V1's `∏ hᵢ^{k_Eᵢ}`, V5 over the 2W output
columns (all public). The adjacent products that are *not* MSMs — the Pedersen
commitments `uᵢ`, re-encryption, `g^{bᵢ}` — are fixed-base batches, served by
`exp_many`, and must not be routed through `multi_exp`.

## 2. The shuffle changes

All of these preserve **bit-identical proofs and accept/reject behaviour**
(except V2's documented batching error): they change how values are computed,
never what they are.

### Verifier

- `big_a`, V1 → `vartime_multi_exp`; `big_f`, V5 → `dist_vartime_multi_exp`
  over the ciphertext columns.
- **Verification 2 is batched.** The N elementwise checks
  `Bᵢ^v·B′ᵢ == g^{k_Bᵢ}·B_{i−1}^{k_Eᵢ}` (with `B₀ = h₁`), formerly 3N
  exponentiations, collapse to one random-weighted check (Bellare–Garay–Rabin
  small-exponent batching):

  ```
  ∏ Bᵢ^{v·tᵢ} · ∏ B′ᵢ^{tᵢ} == g^{Σ tᵢ·k_Bᵢ} · ∏ B_{i−1}^{tᵢ·k_Eᵢ}
  ```

  evaluated as two variable-time multi-exps (sizes 2N and N). The `tᵢ` are the
  **verifier's own randomness, drawn after the proof is fixed** — not part of
  the transcript — so there is no prover coordination, no change to
  `ShuffleChallenges`, and the Verificatum-interop path is unaffected.
  Soundness error ≤ 1/q per failing equation.
  `test_shuffle_batched_v2_rejects_*` pins it by tampering `k_b_n`, which
  appears only in V2 and does not feed the challenge, isolating the batch.

### Prover

- **The bridging chain uses a closed form.** The recurrence
  `B₀ = h₁, Bᵢ = g^{bᵢ}·B_{i−1}^{e′ᵢ}` is inherently serial — the one
  unparallelizable stretch in the prover. `bridging_commitments` computes the
  equivalent closed form `Bᵢ = g^{dᵢ}·h₁^{pᵢ}` as two fixed-base `exp_many`
  batches, fully parallel, with

  ```
  dᵢ = bᵢ + e′ᵢ·d_{i−1}   (d₁ = b₁)   -- the discrete log; also the response d = d_N
  pᵢ = ∏_{k≤i} e′_k        (p₁ = e′₁)   -- prefix products; reused for B′
  ```

  from two cheap sequential scalar scans. `d_N` is the Step-4 response `d`, so
  it is computed once here (the old Step-4 recurrence is gone), and `p` feeds
  `B′`'s own closed form. Bit-identity is guaranteed by the algebra and pinned
  two ways: `test_bridging_closed_form_*` checks closed form == loop for
  N ∈ {1,2,5,10,65}, and V2/V4 uniquely determine `B`/`B′` given the rest, so a
  passing roundtrip implies byte-identical commitments.
- `A′ = g_exp(α)·multi_exp(h, ε)` and `F′` via `dist_multi_exp` over the output
  columns — both constant-time (ε is secret).

### Parallelism policy

Rayon is applied where it earns its overhead and not where it does not, decided
by `benches/parallel_tradeoff.rs` on **absolute wall-clock saved in context**
(not the raw speedup ratio: a 5× speedup on a 10 ms loop is noise on a
multi-second operation). Kept parallel: everything point-exponentiation or
hashing (the MSM sites, `partial_decrypt` factors, `batching_exponents`,
`combine`'s Lagrange step, `ind_generators`, the shuffle's `e_n` derivation —
hashing, ~0.7%). Kept serial: the scalar loops (`b_n`, `beta`/`epsilon`, `a`,
`k_b_n`, `k_e_n`, `e_n_fold`) and the point-product folds (`u_n_fold`,
`h_n_fold`, `combine`'s plaintext extraction) — each under 0.1% of
prove/verify, so serial is simpler for no measurable cost. braid's first-mix
Naor-Yung verify-and-strip loop is parallel (5.9× at N = 10⁵).

### Fold strategy — resolved

The shuffle once routed its width-`W` products through a `fold_values` seam
with a `bounded-combine` cargo feature, whose two strategies (rayon `reduce`
vs. chunked plain loops) traded off stack growth against materialization —
the deep recursion of folding large *point* accumulators (`[[Element;W];2]`,
~32 KB at W = 100) could overflow default thread stacks. The MSM work removed
all three point-valued folds (`A′`, `F′`, `big_f`, V5 are multi-exps now), so
the sole survivor is the scalar `f = Σ(sᵢ⊙eᵢ)` fold, now a plain serial fold
(scalar work, too cheap for rayon). The `bounded-combine` feature and the
`fold_values`/`bounded_combine` seam are removed; the stack hazard they
guarded is gone.

## 3. Constraints that bind this work

- **Bit-identical proofs, no transcript or wire-format change.** The
  `test_shuffle_*` suites and the `v2v`/Verificatum interop (which reproves
  through `VmnChallenges`) require the produced proofs to be unchanged. The
  §2 changes alter how values are computed, not what they are; batched V2 is
  verifier-internal.
- **Constant-time contract** on `multi_exp`/`exp_many`/`dist_multi_exp` for the
  prover's secret scalars.
- **`vsc` lint levels** are strict (`unsafe_code = forbid`; `unwrap_used`,
  `panic`, `arithmetic_side_effects`, pedantic/complexity denied) — new curve
  arithmetic and indexing carry justified, localized `#[allow]`s.
- **wasm**: the chunked MSMs and all parallel sites run on
  `wasm-bindgen-rayon`'s pool exactly as native.

## 4. Measured results (2026-09-21)

Machine: Windows x64, 16 logical cores, dalek AVX2 backend, `--release`. The
current-tree snapshot below was taken quiesced (`bench.ps1`); the campaign
before/after differential was interleaved but agent-run (directional).

**How we measure.** Single-shot cross-run comparison is unreliable here —
sustained load warms the machine and inflates later runs (an *unchanged*
control drifted +43% between two back-to-back sweeps). So a changed site is
measured by interleaving the before/after binaries within one run (shared
conditions per rep), and the current side is cross-checked against the
quiesced snapshot to confirm it is not itself drift-inflated; the criterion
micro-benches (`parallel_tradeoff`, `msm_strategy`) self-calibrate with warmup
and sampling. `bench.ps1` / `bench.sh` run the whole grid under quiescence.

### Campaign before/after — the five targets

The whole effort, measured black-box with `examples/targets.rs` (§6). Because
that tool uses only fork-point public APIs, the same source builds against the
pre-optimization baseline (a sparse `git worktree` at the fork commit), so
baseline and current run interleaved — a valid ratio under shared conditions.
Directional (agent-run, not fully quiesced; the current column agrees with the
quiesced snapshot below). N = 10⁵, W = 2:

| Target | baseline | current | factor |
|---|---|---|---|
| ① shuffle prove | 13.8 s | 9.1 s | **~1.5×** |
| ② shuffle verify | 10.9 s | 5.9 s | **~1.9×** |
| ③ partial decryption | 15.8 s | 3.5 s | **~4.5×** |
| ④ combine | 36.8 s | 10.0 s | **~3.7×** |
| ⑤ Naor-Yung verify-and-strip | 4.4 s | 4.8 s | **~1.0× (flat)** |

Same shape at 10⁴ W = 2 and 10⁵ W = 5. Two things this makes plain:

- **Decryption (③④) is the big win, ~3.5–4×** — it had the most headroom
  (fully serial factor/`batching_exponents`/Lagrange loops *and* single-call
  MSM at baseline, so it caught both stage 0's parallelization and stage 1's
  chunked MSM). At the fork point `combine` was 37 s (130 s at W = 5); it is
  now ~10 s.
- **NY verify-and-strip (⑤) is flat** — the campaign parallelized *braid's*
  first-mix loop, not vsc's `NYStrip` crypto, and `targets` calls that
  primitive with its own `par_iter` (identical on both trees). So ⑤ is the
  one target the campaign never actually optimized: the remaining lever
  (§5 — it is N independent PlEq verifications, not batched).

Current-tree quiesced snapshot (median of 3, for reference; matches the
current column above): shuffle prove/verify 8.5 s / 5.5 s, `partial_decrypt`
3.2 s, `combine` 9.9 s at 10⁵ W = 2. For authoritative before/after, run
`bench.ps1` on both trees under quiescence (build `targets` in a fork-point
worktree, per §6).

### The Amdahl wall — the finding that redirects the next work

The MSM primitive is 8× faster, but end-to-end shuffle **verify improved only
~1.9×** and `combine` sits at ~9.9 s (10⁵ W = 2). MSM is now a *minority* of
wall-clock. The dominant residual is **serialization — ristretto point
compression** (one inverse square root per point), run sequentially inside the
Fiat-Shamir seed derivations (`ser()` of the N-element ciphertext and
commitment lists). In `combine` the batching seed re-serializes the full
ciphertext list once **per contribution** (×T) — the same bytes compressed T
times. The chunked MSMs are only ~1.1 s of `combine`'s 9.9 s.

## 5. Next levers, in priority order

1. **Parallel serialization** (behind the unchanged wire encoding — a `Vec` of
   fixed-size elements has computable boundaries, so chunk/serialize/concat
   needs no format change; see SERIALIZATION.md), plus hoisting `combine`'s
   redundant per-contribution re-serialization. This is where the next factor
   lives.
2. **`ind_generators`** — N ristretto hash-to-curve per prove and per verify,
   already parallel. Measured ~110 ms at N = 10⁵ — ~1–2% of prove/verify, so a
   minor contributor, not a large one; folded into the ① ② target timings
   rather than tracked separately.
3. **Deferred prover fixed-base cleanups** — `apply_permutation`'s
   `uₙ = g^r·h` and the re-encryption `(g^s, y^s)` legs still use per-element
   `exp`/`repl_exp` rather than `exp_many`; part of the 8.5 s prove residual.
4. **GPU** — deferred. The go/no-go rule is: adopt only if, after the residual
   above is fixed, MSM still holds ≥ 70% of verifier wall-clock at the
   deployment's real N *and* a latency requirement CPU scaling cannot meet
   exists. It is currently **not met** — MSM is already a minority. If it ever
   is: Anza's `curve25519-cuda` (sppark-based, in `anza-xyz/cryptography`,
   companion to the `solana-ed25519` dalek fork) is the one candidate GPU MSM
   for this curve — variable-time, with a GPU→CPU fallback — but as of 2026-09
   it is unpublished (crates.io holds a v0.0.0 placeholder) and unaudited, so
   adopting it would be integrate-and-validate against an immature dependency
   in an election verifier's trust chain. The security posture if built: GPU on
   the verifier only (public data), CPU prover (secret ε never reaches VRAM),
   feature-gated with silent CPU fallback, CPU path normative for Verificatum
   interop.
5. **Open question — 128-bit `e_n`.** Shortening the batching challenges from
   full-width to 128 bits would roughly halve the dominant MSM window count
   (~1.6–2× on the whole verifier, and dalek's zero-digit skipping compounds
   it). It is a transcript change (so `NativeChallenges` only, never the
   Verificatum convention, which already uses fixed-bit-length exponents), a
   PROTOCOL.md §2.3/§6.3 edit, and a soundness re-derivation — in Terelius–
   Wikström `e` drives the permutation-matrix argument itself, so the
   Schwartz–Zippel bound becomes ~N/2¹²⁸ and the extraction argument must be
   re-checked. Decide separately; nothing above depends on it.

A PROTOCOL.md §6.4/§9.2 precision note is still to write: a verifier MAY batch
V1–V5 with the stated error bound; the equations as written remain the
normative statement.

## 6. Benchmark inventory

Two layers: *guidance* benches (criterion, sub-primitive, steer implementation)
and the *snapshot* example (the five top-level targets).

| Tool | Layer | What it measures |
|---|---|---|
| `benches/msm_strategy.rs` | guidance | naive-parallel vs single/chunked dalek MSM, constant-time and variable-time; selects the override shape |
| `benches/parallel_tradeoff.rs` | guidance | serial vs parallel for each per-element loop shape; decides where rayon earns its keep |
| `benches/shuffle.rs` | guidance | fixed N = 100 / W = 3 prove/verify micro-benchmark; nightly-only libtest harness |
| `examples/targets.rs` | snapshot | one `(N, W)` cell of the five top-level targets — shuffle prove, shuffle verify (both incl. `ind_generators`), `partial_decrypt`, `combine`, Naor-Yung verify-and-strip — in production form; uses only fork-point public APIs, so it backports for a campaign-wide before/after; T = 3, P = 5 |
| `bench.ps1` / `bench.sh` | — | turnkey controlled run: build untimed, then the whole grid to a timestamped `bench-results/` file |

## Related, tracked elsewhere

- **Incremental fetch (monotonic cursor)** — a pure transport optimization for
  board clients; in the spec §12 with its constraints (never security-relevant,
  cannot certify completeness).
