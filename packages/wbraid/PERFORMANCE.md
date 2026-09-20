<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# Performance — braid v0.6

Broken out of `crates/braid/v0.6_spec.md` §12 (Forward concerns). Everything
here is **non-binding** for v0.6: performance is a first-class *future*
concern (large ballot sets; browser-hosted trustees in M3), and v0.6
prioritizes correctness and clarity. This file collects the ground rules the
spec established, the concrete work items queued so far, and the tooling that
exists to run them.

## Ground rules (from the spec)

- **Optimize from benchmarks, not speculation.**
- The pre-refactor `braid` implementation is a valuable reference and a source
  of reusable, already-tuned code (git preserves it).
- `ascent` runs sequential-only (no `par`, spec §7.8), so parallelism lives in
  the action/crypto layer: rayon natively, `wasm-bindgen-rayon` in the browser.
- Infrastructure note: `braid/Cargo.toml` carries a `jemalloc` feature (gating
  `tikv-jemallocator`) as both a higher-performance native allocator and a
  profiling / introspection tool; it is not yet wired into the runtime but is
  available for use when optimization work begins.
- The `vsc` crate marks known-unoptimized paths inline with
  `#[crate::warning("... not optimized ...")]`; build with
  `--features custom-warnings` to surface them as compiler warnings.

## Work items

### 1. Shuffle fold strategy — benchmark, then choose

`vsc` carries **two implementations** of the shuffle's wide parallel folds
(the `∏` products over `[[Element; W]; 2]`-sized values in proving and
verification), routed through one seam (`zkp::shuffle::fold_values`) and
selected at compile time by the `bounded-combine` cargo feature:

- **default (off):** rayon's recursive `reduce`, fused with the upstream
  `map` — the historical behaviour. Its stack use grows with ballot count `N`,
  width `W` and run-time work stealing, because each split dispatches a frame
  carrying `W`-sized accumulators. Measured on Windows x64 at `W = 100`, pool
  threads need 4 MiB at `N = 100`, 8 MiB at `N = 1,000` and 16 MiB at
  `N = 10,000` — overflowing default-sized thread stacks (`0xC00000FD`) well
  inside realistic parameters, on whatever pool the caller happens to run.
- **`bounded-combine` (on):** materialize the fold's values, then fold chunks
  (chunk count proportional to the thread count) with plain loops. Stack use
  is bounded by a small constant independent of `N` and scheduling; the cost
  is holding the materialized values (`N · 2W · 160` bytes per fold) for the
  duration of that fold.

The folded products — and therefore the **proofs — are byte-identical** across
the two (chunked folding preserves operand order; the operations are
associative), so the choice is purely operational. Measured so far
(Windows x64, 16 threads): timing indistinguishable within run-to-run noise at
`N = 1,000` (`W ∈ {30, 100}`) and `N = 10,000` (`W = 30`); peak RSS slightly
*lower* under `bounded-combine` (deep stacks stop being committed, which
outweighs the materialization at the cells measured).

**To decide:** benchmark at large `N` (10⁵–10⁶) across widths, natively and —
once M3 makes it reachable — under wasm, then either adopt `bounded-combine`
as the default and remove the switch, or record why not. Adopting it removes
the coupling between shuffle parameters and thread stack sizing entirely —
production trustees run on default stacks, and wasm cannot size stacks at all
(fixed at link time), so the default strategy's growing stack demand is a
deployment hazard, not just a tuning knob.

**Tooling:** `vsc`'s `shuffle_scaling` example runs one `(count, width)` cell
per invocation and emits one CSV line recording the compiled-in strategy:

```text
cargo run --release --example shuffle_scaling -- 10000 30
cargo run --release --example shuffle_scaling --features bounded-combine -- 10000 30
```

