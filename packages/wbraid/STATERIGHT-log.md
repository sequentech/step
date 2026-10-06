<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# Stateright work log — braid v0.6

A curated, measurement-carrying log of the model-checking commits on
`exp/braid-stateright/main`, kept for our own working reference (state counts,
what each step verified). Not required reading — the design is in
`STATERIGHT.md`, the full history is in git. Newest developments at the bottom.

| Commit | What | Measurement |
|---|---|---|
| `9d1a62dba9` | Crypto harness (v1): real stack over the in-memory board, 2 trustees / threshold 2 / width 2 / 2 ballots | 153 states, 0.46s, both properties |
| `9b5be76846` | `MemoryPersistence` moved into test code | — |
| `bbf98a71c4` | Lookahead + memoized deterministic `next_state`; two-clause no-change guard | same graph (153) |
| `c338a5e733` | Symbolic harness (v2): token artifacts, strong `eventually` completes | 153 = v1's tree exactly (cross-validation; no folding yet) |
| `95b8a4c5c1` | Canonical order-free state identity; completion = mixing quorum (property fix found by the 3-trustee/threshold-2 run) | 2/2: **35**; 3/3: 262; 3/2: 253 |
| `62f628e1a1`, `acac82a755` | `Action::ComputeBallots` removed (the manager is modeled as an actor that posts ballots) | — |
| `a57c222d84` | Transport model: staging/commit split + per-trustee views | counts unchanged (behavior-preserving fault-free) |
| `2048b8df27` | Fault scaffolding + `DropCommit` (first benign fault) | drops≤2: 187; no halts, every path completes, guards fire |
| `fa22aabf29` | Merge of `feat/braid-0.6.2/main` (vsc fixes, benchmark, lint) | all green post-merge; symbolic counts byte-identical |
| `9c6a687c95` | `CrashBeforeRecord` (second benign fault): real `post` aborted at the record write via a failing-persistence delegate | crashes≤2: 187 (mirrors drops≤2); mixed drops≤1+crashes≤1: 303 |
| `c55d0d3b53` | Fetch-failure stutter lemma pinned (benign tier complete): read failures have zero durable footprint, so no fault class needed | lemma test green |
| `fd6c60dd2c` | `EquivocateBallots` (first adversarial fault) + per-trustee halting | equivocations≤1: 135; benign counts unchanged |
| `949866871b` | Functional properties (differencing, linkage, integrity) over symbolic content — the asset-level reframe | equivocation → 112 (was 135; `halted` became a reason, dropping hash-order dedup noise) |
| `22f4f84994` | Dishonest mixers (`KnownPermutation`, `Forge`) + honest shuffle verification; two negative controls | known-perm: 35 (completes); forger: 22/24 (stalls); controls confirm the properties bite |
| `75b4806d15`, `c84871e493` | Adversarial-b4 withholding + compression-off ruling; refactored to strand-level; stable `HaltReason` | withhold≤1: 461; equiv≤1 withhold≤1: 1638; the completeness-gate guard fires |
| `06ba4290ca` | Input-ballot anchor check + `DishonestKind::SkipsAnchor` + spec trust-model note | **4/2 split-view flips VIOLATED → holds** (3709); the anchor-teeth control reproduces the violation; all 17 configs pass |
| `c49e30439b` | De-clutter: retired the mechanism lineage property (subsumed by differencing) | counts unchanged |
