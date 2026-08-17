<!--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Validation-behaviour characterization

Step 1 of the migration path in
[`docs/VALIDATION_LOGIC_DISTILLATION.md`](../docs/VALIDATION_LOGIC_DISTILLATION.md)
§5.3: enumerate the validation input space through the **real
implementation** and record the observed effects. Two words carry the
whole suite, defined here once: a **cell** is one concrete
(contest-configuration × vote-state) combination — e.g.
`blank_vote_policy = warn` × `invalid_vote_policy = allowed` × an empty
ballot; a rule's **grid** is the full cross-product of the dimensions
its runner varies (blank-rule: 4 blank policies × 4 invalid policies ×
4 vote states = 64 cells). Every recorded table has one row per cell.
Recorded tables are *characterizations* (what the code does). Surprising behaviours are
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
| `velvet-core` counting (`classify_ballot`, plurality, IRV) | tally classes + aggregates | **covered** — per-cell `tally` column in every rule table + the standalone classifier decision table (`classifier-table.md`, 32/32) |
| `PlaintextVoteContest.tsx` ← ballot-verifier `ConfirmationScreen` | the standalone ballot-verifier's display of a decoded ballot | **out of scope** (decision 2026-08-10) — the ballot-verifier is not part of the workbench replication; revisit if/when it is lifted. The more durable of the two exclusions. |
| `wasm_plaintext.rs` | plaintext-interpretation WASM entry feeding the above | **out of scope** (same channel, same decision) |
| `velvet` `mcballot_images.rs` | rendered ballot images | **out of scope** (decision 2026-08-10) — conditional: ballot-images functionality is not in the workbench (verified: no trace in velvet-wasm / velvet-core / app); include this channel if it ever is. Defects found here are recorded in [`../docs/UPSTREAM_FINDINGS.md`](../docs/UPSTREAM_FINDINGS.md) for reporting, not tracked as census gaps. |

The census is part of the artifact set: re-run it when upstream merges
land, and treat a new unmapped read site exactly like a failing test.

### Scope boundaries (named decisions, 2026-08-15)

The census above closes the OUTPUT side: every consumer of the
validation state is mapped. This table closes the INPUT side —
territory the harness deliberately does not exercise, named with a
re-entry condition so each reads as a decision rather than an
oversight. Review it alongside the census when upstream merges land.

