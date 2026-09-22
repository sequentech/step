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

Two kinds of measurement. **Authoritative**: the reference-machine run of
`bench-ec2.sh` (BENCH-EC2.md) — a fresh EC2 instance of a fixed type,
quiesced by construction, both commits built and run interleaved.
**Stage-wise (directional)**: the laptop runs (Windows x64, 16 logical cores,
dalek AVX2 backend, `--release`) that guided each lever — interleaved A/B
where they compare, and noted as agent-run where not quiesced.

**How we measure.** Single-shot cross-run comparison is unreliable here —
sustained load warms the machine and inflates later runs (an *unchanged*
control drifted +43% between two back-to-back sweeps). So a changed site is
measured by interleaving the before/after binaries within one run (shared
conditions per rep), and the current side is cross-checked against the
quiesced snapshot to confirm it is not itself drift-inflated; the criterion
micro-benches (`parallel_tradeoff`, `msm_strategy`) self-calibrate with warmup
and sampling. `bench.ps1` / `bench.sh` run the whole grid under quiescence.

### Campaign before/after — the five targets (authoritative)

Measured 2026-09-22 with `bench-ec2.sh session 185dbbede2 657cb05c20`
(BENCH-EC2.md) on a fresh **`c7i.4xlarge`** — Intel Xeon Platinum 8488C, 16
vCPU (8 cores × 2 threads), 32 GiB; Ubuntu 24.04 `ami-0526a6499f6470118`,
kernel 7.0.0-1012-aws, rustc 1.96.0, `eu-west-1b` — quiesced by construction.
`examples/targets.rs` (§6) was built against both the **fork point**
`657cb05c20` (the parent branch before the campaign; the tool uses only
fork-point public APIs precisely so it drops into that tree) and the **merged
milestone** `185dbbede2`, and the two binaries ran **interleaved**, three reps
per cell. Reps agree within ~1%. Medians, N = 10⁵:

| Target | W = 2: fork → milestone | | W = 5: fork → milestone | |
|---|---|---|---|---|
| ① shuffle prove | 13.85 s → 4.92 s | **2.8×** | 23.5 s → 8.76 s | **2.7×** |
| ② shuffle verify | 10.26 s → 2.23 s | **4.6×** | 19.8 s → 3.57 s | **5.6×** |
| ③ partial decryption | 15.3 s → 1.19 s | **12.8×** | 38.2 s → 2.98 s | **12.8×** |
| ④ combine | 45.9 s → 1.99 s | **23×** | 114.8 s → 4.94 s | **23×** |
| ⑤ Naor-Yung verify-and-strip | 3.64 s → 3.70 s | flat | 8.82 s → 8.87 s | flat |

Raw files: `bench-results/ec2-20260922-012110-185dbbede2/` (differential CSV,
full grid, `machine.txt`). What it says:

- **The decryption path is the headline: ~13× and ~23×.** It had the most
  headroom — fully serial factor/`batching_exponents`/Lagrange loops *and*
  single-call MSM at the fork point — and it caught every stage:
  parallelization, chunked MSM, parallel transcript serialization, and the
  variable-time combine/verify path. At W = 5, `combine` went from 115 s to
  under 5 s.
- **The shuffle is ~2.7–2.8× (prove) and ~4.6–5.6× (verify).** Verify gains
  more because its MSMs are variable-time and V2 is batched; the prover's
  remaining cost is constant-time by necessity (§1).
