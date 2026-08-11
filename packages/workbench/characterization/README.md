<!--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Validation-behaviour characterization

Step 1 of the migration path in
[`docs/VALIDATION_LOGIC_DISTILLATION.md`](../docs/VALIDATION_LOGIC_DISTILLATION.md)
§5.3: enumerate cells of the validation input space through the **real
implementation** and record the observed effects. Recorded tables are
*characterizations* (what the code does). Surprising behaviours are
**suspects** — recorded precisely, escalated for consultation in
[`../docs/UPSTREAM_FINDINGS.md`](../docs/UPSTREAM_FINDINGS.md), and
adjudicated (blessed or filed as defects) only by the parties who hold
design authority. Disagreements between the recording, the docs and
expectations are the product here, not noise; intuitions about them guide
attention (the silent-discount families get their own report because we
strongly suspect a defect) but never adjudicate.

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
   (evidence, and after adjudication the regression oracle); the
   `predict()` rule set is the embryonic *specification* (the canonical
   statement of behaviour — small, readable, per-rule); tables are *views*
   for humans; **suspects** live in `../docs/UPSTREAM_FINDINGS.md` until
   consultation adjudicates them. The spec is validated against the
   recording by enumeration, not by eye.

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
Each cell also records its per-ballot tally class (added after the
over-vote rule, so the blank rule participates in the no-silent-discount
query too).

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

### overvote-rule (2026-08-10) — all three layers + recorded tally class

`overvote-rule.mjs`: 60 cells (`over_vote_policy` × `invalid_vote_policy`
× {empty, at_max, over_max}) over the Council seat contest (Ada / Bruno,
`max_votes: 1`). **60/60 match the documented prediction.** Each cell also
records its **tally class** — the counter that incremented when the
decoded ballot ran through velvet-wasm's real tally
(`overvote-rule.recorded.json`, `overvote-rule.md`).

`overvote-rule.browser.mjs` (invalid policy at default): inline
visibility and dialogs match the headless prediction for every variant,
plus two firsts —

- **First reachability recording.** Under
  `NOT_ALLOWED_WITH_MSG_AND_DISABLE` the over-vote state **did not form**
  through the UI (selection count stayed at max after clicking a further
  candidate); the `overVoteDisabled` alert was visible. Prevention
  observed behaviourally. (The direct probe of the input's `disabled`
  attribute returned null — DOM-selector gap, listed as a TODO — but the
  state-level evidence is conclusive.)
- **Violation reproduced against the real components** — see below.

### undervote-rule (2026-08-11) — headless, layers 1+2 + tally