| boundary | decision | what would re-open it |
|---|---|---|
| **multi_ballot codec lane** — the multi-contest decode (six checkers, the ballot-level decline bit, the 30-byte capacity) | out of scope: the workbench encrypts per-contest only (`raw_ballot`), so no browser lane can reach it (README, Known gaps). The classifier's decline cells are characterized headlessly (`classifier-table.md`) | the `multi_ballot` encrypt/decrypt lift — the same lift as the decline booth flow (README "What's next"). First target when it opens: upstream's own single/multi divergence report at n = 0 (meta#8235 error 5: blank warn in one codec, undervote warn in the other) |
| **encoding-error emissions** — write-in overflow (`writeInCharsExceeded`), `invalidMinVotes` / `invalidMaxVotes` | out of scope: no bundled fixture carries a write-in candidate (FIXTURE_VARIANCE.md §8) and every grid's bounds are sane. `spec.mjs` lists the keys only for hard-gate faithfulness (`EXPLICIT_OR_ENCODING`) | a write-in fixture, a malformed-bounds slice, or an adjudication question that turns on these cells |
| **config-rejection outcomes** — `check_contest_configuration` refuses a contest with two explicit-blank (or two explicit-invalid) marker candidates, before any per-ballot rule runs | out of scope: a distinct outcome class ("config rejected"), headlessly reachable but on no grid | a cheap headless slice (construct the two-marker contest) whenever that class needs pinning |
| **gate composition across contests / pages** | the spec and every grid are per-contest. Production's gates iterate all contests and fire if ANY matches (VOTE_VALIDATION.md, "Interaction with pagination") — plain OR-composition over the per-contest predicate the spec models; the composition itself is unvalidated | a multi-contest grid (e.g. on `mixed-3contests`) if the OR-composition is ever in doubt |
| **message parameters** — every checker warning is an `InvalidPlaintextError` carrying a `message` (the i18n key identifying the warning, e.g. `errors.implicit.selectedMax` — what the tables' *errors*/*alerts* cells show and what `data-warn-id` exposes in the DOM) and a `message_map` (the values interpolated into the translated text, e.g. numSelected/min/max — the booth renders `t(message, message_map)`, `InvalidErrorsList.tsx`) | comparisons are over the `message` keys only, at every layer; the interpolated `message_map` values are never checked | a finding that turns on a wrong interpolated value (say, the wrong maximum printed in an otherwise-correct warning) rather than a wrong message key |
| **marker preconditions as fixture choice** — `has_explicit_blank/invalid_candidate` | not spec inputs: they gate which vote-states are REACHABLE (a marker state needs a marker candidate), not what the mapping says about a state that exists. Each grid picks a fixture carrying its rule's preconditions | a rule whose EFFECTS (not merely reachability) turn out to depend on marker presence |
| **untouched voting view** — the voting screen before the voter's first selection in the contest (`isTouched`, Question.tsx state, armed when a selection appears). While untouched, `filterErrorList` unconditionally empties BOTH warning lists, so nothing renders inline even when the checker has already emitted warnings (e.g. `selectedMin` on a just-opened screen with `min_votes: 1`); the spec carries the view as the constant `votingUntouched: []` | validated as a constant, not per cell: the clear is a branchless early-exit — no policy or state to enumerate — and it is observed doing real work in `blank-rule.filter.md` (untouched column: warnings emitted under `warn`/`not-allowed`, nothing visible). The per-cell voting observations in `dom-validate.md` deliberately arm the touch first, so they never re-observe this view | any code path that renders inline content on an untouched contest (a warning exempted from the clear, or the clear gaining a condition) — the constant would stop being one, and the view would need per-cell treatment like the other two |

## Conventions

These exist to keep cognitive load minimal and the artifacts trustworthy;
they were adopted after an early hand-written summary table diverged from
its source within minutes of being written.

1. **Every table ships its legend.** The experiment (what act is repeated
   across rows), the meaning of every column (what moment / effect category
   is observed), and the meaning of cell values (what a key or a dash denotes)
   are stated immediately above the table. A reader must never need the
   generating script or a conversation to decode a cell.
2. **Tables are generated, never hand-written.** Human-readable tables are
   projections of the recorded JSON (or of the rule set), produced by the
   runner. Retyping a table creates a second source of truth that will
   drift.
3. **Observations are pure per layer.** A layer-3 cell records only what is
   visibly rendered there. "The alert exists but is hidden" is a *derived*
   fact — the join of layer 1 (emitted) with layer 3 (not visible) — and
   belongs in a cross-layer view labelled as such, never in a raw
   observation cell.
4. **Roles of the artifacts.** The recorded JSON is the *characterization*
   (evidence, and after adjudication the regression oracle); the embryonic
   *specification* is [`spec.mjs`](spec.mjs) — the whole mapping as one
   function `f(config, voteState)`: checker emissions, both
   gates, the classifier, the message filter (all three observation
   points of the inline effect), and reachability — with the
   per-cell meaning of each rule's grid defined once in
   [`rule-specs.mjs`](rule-specs.mjs); tables are *views* for humans;
   **suspects** live in `../docs/UPSTREAM_FINDINGS.md` until consultation
   adjudicates them. The spec is validated against the recording by
   enumeration, not by eye.
5. **Narrative claims meet the table standard.** The prose in `../docs/`
   is the reviewer's interface; the tables are its evidence — if the
   prose is wrong, correct tables do not save the material. Before a
   claim lands in findings prose, run the cheap test that would falsify
   it: a *causal* claim ("because", "produces") must survive the
   counterfactual against the recorded cells (would the outcome differ
   if the premise were false?); a *distinction* claim ("two harms",
   "also") must survive an identity test (can the two things come apart,
   even in principle?); a *category* word must fit its definition (an
   asymmetry is not a discrepancy). Ground each such claim in the
   specific cell, diff, or code line that supports it — connective
   tissue is derived bottom-up from the evidence, never written
   top-down from plausibility.

## Harness

One headless runner per rule plus one shared browser lane, matching the
functional model in [`docs/VOTE_VALIDATION.md`](../docs/VOTE_VALIDATION.md):

- **Layers 1+2 (checker + gates), headless — per rule.** `harness.mjs`
  loads the sequent-core wasm package directly in Node — no browser, no dev
  server — and calls the same entry points the booth calls
  (`test_contest_reencoding_js`, `check_voting_not_allowed_next`,
  `check_voting_error_dialog`). Requires `packages/sequent-core/pkg/` (built
  by the workbench's `predev`, or `yarn build:sequent-core`).
- **Layer 3 (filter, constraint, dialog wiring), browser — shared.**
  [`dom-validate.mjs`](dom-validate.mjs) drives every cell of every rule
  through the real booth via Playwright against the dev server on :5173,
  setting config through the **Policy-overrides panel** (the reviewer path)
  and navigating reload-free. Inline visibility is observed at **both
  observation points** — the touched voting screen (a deterministic
  tick-untick arms the touch per cell; the untouched view is a recorded
  constant — empty) and the review screen — through the
  `data-warn-id` attribute every WarnBox carries (upstream #2832), which
  yields raw message keys — no i18n ambiguity. (The two `*.browser.mjs`
  runners predate it — per-rule, store-dispatch config, reload-per-cell —
  and are kept as a cheaper dispatch-path check; see *Running the
  analysis*.)

Every prediction comes from [`spec.mjs`](spec.mjs) — the single shared
transcription of the production rules, exposed as one function
`f(config, voteState)` covering the checker emissions (which
errors/alerts the checker produces per config × vote state), the two
gates, the tally classifier, the booth message filter (inline visibility
at all three observation points — untouched voting, touched voting,
review — indexed in the output, not taken as an input), and reachability
(whether the booth UI lets a state form at all). A rule runner's `predict()` is a thin call into `f`,
fed from the rule's cell definitions in [`rule-specs.mjs`](rule-specs.mjs)
(`specConfig` / `voteState` — what each recorded row means in spec
terms); the runner itself contributes only its experiment grid and the
wire-level state construction. Every recorded cell is compared against
the prediction (`pred?` column / mismatch report). The `classifier-table`
runner's `predict()` *is* `spec.classify`, so that table validates the
shared classifier directly.

The spec now exists twice, deliberately: `spec.mjs` (validated against
the real WASM and DOM as above) and its typed Rust port,
[`../validation-spec/`](../validation-spec/) — the
VALIDATION_LOGIC_DISTILLATION.md §5.3 step-3 artifact. The Rust crate is
bug-compatible, with every surprising behaviour carried as a **named
quirk** (`quirks()` in its `lib.rs`, each tied to its
UPSTREAM_FINDINGS.md suspect/defect — toggling one is an adjudication
decision, not a refactor). Equivalence between the two specs, and
against the recorded ground truth, is checked by `rust-conformance.mjs`
(below).

**Two validation lanes, unequal coverage.** `spec.mjs`'s emissions, gates
and classifier transcribe Rust that IS compiled to wasm, so `pred?` checks
them against the real wasm on every cell (independent derivations — this
JS vs that Rust — so agreement is real information). Its `inlineViews` /
`reachability` transcribe TypeScript that is NOT callable headlessly
(`filterErrorList`; the input disable; the blank-marker clearing), so
they are **predictions only** in a Node runner,
validated against the real DOM only where a browser runner covers the cell,
and never against a re-computation of themselves (that check would be
tautological). That per-cell DOM-validation lane now exists:
[`dom-validate.mjs`](dom-validate.mjs) drives every cell of **all seven
rules** through the real booth reload-free — the five plurality rules
(over-vote, min-vote, blank, under-vote, invalid) on the explicit-blank-
invalid fixture, and the two preferential rules (duplicate-rank,
preference-gaps) on the IRV fixture with ranked selection — observing inline
visibility at the touched voting screen and at the review screen, and
reachability (the input constraint), and
emits the **complete** tables in [`dom-validate.md`](dom-validate.md) —
**229/229 matching the spec** (see the e2e-cost note in
[`../docs/VALIDATION_LOGIC_DISTILLATION.md`](../docs/VALIDATION_LOGIC_DISTILLATION.md)
§5.3).

## Running the analysis

All commands run from `packages/workbench`; each runner writes its artifacts
next to itself in `characterization/`. There are two lanes — a fast headless
one and a browser one.

### Headless lane (no dev server)

Needs the sequent-core wasm at `packages/sequent-core/pkg/` (built by the
workbench's `predev`, or `yarn build:sequent-core`). Each runner is
independent and takes a second or two (WASM in Node, no browser), and writes a
**partial (headless) table** — the WASM observations (checker emissions,
gates, recorded tally) with a `pred?` column comparing them to `spec.mjs`:

| command (`node characterization/…`) | produces |
|---|---|
| `blank-rule.mjs` | `blank-rule.recorded.json` + `.md` |
| `overvote-rule.mjs` | `overvote-rule.recorded.json` + `.md` |
| `undervote-rule.mjs` | `undervote-rule.recorded.json` + `.md` |
| `minvote-rule.mjs` | `minvote-rule.recorded.json` + `.md` |
| `duprank-rule.mjs` | `duprank-rule.recorded.json` + `.md` |
| `prefgaps-rule.mjs` | `prefgaps-rule.recorded.json` + `.md` |
| `invalid-rule.mjs` | `invalid-rule.recorded.json` + `.md` |
| `classifier-table.mjs` | `classifier-table.recorded.json` + `.md` |
| `rust-conformance.mjs` | `rust-conformance.recorded.json` + `.md` — the typed Rust spec (`../validation-spec/`) vs the recorded ground truth (all 280 cells) and vs `spec.mjs` on 20 000 seeded-random cells. Needs `cargo` (builds `emit-grid` on first run), not the wasm pkgs |
| `effect-dependencies.mjs` | `effect-dependencies.md` + `.recorded.json` — the effect-first decomposition: per effect component, its support, its conditional-independence restrictions, and one executable witness per dependence, computed exhaustively on the Rust spec over the full modelled domain (~29M evaluations); representable witnesses re-run through the real WASM, the rest labelled. Needs `cargo` **and** the wasm pkg; ~2 min |
| `headless-sweep.mjs` | `headless-sweep.md` + `.recorded.json` — production (real WASM checker/gates/tally) vs `spec.f` on **every** cell of the representable headless subdomain (all six policies × sane bounds × plurality states; 138,240 cells, ~30 s): discharges the spec's headless *independence* claims by coverage, and emits the quotient inventory (reachable emissions × consulted-policies classes + representative cells) that the browser stage consumes |

### Browser lane (dev server on :5173)

Start the dev server first (its `predev` builds the wasm):

```
corepack yarn workspace "@sequentech/workbench-app" dev
```

Then, from `packages/workbench`:

| command (`node characterization/…`) | produces | what it does |
|---|---|---|
| `dom-validate.mjs` | `dom-validate.md` + `.recorded.json` | the **complete** tables — `spec.mjs` vs the real DOM across every cell of all seven rules (inline visibility at review + reachability); 229/229, ~8 min |
| `no-silent-discount.mjs` | `no-silent-discount.md` + `.report.json` | the no-silent-discount property query (headless pre-filter → browser confirm at review) |
| `reproduce-verify.mjs` | `reproduce-verify.recorded.json` | runs the three end-to-end runners below in sequence and aggregates one pass/fail |
| `overvote-e2e-pipeline.mjs` | `overvote-e2e-pipeline.recorded.json` | S1 over-vote: booth → cast → decrypt → decode → tally in one continuous run |
| `minvote-e2e-pipeline.mjs` | `minvote-e2e-pipeline.recorded.json` | S2 below-min (all four cells), same full pipeline |
| `invalid-latent-choices-e2e.mjs` | `invalid-latent-choices-e2e.recorded.json` | S5 null-vote choice leakage, same full pipeline |

Each of these browser runners exits nonzero on failure, so they compose in
CI. The earlier per-rule filter runners (`blank-rule.browser.mjs` →
`blank-rule.filter.recorded.json` + `.md`; `overvote-rule.browser.mjs` →
`overvote-rule.filter.recorded.json`) predate `dom-validate` and are kept as
the cheaper two-halves grid check; `dom-validate` now covers the filter lane
for all seven rules.

`harness.mjs`, `spec.mjs`, `rule-specs.mjs`, and `browser-harness.mjs` are
shared modules (imported, not run); `dom-probe-overvote.mjs` is a timing
spike with no artifact.

## Coverage so far

A dated log of when each recording landed and what it pinned — milestones,
not current validation state. The current state is the status table at the
end of this file and [`dom-validate.md`](dom-validate.md): **all seven
rules DOM-validated, 229/229**. In particular, entries below that say
"headless" describe what that recording did at the time; every rule has
since been driven through the real booth by `dom-validate.mjs`.

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
  candidate); the `overVoteDisabled` alert was visible. Prevention observed
  behaviourally — and now **also directly**: `dom-validate.mjs` probes the
  (max+1)th control's `disabled` attribute
  (`input.candidate-input[aria-label="Bruno"]`) on every `disable × over_max`
  cell, which reads `no (disabled)` in the complete table. Two independent
  signals for the same constraint.
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

### duprank-rule + prefgaps-rule (2026-08-11) — the preferential pair

`duprank-rule.mjs` / `prefgaps-rule.mjs`: 16 cells each (its own policy ×
`invalid_vote_policy` × {valid_full, duplicate|gap}) over the IRV
*Favourite fruit* contest (`instant-runoff-3cand` fixture; `selected` =
rank). **16/16 + 16/16 match.** The structural finding is shared: both
enums have *only* `*_WARN_AND_DIALOG` variants — no silent `allowed` — so
a gate always fires when the rule's error is present, whatever
`invalid_vote_policy` is. Zero silent-discount cells for both, **by
construction** (see VALIDATION_LOGIC_DISTILLATION.md §4.5, condition 2).

### no-silent-discount — the property query (first run 2026-08-10; observation-based since 2026-08-12)

`no-silent-discount.mjs` is **observation-based end to end** — no model in
the finding path. *Phase 1 (headless):* scan every recorded cell across
all seven rule recordings (blank, over-vote, under-vote, min-vote,
duplicated-rank, preference-gaps, invalid) — **248 cells** — for
`tally = ImplicitInvalid` ∧ no gate (both real WASM observations), a sound
superset: **7 candidates**. *Phase 2 (browser):* drive each candidate
through the real booth and confirm it is reachable and shows nothing
inline at the review screen with no dialog. Result: **5 confirmed silent
discounts in two distinct families** (2 candidates rejected), all requiring
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

**Each of the five violations is confirmed through one continuous run of
the real workbench pipeline** — booth encrypt → cast → bridge decrypt →
decode (checkers populate `invalid_errors`) → tally — with the voter
shown nothing and the ballot ending `total_valid_votes: 0`,
`invalid_votes.implicit: 1`:

- over-vote (`over=allowed, invalid=allowed, over_max`) —
  `overvote-e2e-pipeline.recorded.json`;
- all four min-vote cells (`invalid=allowed`; `min=1/none`, `min=2/none`,
  `min=2/one`, `min=2/marker_only`) — `minvote-e2e-pipeline.recorded.json`.

The sharpest is `min=2/marker_only`: the voter selects the **Blank vote (explicit blank)**
marker — a deliberate blank — and it is silently discarded
`ImplicitInvalid` (the marker counts as 1 < 2). The end-to-end
crypto-chaining check is complete for the whole finding; the whole-grid
booth check is `dom-validate.mjs` (all seven rules, 229/229).

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
`f(config, vote_state) → one value per effect category` — is only
characterized when all six are:

| Role | Harness layer | Status |
|---|---|---|
| Checkers | 1 (headless wasm) | blank, over-vote, under-vote, min-vote, duplicated-rank, preference-gaps, invalid rules done. One recording serves **both** bands: the tally decode runs the identical function, so layer 1 is also the tally-side checker characterization. |
| Gates | 2 (headless wasm) | blank, over-vote, under-vote, min-vote, duplicated-rank, preference-gaps, invalid rules done |
| Filter | 3 (browser, booth) | **done for all seven rules, at both observation points** — `dom-validate.mjs` observes inline visibility at the touched voting screen and at the review screen across every cell of the five plurality rules (explicit-blank-invalid fixture) and the two preferential rules (IRV fixture, ranked selection): **229/229**. The untouched voting view is a recorded constant (empty — `blank-rule.filter.md`) |
| Input constraint | 3 (browser) — `spec.mjs`'s `reachability` | **done** — observed both **behaviourally** across every rule cell (the `reachable` column of `dom-validate.md`: the state forms or it does not) and **directly** for both prevention mechanisms: the over-vote `disable` policy (`no (disabled)`, from probing the (max+1)th control's `disabled` attribute) and blank-marker exclusivity (`no (cleared)`, the marker collapsing a co-selected regular) |
| Marker exclusivity (prevention) | browser — *reachability*, not effects | first reachability recording exists (over-vote under DISABLE: the state does not form); all five S1/S2 violations (over-vote + four min-vote) are confirmed through the full booth→cast→decrypt→tally pipeline (`overvote-e2e-pipeline.mjs`, `minvote-e2e-pipeline.mjs`). Prevention is characterized by *attempting* to create each state through the UI and recording whether it forms. Both marker directions are now recorded in `dom-validate`: the invalid marker does **not** clear (the `marker_plus` state forms — reachable `yes` — also confirmed end-to-end by `invalid-latent-choices-e2e.mjs`), and the blank marker **does** clear (the `regular_then_marker` state collapses to {marker only} — reachable `no (cleared)`). Open: the decline booth flow. |
| Tally classifier | headless (velvet-wasm `tally_decoded_ballots`) | **done**: per-cell `tally` column in all seven rule tables, plus the standalone 32-cell six-class decision table (`classifier-table.md`, 32/32 matching the documented precedence) |

Two consequences worth stating plainly. First, a per-rule recording like
blank-rule is a *slice* of `f`, by design — the mapping decomposes per
rule and per effect category, and coverage is the union of slices, not any single
run. Second, prevention does not produce effects; it prunes the *input
space* — so its characterization output is a reachability table
(state × config → forms / does-not-form), which is also exactly the data
needed to justify (or refuse) pruning cells from the other layers'
enumerations.

## Adding a rule

1. **Spec.** Add the rule's checker emissions to `spec.mjs::emissions`
   (transcribed from checker.rs, in decode order), and a `RULE_SPECS`
   entry in `rule-specs.mjs` carrying the rule's cell definitions
   (`specConfig` / `voteState` — what each cell means in spec terms).
2. **Headless runner.** Copy `blank-rule.mjs`, swap the policy/state
   dimensions and the wire-level state construction, and pick or extend a
   bundled fixture whose contest carries the rule's preconditions (see
   FIXTURE_VARIANCE.md §13.2 for why marker candidates are preconditions,
   not policies). This writes the partial `<rule>.recorded.json` + `.md`.
3. **Complete table.** Extend the rule's `RULE_SPECS` entry with the
   browser-driving half (contest selector, panel config, selection
   clicks, landmark, reachability checks) and add a `RULES` entry in
   `dom-validate.mjs`. The rule's cells then join the DOM-validated
   complete table.

Still open: the **decline-to-vote booth flow** — the classifier's decline
cells are recorded headlessly (`classifier-table`), but no booth-side runner
drives a declined ballot. (The blank-vs-invalid marker exclusivity asymmetry
is now fully observed: the invalid marker does *not* clear — `dom-validate`'s
invalid `marker_plus` cell forms {regular + null marker}, confirmed
end-to-end by `invalid-latent-choices-e2e.mjs` — and the blank marker *does*
— `dom-validate`'s blank `regular_then_marker` cell collapses to {marker
only}, `no (cleared)`.)
