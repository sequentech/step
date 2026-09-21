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

## 4. Measured results (controlled run, 2026-09-21)

Machine: Windows x64, 16 logical cores, dalek AVX2 backend, `--release`, quiesced.
Times in ms; scaling cells are the median of 3 reps.

**How we measure.** Single-shot cross-run comparison is unreliable here —
sustained load warms the machine and inflates later runs (an *unchanged*
control drifted +43% between two back-to-back sweeps). So a changed site is
measured by interleaving the before/after binaries within one run and taking
the median, with an unchanged column as a thermal-neutral control; the
criterion micro-benches (`parallel_tradeoff`, `msm_strategy`) self-calibrate
with warmup and sampling. `bench.ps1` / `bench.sh` run the whole grid under
quiescence.

**End-to-end shuffle** (all optimizations):

| N | W | prove | verify |
|---|---|---|---|
| 10⁴ | 2 | 868 | 580 |
| 10⁴ | 5 | 1 682 | 1 131 |
| 10⁵ | 2 | 8 506 | 5 461 |
| 10⁵ | 5 | 17 327 | 10 996 |

Against the pre-optimization baseline (~12.9 s prove / ~10.3 s verify at
10⁵ W = 2): **verify ~1.9×, prove ~1.5×.**

**Decryption, plus the first-mix strip** (T = 3, P = 5). `partial_decrypt` and
`combine` are decryption, over ElGamal ciphertexts. The strip columns are *not*
decryption — Naor-Yung verify-and-strip is a first-mix input cost (§2.5/§6.5),
measured here because `decrypt_scaling` builds Naor-Yung ballots and must strip
them to ElGamal to have something to decrypt:

| N | W | strip serial | strip parallel | partial_decrypt | combine |
|---|---|---|---|---|---|
| 10⁴ | 2 | 2 563 | 428 | 314 | 952 |
| 10⁵ | 2 | 25 742 | 4 330 | 3 246 | 9 882 |

The strip columns are a same-run control: parallel strip is 5.9× the serial
loop (the braid first-mix gain).

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
2. **`ind_generators`** — N ristretto hash-to-curve per prove and per verify;
   already parallel, but a large fixed cost.
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

| Tool | What it measures |
|---|---|
| `benches/msm_strategy.rs` | naive-parallel vs single/chunked dalek MSM, constant-time and variable-time; selects the override shape |
| `benches/parallel_tradeoff.rs` | serial vs parallel for each per-element loop shape; decides where rayon earns its keep |
| `examples/shuffle_scaling.rs` | one `(N, W)` cell, prove + verify wall-clock; CSV for sweeps |
| `examples/decrypt_scaling.rs` | one `(N, W)` cell of the tally's per-ballot crypto: threshold decryption (`partial_decrypt`, `combine`) plus the first-mix Naor-Yung verify-and-strip (serial+parallel, a mixing-input cost co-measured here); T = 3, P = 5 |
| `benches/shuffle.rs` | fixed N = 100 / W = 3 prove/verify micro-benchmark; nightly-only libtest harness |
| `bench.ps1` / `bench.sh` | turnkey controlled run: build untimed, then the whole grid to a timestamped `bench-results/` file |

## Related, tracked elsewhere

- **Incremental fetch (monotonic cursor)** — a pure transport optimization for
  board clients; in the spec §12 with its constraints (never security-relevant,
  cannot certify completeness).