- **⑤ is flat, exactly as predicted** — the campaign parallelized *braid's*
  first-mix loop, not vsc's `NYStrip` crypto, which `targets` calls with its
  own `par_iter` on both trees. On the reference machine ⑤ is now the
  **slowest target at both widths** (3.7 s vs verify's 2.2 s at W = 2): the
  unambiguous next lever (§5 — N independent PlEq verifications, not batched).

Milestone snapshot on the reference machine (median of 3, ms):

| N : W | prove | verify | partial_decrypt | combine | ny_strip |
|---|---|---|---|---|---|
| 10³ : 2 | 62 | 38 | 17 | 36 | 39 |
| 10⁴ : 2 | 490 | 249 | 128 | 228 | 368 |
| 10⁴ : 5 | 887 | 414 | 322 | 563 | 892 |
| 10⁵ : 2 | 4 919 | 2 234 | 1 194 | 1 989 | 3 718 |
| 10⁵ : 5 | 8 811 | 3 563 | 2 970 | 4 923 | 8 842 |

The criterion guidance benches also ran on this machine (the packaged
milestone's `bench.sh` predates the `GUIDANCE` flag) and reproduce §1's
strategy findings at N = 10⁵: a single constant-time dalek call is 4.3×
*slower* than the naive parallel product, chunked constant-time 2.6× faster,
chunked variable-time **8.0×** faster (47.8 ms). The laptop's stage-by-stage
runs below told the same story directionally and are kept as the record of
how each lever was found.

### The Amdahl wall — the finding that redirects the next work

The MSM primitive is 8× faster, but end-to-end shuffle **verify improved only
~1.9×** and `combine` sits at ~9.9 s (10⁵ W = 2). MSM is now a *minority* of
wall-clock. The dominant residual is **serialization — ristretto point
compression** (one inverse square root per point), run sequentially inside the
Fiat-Shamir seed derivations (`ser()` of the N-element ciphertext and
commitment lists). In `combine` the batching seed re-serializes the full
ciphertext list once **per contribution** (×T) — the same bytes compressed T
times. The chunked MSMs are only ~1.1 s of `combine`'s 9.9 s.

**Addressed (2026-09-21).** `par_ser` (§1) parallelized the transcript
serialization at both hot sites. Interleaved pre/post at N = 10⁵ W = 2:
**shuffle verify ~2.8×** (ser was ~65% of it), **prove ~1.9×**,
**partial_decrypt ~2.0×**, **combine ~2.0×**; `ny_strip` flat (no transcript
ser — the control). So the wall moved to the MSM work: verify's residual is
now the MSM plus the five equations plus SHA3 hashing. The `combine` per-
contribution re-serialization dedup was measured at only ~2–4% post-`par_ser`
and dropped as not worth a signature change.

**Decryption vartime (2026-09-21).** `combine` and `partial_decrypt` verify and
combine *published* factors — all public — yet were still on the constant-time
MSM path (the un-done original "item 2"). Switched to variable-time:
`combine`'s `a`/`b` statement rebuild → `dist_vartime_multi_exp`, and its
Lagrange accumulation `∏ f_{i,j}^{λ_i}` → one N-parallel region where each
`F_j` is a size-`T` `dist_vartime_multi_exp` (not inherent after all — it is
public-data exponentiation, so vartime applies). Interleaved CT/vartime at
N = 10⁵ W = 2: **`combine` ~2.4×** (5.96 s → 2.53 s), **`partial_decrypt`
~1.3×** (only its `a`/`b`; the `u^{xᵢ}` factors stay constant-time, secret
share). `combine`'s remaining cost is those vartime MSMs plus the T·N size-`T`
products.

## 5. Next levers, in priority order

1. **Parallel serialization — done** (2026-09-21, `par_ser`; §4 "Amdahl
   wall"). Behind the unchanged wire encoding. Verify ~2.8×, the rest ~2×. The
   `combine` re-serialization dedup was dropped (measured ~2–4%). Parallel
   *deser* (the other half of SERIALIZATION.md §10) remains, but is a
   different, colder site (message loading, not the transcript) and is
   safety-sensitive; do it only if a profile of the loading path warrants.
2. **Deferred prover fixed-base cleanups** — `apply_permutation`'s
   `uₙ = g^r·h` and the re-encryption `(g^s, y^s)` legs still use per-element
   `exp`/`repl_exp` rather than `exp_many`; part of the prove residual.
3. **Decryption vartime — done** (2026-09-21; §4 "Decryption vartime"). The
   public-data exps in `combine`/`partial_decrypt` moved to variable-time:
   `combine` ~2.4×, `partial_decrypt` ~1.3×. What remains in `combine` is those
   vartime MSMs plus the T·N size-`T` products — a smaller residual, and the
   exp *count* there is genuinely inherent to threshold interpolation.
4. **`ind_generators`** — N ristretto hash-to-curve per prove and per verify,
   already parallel. Measured ~110 ms at N = 10⁵ — ~1–2% of prove/verify, a
   minor contributor; folded into the ① ② target timings, not tracked
   separately.
5. **GPU** — deferred. The go/no-go rule is: adopt only if, after the residual
   above is fixed, MSM still holds ≥ 70% of verifier wall-clock at the
   deployment's real N *and* a latency requirement CPU scaling cannot meet
   exists. **Evaluated 2026-09-22 against the reference-machine numbers (§4): not met.** At N = 10⁵ W = 2 the verifier takes 2.2 s, of which its roughly a dozen MSM-equivalents — at ~48 ms each for a chunked variable-time MSM of 10⁵ on that machine — are ~0.6 s: a quarter to a third, not 70%. Even a free GPU MSM would buy at most ~1.4× on verify, while the untouched Naor-Yung verify-and-strip (⑤, 3.7 s) is a larger, CPU-side lever. The *G and VT* instance quota is granted (8 vCPUs), so a feasibility session is possible whenever it is wanted. If it ever
   is: Anza's `curve25519-cuda` (sppark-based, in `anza-xyz/cryptography`,
   companion to the `solana-ed25519` dalek fork) is the one candidate GPU MSM
   for this curve — variable-time, with a GPU→CPU fallback — but as of 2026-09
   it is unpublished (crates.io holds a v0.0.0 placeholder) and unaudited, so
   adopting it would be integrate-and-validate against an immature dependency
   in an election verifier's trust chain. The security posture if built: GPU on
   the verifier only (public data), CPU prover (secret ε never reaches VRAM),
   feature-gated with silent CPU fallback, CPU path normative for Verificatum
   interop.
6. **128-bit `e_n` — dropped** (2026-09-21). Shortening the batching
   challenges would roughly halve the MSM window count, but in Terelius–
   Wikström `e` drives the permutation-matrix argument itself: the
   Schwartz–Zippel bound becomes ~N/2¹²⁸ and the extraction argument needs
   re-deriving. That alters the protocol's soundness argument, which is not a
   performance decision; it stays out of scope here.

The batched-V2 verifier is now a documented, permitted check: PROTOCOL.md §6.4
states its form and exact `1/q` error bound (§9.2 references it), with the
per-index equations kept normative — PROTOCOL-alignment.md D7 (2026-09-21).

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

## 7. Regression verification of the campaign (2026-09-21)

Every CI gate in `.github/workflows/wbraid.yml`, run against the final tree
with CI's exact commands:

- `cargo fmt -- --check` — clean, workspace-wide.
- `cargo clippy --workspace --exclude vsc --features sqlite,postgres
  --all-targets --no-deps -- -D warnings` — clean.
- `cargo clippy -p vsc --no-deps` — clean.
- `cargo test --release --features sqlite,postgres` — all five crates green:
  vsc 198 + 38; braid unit tests, all 17 model-check configurations, the
  protocol harnesses on both curves, the serialization properties; b4 8;
  rnk 51; v2v's 48 local tests.
- `cargo build -p braid --lib --release --target wasm32-unknown-unknown
  --no-default-features --features wasm-core` — compiles.

Beyond CI:

- **Verificatum interop** (`crates/v2v/TESTING.md`), with `V2V_REQUIRE_VMN=1`
  so a pass cannot be a silent skip: **70/70 against Verificatum 3.1.0** —
  `they_verify_ours` 8/8 (our shuffle and decryption proofs accepted by
  `vmnv`), `we_verify_theirs` 3/3 (`vmn` corpora accepted by our verifier),
  the transcript-match tests 7/7 (shuffle seed, challenge, decryption
  transcript and generators byte-identical to `vmnv -t`), and the negative
  controls in both directions. The prover's closed-form commitments and the
  verifier's batched-V2 / variable-time path interoperate unchanged.
- The production `wasm` feature (wasm-bindgen-rayon pool, atomics, build-std)
  compiles: `par_ser` and every parallel site build in the pool configuration.
  CI checks only wasm-core; this exceeds it.
- `cargo doc -p vsc --no-deps` (broken intra-doc links denied) clean; SPDX
  headers on every new file; `Cargo.lock` consistent under `--locked`.

**Wasm runtime** (tooling installed 2026-09-21: wasm-bindgen-cli 0.2.128 to
match the pin, a chromedriver matching the installed Chrome, Docker Desktop
with LocalStack pinned to `:4`, the AWS CLI):

- `test-wasm.ps1`, the headless IndexedDB test — **passes**: the `wasm-core`
  build (the feature the campaign's braid `rayon` change touches) compiles for
  wasm32 and `indexeddb_round_trips_predicates` runs green in headless Chrome.
- CI's opt-in **live-b4** tests (`cargo test -p braid --release -- --ignored`
  against the real b4v6 + LocalStack S3) — **pass**: `test_protocol_http`,
  `test_protocol_http_union` (client-side `SqlitePersistence`), and the
  real-crypto `model_check_two_trustees`.
- The interactive **emulator** (`TESTING.md` — the one end-to-end check of the
  protocol under wasm with the rayon pool) — **passes** (2026-09-21): the
  production `wasm` build (wasm-bindgen-rayon pool, atomics), served with
  COOP/COEP against the real b4v6 + LocalStack S3, ran create board → DKG to
  fixpoint → tally to fixpoint → **Verify plaintexts** green → a second tally
  on the same DKG → page refresh with automatic reconnect.

Every check in this section has now run against the final tree and passed.