`undervote-rule.mjs`: 48 cells (`under_vote_policy` × `invalid_vote_policy`
× {empty, under, full}) on the Referendum contest with `min_votes` forced
to 0, `max_votes` to 2. **48/48 match** (after the prediction was corrected
— see below) → `undervote-rule.recorded.json`, `undervote-rule.md`. Zero
silent-discount cells, as designed: the under-vote checker emits only
alerts, never errors, so an under-voted ballot is structurally `Valid`
(confirming §4.4's "cosmetic policy").

Two facts the recording pinned, both mis-transcribed on the first pass:
with `min_votes = 0` the under-vote zone `min ≤ n < max` **includes n = 0**,
so the alert fires on an empty ballot too (overlapping blank); and the
soft gate requires `n > 0`, so it fires only for `under`, not for the
empty ballot the checker just alerted on — the alert and gate thresholds
differ. (This is characterization catching the doc, not the code: the code
was right.)

### minvote-rule (2026-08-11) — headless, layers 1+2 + tally

`minvote-rule.mjs`: 24 cells (`min_votes` ∈ {1,2} × `invalid_vote_policy`
× {none, one, marker_only}) on the Referendum contest. **24/24 match** →
`minvote-rule.recorded.json`, `minvote-rule.md`. Min-vote is not a policy
enum — it always pushes a `selectedMin` *error* when the marker-inclusive
count is below `min_votes` — and this rule **produces the second
silent-discount family** (4 cells; see below).

### classifier-table (2026-08-11) — the tally classifier's own decision table

`classifier-table.mjs`: 32 cells — the **full cross-product of the inputs
`classify_ballot` reads** (`is_decline_to_vote` × `is_explicit_invalid` ×
errors-present × {none, regular, marker, mixed}) — with the class recorded
through velvet-wasm's real tally. **32/32 match the documented
precedence** → `classifier-table.recorded.json`, `classifier-table.md`.

The contrast with the rule tables' `tally` column, made concrete: that
column samples this same classifier only at the decoded ballots each
rule's cells produce; this table probes it deliberately at every input
combination, including pipeline-unreachable ones (decline rows exist only
on multi-contest ballots; `has_errors` is synthetic here where the
pipeline would derive it from decode).

The recorded structure is itself a finding: **four of the six classes are
singletons** — `Valid`, `ImplicitBlank`, `ExplicitBlank` and `Declined`
are each produced by exactly one of the 32 combinations — while
**`ImplicitInvalid` absorbs 20 of 32** (`ExplicitInvalid` takes the
remaining 8: flag set, not declined). Every clean outcome is a knife-edge
and the discarding class is the sink — the structural reason the
silent-discount property matters. One precedence subtlety worth noting:
`decline + explicit-invalid flag + empty` classifies **Implicit**Invalid,
not ExplicitInvalid — the decline branch tests blankness, not the flag.

### invalid-rule (2026-08-11) — invalid-vote rule as subject, headless

`invalid-rule.mjs`: 20 cells (`invalid_vote_policy` × {none, regular,
flag_only, marker, marker_plus}) over the Council seat contest
(`max_votes` forced to 2 to isolate from over-vote). **20/20 match** after
the recording corrected the prediction (the soft gate *also* fires under
`not-allowed` via its generic errors-present condition, so `not-allowed`
trips both gates — the functions are independent booleans).

Two findings: (1) the flag and marker routes to explicit invalidity
**converge** on all four policies — selecting the null-vote marker is
equivalent to setting the flag, the gates' `explicit_invalid_marker_selected`
dedup working as intended. And the round-trip made explicit *why* there is
only one decoded representation: a marker-selected input with the flag
unset is rejected as inconsistent (decode drops the marker from the choice
slots and reads invalidity from `choices[0]`), so the marker click must
set the flag — which the booth reducer does. (2) Zero silent-discount
cells, and here that is *by definition*: an explicit-invalid ballot tallies
`ExplicitInvalid` (a deliberate opt-in), which the property excludes. A
separate, real consequence *is* recorded — S5 in `UPSTREAM_FINDINGS.md`:
the invalid reducer does not clear `choices`, so a null-voter's candidate
selections are preserved into the cast ciphertext (confirmed end-to-end,
`invalid-latent-choices-e2e.recorded.json`). Not a silent discount; a
privacy-adjacent asymmetry.

### no-silent-discount — first model-check query (2026-08-10)

`no-silent-discount.mjs` scans every recorded cell that carries a tally
class for: *silent on every booth surface ∧ classified `ImplicitInvalid`*.
Sources are all seven rule recordings (blank, over-vote, under-vote,
min-vote, duplicated-rank, preference-gaps, invalid) — **248 cells** — and the
result is **5 violating cells in two distinct families**, all requiring
`invalid_vote_policy = allowed`
(`no-silent-discount.report.json`, `no-silent-discount.md`):

| family | configuration | states |
|---|---|---|
| over-vote | `over_vote_policy=allowed`, `invalid=allowed` | over_max |
| min-vote | `min_votes=1`, `invalid=allowed` | none |
| min-vote | `min_votes=2`, `invalid=allowed` | none, one, marker_only |

The over-vote family matches §4.2's prediction. The min-vote family was
found by this pass: `selectedMin` is not in the booth filter's keep-list,
so under `invalid=allowed` a below-minimum ballot is suppressed and
neither gate fires, yet the tally discards it `ImplicitInvalid`. The
`min_votes=2 / marker_only` cell is the sharpest: a voter who selects the
explicit-blank marker (a deliberate blank) has it silently discarded,
because the marker counts as 1 < 2. Blank and under-vote contribute
zero: blank's `ImplicitInvalid` cells hard-gate, under-vote never produces
an error (only alerts), the two preferential rules are immune by construction (no silent policy
variant), and the invalid rule only ever tallies `ExplicitInvalid`, which
the property excludes.

**All five violations are confirmed through ONE continuous run of the
real workbench pipeline** — booth encrypt → cast → bridge decrypt →
decode (checkers populate `invalid_errors`) → tally — with the voter
shown nothing and the ballot ending `total_valid_votes: 0`,
`invalid_votes.implicit: 1`:

- over-vote (`over=allowed, invalid=allowed, over_max`) —
  `overvote-e2e-pipeline.recorded.json`;
- all four min-vote cells (`invalid=allowed`; `min=1/none`, `min=2/none`,
  `min=2/one`, `min=2/marker_only`) — `minvote-e2e-pipeline.recorded.json`.

The sharpest is `min=2/marker_only`: the voter selects the **Blank vote (explicit blank)**
marker — a deliberate blank — and it is silently discarded
`ImplicitInvalid` (the marker counts as 1 < 2). The crypto-chaining TODO
is closed for the whole finding. The cheaper two-halves browser runner
(`overvote-rule.browser.mjs`) stays as the whole-policy-grid check.

**A faithfulness rule this exercise taught** (an earlier revision of the
e2e check got it wrong and reported "Valid"): the tally classifies
*decoded* ballots — decode is what populates the `invalid_errors` that
`is_invalid()` reads. Feeding a hand-built selection straight into
`tally_decoded_ballots` classifies a checker-clean ballot. Any harness
step that tallies must run the encode→decode round trip first, exactly
as the production pipeline does.

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
| Checkers | 1 (headless wasm) | blank, over-vote, under-vote, min-vote, duplicated-rank, preference-gaps, invalid rules done. One recording serves **both** bands: the tally decode runs the identical function, so layer 1 is also the tally-side checker characterization. |
| Gates | 2 (headless wasm) | blank, over-vote, under-vote, min-vote, duplicated-rank, preference-gaps, invalid rules done |
| Filter | 3 (browser, booth) | blank + over-vote rules done (under/min-vote headless only so far) |
| Input constraint | 3 (browser) — the `constraint` component of the effect triple | observed **behaviourally** for `NOT_ALLOWED_WITH_MSG_AND_DISABLE` (the over-vote state does not form through the UI); the direct `disabled`-attribute probe is a DOM-selector TODO |
| Marker exclusivity (prevention) | browser — *reachability*, not effects | first reachability recording exists (over-vote under DISABLE: the state does not form); all five S1/S2 violations (over-vote + four min-vote) are confirmed through the full booth→cast→decrypt→tally pipeline (`overvote-e2e-pipeline.mjs`, `minvote-e2e-pipeline.mjs`). Prevention is characterized by *attempting* to create each state through the UI and recording whether it forms; the mixed marker state's booth-reachability is still an open cell. |
| Tally classifier | headless (velvet-wasm `tally_decoded_ballots`) | **done**: per-cell `tally` column in all seven rule tables, plus the standalone 32-cell six-class decision table (`classifier-table.md`, 32/32 matching the documented precedence) |

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
candidates are preconditions, not policies). Still open:
the decline-to-vote booth flow (the classifier's decline cells are now
recorded, but no booth-side runner drives a declined ballot), and the
blank-vs-invalid marker exclusivity asymmetry (a browser-reachability
check, deferred to the decline work).
