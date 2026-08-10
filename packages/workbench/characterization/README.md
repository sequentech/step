<!--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Validation-behaviour characterization

Step 1 of the migration path in
[`docs/VALIDATION_LOGIC_DISTILLATION.md`](../docs/VALIDATION_LOGIC_DISTILLATION.md)
§5.3: enumerate cells of the validation input space through the **real
implementation** and record the observed effects. Recorded tables are
*characterizations* (what the code does); they become *specifications* only
after human sign-off — disagreements between the recording, the docs and
expectations are the product here, not noise.

## Intent: all behaviour, and what "all" requires

The goal is to capture **the whole behaviour of the validation subsystem**,
not a subset — with per-rule `predict()` specs that are complete within
the decomposition (no global predict needed). That claim rests on two
independent completeness arguments, and only the first is delivered by
running more cells:

1. **Input completeness** — every rule × every cell, as the union of
   per-rule slices (tracked in "Coverage so far" / "What complete coverage
   means" below).
2. **Codomain closure** — the set of *observables the harness records*
   must cover every channel through which validation state reaches the
   world. Recording only dialogs and messages would be silently
   incomplete no matter how many cells run. Closure is not provable by
   enumeration; it is established by a **consumer census**: grep every
   read site of the validation state (`invalid_errors`, `invalid_alerts`,
   the policy fields, the marker/decline flags) and map each site to a
   recorded observable, an explicit out-of-scope entry, or a named gap.

### Consumer census (2026-08-10)

Read sites of `invalid_errors` / `invalid_alerts` outside checker/codec
internals and tests:

| Consumer | Channel | Census verdict |
|---|---|---|
| `InvalidErrorsList.tsx`, `VotingScreen.tsx`, `ballotSelectionsSlice.ts` | booth inline messages, dialogs, persisted record | **covered** (layers 1–3) |
| `voting_screen.rs` | gates | **covered** (layer 2) |
| `normalize_vote.rs`, `character_map.rs`, codec internals | round-trip plumbing | **covered** implicitly by layer 1 (they run inside the recorded decode) |
| `velvet-core` counting (`classify_ballot`, plurality, IRV) | tally classes + aggregates | known pending (classifier table) |
| `PlaintextVoteContest.tsx` ← ballot-verifier `ConfirmationScreen` | the standalone ballot-verifier's display of a decoded ballot | **out of scope** (decision 2026-08-10) — the ballot-verifier is not part of the workbench replication; revisit if/when it is lifted. The more durable of the two exclusions. |
| `wasm_plaintext.rs` | plaintext-interpretation WASM entry feeding the above | **out of scope** (same channel, same decision) |
| `velvet` `mcballot_images.rs` | rendered ballot images | **out of scope** (decision 2026-08-10) — conditional: ballot-images functionality is not in the workbench (verified: no trace in velvet-wasm / velvet-core / app); include this channel if it ever is. Defects found here are recorded in [`../docs/UPSTREAM_FINDINGS.md`](../docs/UPSTREAM_FINDINGS.md) for reporting, not tracked as census gaps. |

The census is part of the artifact set: re-run it when upstream merges
land, and treat a new unmapped read site exactly like a failing test.

## Conventions

These exist to keep cognitive load minimal and the artifacts trustworthy;
they were adopted after an early hand-written summary table diverged from
its source within minutes of being written.

1. **Every table ships its legend.** The experiment (what act is repeated
   across rows), the meaning of every column (what moment / surface is
   observed), and the meaning of cell values (what a key or a dash denotes)
   are stated immediately above the table. A reader must never need the
   generating script or a conversation to decode a cell.
2. **Tables are generated, never hand-written.** Human-readable tables are
   projections of the recorded JSON (or of the rule set), produced by the
   runner. Retyping a table creates a second source of truth that will
   drift.
3. **Observations are pure per layer.** A layer-3 cell records only what is
   visible at that surface. "The alert exists but is hidden" is a *derived*
   fact — the join of layer 1 (emitted) with layer 3 (not visible) — and
   belongs in a cross-layer view labelled as such, never in a raw
   observation cell.
4. **Roles of the artifacts.** The recorded JSON is the *characterization*
   (evidence, and after sign-off the regression oracle); the `predict()`
   rule set is the embryonic *specification* (the canonical statement of
   behaviour — small, readable, per-rule); tables are *views* for humans.
   The spec is validated against the recording by enumeration, not by eye.

## Harness

Two runners per rule, matching the functional model in
[`docs/VOTE_VALIDATION.md`](../docs/VOTE_VALIDATION.md):

- **Layers 1+2 (checker + gates), headless.** `harness.mjs` loads the
  sequent-core wasm package directly in Node — no browser, no dev server —
  and calls the same entry points the booth calls
  (`test_contest_reencoding_js`, `check_voting_not_allowed_next`,
  `check_voting_error_dialog`). Requires `packages/sequent-core/pkg/` (built
  by the workbench's `predev`, or `yarn build:sequent-core`).
- **Layer 3 (filter + dialog wiring), browser.** `*.browser.mjs` drives the
  real booth via Playwright against the dev server on :5173, swapping
  policies per cell by dispatching a modified ballot style into the portal
  store. Inline visibility is observed through the `data-warn-id` attribute
  every WarnBox carries (upstream #2832), which yields raw message keys —
  no i18n ambiguity.

Each layers-1+2 runner also carries a `predict()` function — a direct
transcription of the documented rules. It is deliberately independent of
the implementation: it is the embryonic declarative mapping, and every
recorded cell is compared against it (`pred?` column / mismatch report).

## Coverage so far

### blank-rule (2026-08-10) — all three layers, zero disagreements

`blank-rule.mjs`: 64 cells (`blank_vote_policy` × `invalid_vote_policy` ×
{empty, explicit_invalid, marker_only, one_regular}) over the Referendum
contest of the `explicit-blank-invalid` fixture. **64/64 match the
documented prediction** → `blank-rule.recorded.json`, `blank-rule.md`.

`blank-rule.browser.mjs`: the blank condition through the real booth
(invalid policy at default) → `blank-rule.filter.recorded.json` +
[`blank-rule.filter.md`](./blank-rule.filter.md), the generated table
(with its legend — see the conventions above).

Confirmed live: the touch gate (untouched → everything cleared), the
`WARN_ONLY_IN_REVIEW` observation-point dependency, the master-filter
exception (`not-allowed` blank error visible under `invalid_vote_policy ==
ALLOWED`), and both dialog classes.

Incidental finding: the gate's debug logging prints `max={min:?}` —
`voting_screen.rs`'s `console_log!` interpolates `min` for both fields.
Cosmetic, upstream.

## Marker-inclusive counting caveat

The vote-state classes are defined over `num_selected_with_markers`: the
`marker_only` state (explicit-blank marker selected, nothing else) counts
**1**, so it is *not* blank at the booth — verified in the recording (no
blank checker output, no gate) — while classifying as `ExplicitBlank` at
tally. See VOTE_VALIDATION.md "Selection counting and marker candidates".

## What complete coverage means

The functional model in VOTE_VALIDATION.md has **six roles**; the harness
layers observe them unevenly, and full behaviour — the distillation's
`f(config, vote_state, context) → effects` — is only characterized when
all six are:

| Role | Harness layer | Status |
|---|---|---|
| Checkers | 1 (headless wasm) | blank rule done. One recording serves **both** bands: the tally decode runs the identical function, so layer 1 is also the tally-side checker characterization. |
| Gates | 2 (headless wasm) | blank rule done |
| Filter | 3 (browser, booth) | blank rule done |
| Input constraint | 3 (browser) — the `constraint` component of the effect triple | **not yet observed** by any runner; becomes relevant with the over-vote rule (`NOT_ALLOWED_WITH_MSG_AND_DISABLE`) |
| Marker exclusivity (prevention) | browser — *reachability*, not effects | **not yet characterized.** Prevention is characterized by *attempting* to create each vote state through the UI and recording whether it forms. The mixed marker state was verified at tally by direct injection into the tally sandbox — which bypasses booth prevention, so its booth-reachability is still an open cell. |
| Tally classifier | headless (velvet-core) | unit-tested upstream (16 tests) and spot-verified end-to-end via the marker fixture, but **no recorded decision table yet** |

Two consequences worth stating plainly. First, a per-rule recording like
blank-rule is a *slice* of `f`, by design — the mapping decomposes per
rule and per surface, and coverage is the union of slices, not any single
run. Second, prevention does not produce effects; it prunes the *input
space* — so its characterization output is a reachability table
(state × config → forms / does-not-form), which is also exactly the data
needed to justify (or refuse) pruning cells from the other layers'
enumerations.

## Adding a rule

Copy `blank-rule.mjs`, swap the policy/state dimensions and the `predict()`
transcription, and pick or extend a bundled fixture whose contest carries
the rule's preconditions (see FIXTURE_VARIANCE.md §13.2 for why marker
candidates are preconditions, not policies). Rules still uncharacterized:
over-vote, under-vote, min-vote, invalid (beyond the blank interplay),
duplicated-rank and preference-gaps (need a preferential fixture), the
decline-to-vote flow, and the tally classifier's six-class table.