> **Update (2026-09-20):** items 2 and the related shuffle multi-exp review
> are now designed in full in `MSM.md` (assessed against the implementation
> and `PROTOCOL.md`), staged as: parallelism-completeness pass → MSM traits →
> verifier → prover → decryption rider. Measurements for every stage are
> recorded in the [Measurement log](#measurement-log) below.

### 2. Multi-exponentiation in batched verifiable decryption

The batched decryption proof (`vsc`'s `dkgd::recipient`) computes its batched
statements `A = ∏ uᵢ^{eᵢ}`, `B = ∏ fᵢ^{eᵢ}` through
`DistGroupOps::dist_multi_exp` → `GroupElement::multi_exp`, which for
Ristretto is dalek's **constant-time** Straus (`MultiscalarMul`). The
`multi_exp` contract requires constant time because callers may pass secret
scalars — but at all four of these sites the inputs are public: the bases are
ciphertext `u` components and published decryption factors, and the exponents
are hash-derived batching values. Variable-time algorithms are sound there and
faster.

**Review adopting dalek's vartime paths** for these sites, in particular
[`VartimePrecomputedMultiscalarMul`](https://docs.rs/curve25519-dalek/latest/curve25519_dalek/traits/trait.VartimePrecomputedMultiscalarMul.html):
in `combine`, the ciphertext bases are **the same across all `T`
contributions** (statement `A` is recomputed per contribution), so a
precomputed table amortizes over `T`; the trait's *mixed* variant handles the
per-contribution dynamic part (the published factors) alongside the static
table. Per the `multi_exp` contract (and the note on
`RistrettoElement::multi_exp`), a variable-time variant must be a **separate
trait method with the public-inputs precondition in its name**, never a change
to `multi_exp` itself.

Related, same review: the shuffle verifier computes several `∏ basesᵢ^{expsᵢ}`
products (e.g. `A = ∏ uᵢ^{eᵢ}`) as per-item `exp` + fold rather than as a
multiscalar multiplication at all; those sites are also public-input and would
benefit from the same vartime multi-exp before any lower-level tuning.

### 3. Parallel serialization of large collections

Carried over from the serialization rewrite (`SERIALIZATION.md` §10, 2026-08-28),
where the `LargeVector` placeholder type was deleted. Its intent survives the
type: for very large collections (ciphertext lists in the tens of thousands),
parallel `write`/`read` could pay. In the canonical encoding, a `Vec` of
fixed-size elements has computable element boundaries, so parallelism is an
**implementation strategy behind the existing wire encoding** — chunk the element
region, serialize/deserialize chunks on rayon, concatenate — with no format
change and no distinct type. (Under the old format this required an
incompatible encoding, which is why `LargeVector` existed as a separate type.)
Do this only when profiling shows serialization on the critical path; the
encoding work per element is trivial next to the group operations that surround
it.

## Benchmark inventory

| Tool | What it measures | Notes |
| --- | --- | --- |
| `vsc` `benches/shuffle.rs` | shuffle prove/verify micro-benchmark | fixed `N = 100`, `W = 3`; Bencher auto-calibrated; nightly-only |
| `vsc` `examples/shuffle_scaling.rs` | one `(N, W)` cell, prove + verify wall-clock | fold-strategy A/B (item 1); CSV output for sweeps |
| `vsc` `examples/decrypt_scaling.rs` | one `(N, W)` cell of the decryption path: Naor-Yung verify-and-strip (serial and parallel), `partial_decrypt`, `combine` | fixed `T = 3, P = 5`; CSV output; covers the costs `shuffle_scaling` does not |
| `vsc` `benches/parallel_tradeoff.rs` | serial-vs-parallel for each per-element loop shape (scalar RNG, scalar arithmetic, scalar/point products, hash-to-scalar, point-exp) | criterion (stable); decides where rayon earns its keep vs where serial is simpler for no cost |
| `vsc` `benches/msm_strategy.rs` | multi-exp strategies: naive-parallel vs single/chunked dalek MSM, constant-time and variable-time | criterion (stable); selects the `multi_exp`/`vartime_multi_exp` override shape |

## Measurement log

The optimization campaign designed in `MSM.md` lands in stages, each measured
before the next begins so gains stay attributable to their stage (in
particular, MSM gains are measured against the *rayon-complete* stage-0
baseline, not the original one).

Machine for all rows below: Windows x64, 16 logical cores, dalek AVX2
backend, `--release`. Times in ms.

> **Provisional.** Every number in this log so far was taken on a machine
> that was also compiling and doing other work. The criterion benches
> (`parallel_tradeoff`, `msm_strategy`) self-calibrate (warmup + sampling), so
> their *verdicts* are trustworthy; the single-shot and interleaved scaling
> sweeps are noise-sensitive and their absolute ms should be treated as
> directional. **An authoritative run under a quiesced machine is pending** —
> use `./bench.sh` (builds first untimed, then runs the whole grid to a
> timestamped `bench-results/` file) and replace the numbers here with that
> run's, marking them controlled.

### Methodology note (learned at stage 0)

**Single-shot cross-run comparison is invalid on this machine.** Running the
baseline sweep and the stage-0 sweep back to back, `strip_serial` — the
*same, unchanged* serial code in both binaries — "regressed" +43% at
N = 10⁵, purely because sustained load had warmed the machine (thermal
throttling). Any speedup read off two separate sweeps is contaminated by that
drift.

So changed sites are measured by **interleaving** the baseline and stage-0
binaries within one run (`base, s0, base, s0, …`), reporting the **median of
3 reps**, with an unchanged column as a control that must match between the
two. `decrypt_scaling`'s `strip_serial` is a perfect control (identical code
in both binaries); a factor is trusted only when it holds. `git stash` builds
the baseline binary from the same tree, so the two differ only by the stage's
edits.

### Baseline (branch point, pre-stage-0) — 2026-09-20

`shuffle_scaling` (prove / verify), single-shot (see the caveat above — use
for orientation, not for stage deltas):

| N | W | prove | verify |
|---|---|---|---|
| 10³ | 2 | 144 | 110 |
| 10⁴ | 2 | 1 355 | 1 058 |
| 10⁴ | 5 | 2 168 | 1 943 |
| 10⁵ | 2 | 18 535 | 13 172 |
| 10⁵ | 5 | 22 496 | 21 197 |

`decrypt_scaling` (T = 3, P = 5), single-shot:

| N | W | strip serial | strip parallel | partial_decrypt | combine |
|---|---|---|---|---|---|
| 10⁴ | 2 | 2 716 | 462 | 1 152 | 3 519 |
| 10⁴ | 5 | 6 506 | 1 159 | 3 351 | 11 532 |
| 10⁵ | 2 | 27 376 | 4 594 | 12 098 | 37 020 |

Two reads: the decryption path had never been measured and is *costlier than
the shuffle* at the same size (`combine` 37 s vs verify 13.2 s at N = 10⁵,
W = 2 — the sequential Lagrange accumulation, T·N·W exponentiations); and the
strip columns show the braid first-mix loop's available gain directly.

### Stage 0 — parallelism-completeness pass — 2026-09-20

Edits: parallelized `e_n`/`b_n`/`u_n_fold`/`h_n_fold` in the shuffle;
`partial_decrypt` factors, `batching_exponents`, `combine`'s Lagrange
accumulation and plaintext extraction in `dkgd`; P-256 `ind_generators`;
braid's first-mix Naor-Yung strip loop. All outputs bit-identical (vsc +
braid suites pass, including all 17 model-check configs).

Interleaved A/B, N = 10⁵ W = 2, median of 3 reps (ms):

| Site | baseline | stage 0 | factor | note |
|---|---|---|---|---|
| `strip_serial` (control) | 35 813 | 35 829 | 1.00× | identical code — confirms the A/B is thermal-neutral |
| Naor-Yung strip loop (braid B1) | 35 813 (serial) | 4 600 (parallel) | **~7.8×** | serial-vs-parallel measured in one run |
| `partial_decrypt` | 15 208 | 9 602 | **~1.6×** | now bounded by the two single-threaded CT-Straus `dist_multi_exp` calls |
| `combine` | 44 653 | 27 202 | **~1.6×** | now bounded by 2·T single-threaded CT-Straus `dist_multi_exp` calls |

Shuffle prove/verify, interleaved median of 3 at N = 10⁵ W = 2: prove
12 939 → 12 888, verify 10 663 → 10 294 — no measurable change (the stage-0
shuffle edits — `e_n`, `b_n`, folds — are together well under 1% of shuffle
wall-clock; the shuffle's cost is the MSM sites and the serial `big_b_n`
chain, addressed in stages 1–3). Note these interleaved figures are well
below the single-shot baseline table above (prove 18 535, verify 13 172),
confirming that sweep was thermally inflated: **the ~12.9 s / ~10.3 s
interleaved numbers are the trustworthy stage-1 starting point**, not the
single-shot ones.

**Finding that shapes stage 1:** once the trivial loops are parallel,
`partial_decrypt` and `combine` are dominated by dalek's *single-threaded*
`multi_exp` (CT Straus) — exactly the site stage 1's chunk-parallel +
vartime `multi_exp` targets. The 1.6× here is the floor; stage 1 compounds
on it.

### Stage 0b — remove parallelism that doesn't pay — 2026-09-20

Not every rayon site earns its overhead and visual noise. `parallel_tradeoff`
(criterion) measured each per-element loop shape serial vs parallel; the
decision rule is *absolute wall-clock saved in context*, not the raw speedup
ratio (a 5× speedup on a 10 ms loop is 8 ms on a multi-second operation —
noise).

Per-shape at N = 10⁵ (serial → parallel):

| Shape | serial | parallel | speedup | abs. saved | % of prove/verify | decision |
|---|---|---|---|---|---|---|
| point exp-map (control) | 2.76 s* | 0.46 s* | 6.0× | ~2.3 s | dominant | **parallel** |
| hash-to-scalar | 86.7 ms | 11.1 ms | 7.8× | 75.6 ms | ~0.7% | **parallel** (real per-element work) |
| point product | 12.5 ms | 2.6 ms | 4.8× | 9.9 ms | <0.1% | **serial** |
| scalar RNG | 10.3 ms | 1.9 ms | 5.5× | 8.4 ms | <0.1% | **serial** |
| scalar inner-product | 8.5 ms | 1.4 ms | 5.9× | 7.1 ms | <0.1% | **serial** |
| scalar mul+add | 8.8 ms | 1.8 ms | 5.0× | 7.0 ms | <0.1% | **serial** |
| scalar product | 7.0 ms | 1.2 ms | 5.8× | 5.8 ms | <0.1% | **serial** |

\* point exp-map measured at N = 10⁴ (100k is seconds); it scales ~linearly.

Reverted to serial (with a comment at each site citing this bench): shuffle
`b_n`, `beta_n`/`epsilon_n`, `a`, `k_b_n`, `k_e_n`, `e_n_fold`, `u_n_fold`,
`h_n_fold`; dkgd `combine`'s plaintext extraction. Kept parallel: everything
point-exponentiation or hashing — the MSM sites, `g_b_n`, `big_b_prime_n`,
F′, `apply_permutation`, `partial_decrypt`'s factors, `batching_exponents`,
`combine`'s Lagrange accumulation, P-256 `ind_generators`, and the shuffle's
`e_n` derivation (hashing, the one cheap-looking site that is really ~0.7%).
Outputs bit-identical (associative ops, same order); vsc + braid suites pass.
Total wall-clock cost of these reverts: well under 0.5% of prove/verify, for
markedly simpler code and less committed worker-thread stack.

### Stage 1 — MSM primitives — 2026-09-20

Added `GroupElement::vartime_multi_exp` and `exp_many` (defaults + ristretto
overrides), and changed the ristretto `multi_exp` override from a single dalek
call to a chunked one. Chunk strategy chosen by `benches/msm_strategy.rs`.

MSM strategy vs the naive parallel product (the current shuffle pattern), at
N = 10⁵ (ms):

| strategy | time | vs naive_par |
|---|---|---|
| `naive_par` (baseline) | 536 | 1.0× |
| `ct_single` (one Straus call) | 2081 | **0.26× — 3.9× slower** |
| `vt_single` (one vartime call) | 534 | 1.0× — no gain |
| `ct_chunk_t` (chunked, CT) | 220 | 2.4× |
| `vt_chunk_t` (chunked, vartime) | 66 | **8.1×** |

This is the empirical core of the whole approach (MSM.md §2.2): a *single*
dalek MSM is single-threaded and **loses** to the already-parallel naive
product — a bare call would be a regression. Chunking into `num_threads`
pieces (one dalek MSM per chunk on the pool, partials summed) is what wins:
2.4× constant-time, 8.1× variable-time. `chunk_t` beat `chunk_4t` for vartime
at large N, so the override uses `num_threads` chunks with a 64-element floor.

The chunked CT `multi_exp` is already load-bearing before any shuffle wiring:
the decryption path (`partial_decrypt`, `combine`) routes through it via
`dist_multi_exp`, so both should speed up for free. Quantifying that
(stage 0 vs stage 1 binaries, `partial_decrypt` and `combine` at N = 10⁵) is
**deferred to the controlled `bench.sh` run** rather than measured on the
busy machine — expected to compound on stage 0b's 1.6× toward the
`ct_chunk_t` 2.4× the strategy bench showed.

Correctness: outputs bit-identical (chunked = single-call Straus by
associativity); new differential tests pin `vartime_multi_exp`/`exp_many`
against the naive default at N = 200 (multiple chunks, across dalek's
Straus/Pippenger switch); vsc + braid suites pass.

## Related, tracked elsewhere

- **Incremental fetch (monotonic cursor)** — a pure transport optimization for
  board clients; recorded in the spec §12 with its constraints (never
  security-relevant, cannot certify completeness).
