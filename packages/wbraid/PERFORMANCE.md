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

## Benchmark inventory

| Tool | What it measures | Notes |
| --- | --- | --- |
| `vsc` `benches/shuffle.rs` | shuffle prove/verify micro-benchmark | fixed `N = 100`, `W = 3`; Bencher auto-calibrated |
| `vsc` `benches/large_vector.rs` | `LargeVector` vs serde+bincode serialization | `LargeVector` is currently unused in production paths |
| `vsc` `examples/shuffle_scaling.rs` | one `(N, W)` cell, prove + verify wall-clock | fold-strategy A/B (item 1); CSV output for sweeps |

## Related, tracked elsewhere

- **Incremental fetch (monotonic cursor)** — a pure transport optimization for
  board clients; recorded in the spec §12 with its constraints (never
  security-relevant, cannot certify completeness).
