<!--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Evidence restructure — migration plan

**Role of this document.** A one-off migration plan, agreed 2026-08-18.
It describes moving the validation apparatus from *two specs and a
transitive evidence chain* to *one spec and two layers*. **Delete it once
the migration lands** — the end state belongs in
`../characterization/README.md`, not here.

**Why.** Not to fix a defect: the current apparatus is green everywhere.
The goal is that the evidence story fit in one person's head. Today it
does not — it takes seven per-rule runners with four jobs each, a
"superseded but retained" caveat, and a two-hop chain
(production ≡ `spec.mjs` ≡ Rust) whose second hop is established by
sampling. That complexity has already produced several inaccuracies in
the written record that only source-reading caught.

## Target: two layers

**Evidence layer — establishes `production ≡ Rust spec`.** Everything
here touches real production code.

| instrument | what it establishes |
|---|---|
| `headless-sweep.mjs` | exhaustion: production ≡ spec on every headlessly representable cell, plurality **and** preferential |
| `classifier-table.mjs` | the classifier at inputs decode cannot reach (decline — `raw_ballot` hardcodes `is_decline_to_vote: false`) |
| `dom-validate.mjs` | per-cell in the real booth on the reviewer path — the 229 cells |
| `quotient-validate.mjs` | sufficiency: browser independence by equivalence class |
| `browser-witnesses.mjs` | browser-side existential dependence claims |

**Analysis layer — consumes the certified spec, never touches
production.**

| instrument | what it yields |
|---|---|
| `effect-dependencies.mjs` | the dependency ledger (support, conditional restrictions, witnesses) |
| `effect-map.mjs` | the causal diagram, cancellations table, per-knob cards |
| `no-silent-discount.mjs` | the property, evaluated over the spec's **full** domain |

The findings track (`*-e2e-pipeline.mjs`, `reproduce-verify.mjs`) is
untouched: it confirms specific defects booth-to-tally and belongs to
neither layer.

## Inventory

**Removed (9).** `spec.mjs`; `rust-conformance.mjs` (both its comparisons
dissolve — the sweep *is* the ground-truth check, exhaustively, and
there is no second spec left to cross-check); the seven per-rule runners
in their validation role.

**Re-pointed at the Rust spec (6).** `headless-sweep`,
`quotient-validate`, `dom-validate`, `browser-witnesses`,
`effect-dependencies`, `classifier-table`.

**`emit-grid` needs no new cell kinds.** `Effects` already returns
`inline` and `reachability`; the `f` and `classify` kinds cover
everything. What it needs is throughput — callers batch their cells
instead of evaluating one at a time.

**`effect-dependencies` loses lane B.** It re-runs representable
witnesses through real WASM; once the sweep certifies production ≡ spec
across that entire domain, those re-runs prove nothing new. It becomes
pure analysis. Browser-side witnesses keep needing a booth, in
`browser-witnesses.mjs`.

## The oracle

Rust ≡ `spec.mjs` is already established on 20,280 cells, so **a faithful
migration leaves every recorded artifact byte-identical**. That is the
check at each step below, and the same standard used throughout this
suite.

Where an artifact does *not* come back identical, that is not migration
noise — it is a cell where the two transcriptions disagree that 20,000
random cells missed. Treat it as a finding: diff the two, decide which is
right, record it.

## Steps

Each step ends green, with the suite runnable.

0. **Batch throughput in `emit-grid` callers.** No behaviour change; a
   helper that takes an array of cells and returns an array of `Effects`.
   *Check:* `rust-conformance` still green (it is the existing user).

1. **Generalize the sweep's cell machinery beyond plurality.**
   `plurality-cell.mjs` becomes a cell module that also builds ranked
   selections on `instant-runoff-3cand.json` — the fixture
   `duprank-rule.mjs` already drives headlessly, so this is fixture
   selection, not new capability. `representable()` stops excluding
   preferential state; decline and `max_votes = 0` stay out.
   *Check:* sweep green against `spec.mjs`, cell count grows, 0
   disagreements. Keeping the comparison on JS isolates this step.

2. **Point the sweep at Rust.** Swap `specF` for `emit-grid`.
   *Check:* 0 disagreements. This is the load-bearing step — it
   establishes production ≡ Rust exhaustively over the whole headless
   domain, and subsumes `rust-conformance`'s ground-truth replay.

3. **Decouple `dom-validate`'s cells from the recorded grids.** Today
   each entry is `{...RULE_SPECS[rule], rows: rec("<rule>.recorded.json")}`
   — cell list *and* predictions from the recordings. The cell list moves
   into `rule-specs.mjs` (it is really the booth recipe: contest
   selector, panel config, selection clicks, landmark), and predictions
   come from `emit-grid`. Preserve the two existing filters (blank-rule
   drops `explicit_invalid`, invalid-rule drops `flag_only`).
   *Check:* 229/229, `dom-validate.recorded.json` content-identical.
   *Also assert:* every dom-validate cell lies inside the swept domain —
   that is what licenses comparing the DOM against spec-derived inline
   (see Decision 1).

4. **Re-point `quotient-validate` and `browser-witnesses`.**
   *Check:* 2,208 classes / 0 disagreements; 47 witnesses / 0; both
   recordings content-identical.

5. **Re-point `classifier-table`; strip `effect-dependencies` lane B.**
   *Check:* 32/32; the dependency ledger unchanged but for lane B's
   removal.

6. **Delete.** `spec.mjs`, `rust-conformance.mjs`, the seven rule
   runners. Regenerate the seven per-rule tables as **projections of the
   sweep's recording** if Decision 2 says keep.
   *Check:* no dangling imports; docs swept for references.

7. **Move the findings to the analysis layer.** Findings stop being
   things someone noticed and become **properties evaluated over the
   certified spec**, with production never involved. The acceptance test
   for the whole restructure: every finding we already hold must be
   re-derivable this way.

   - *no-silent-discount* — evaluate over the spec's full domain instead
     of pre-filtering 248 recorded cells. A strengthening, not a port:
     today the property is only as complete as the configurations the
     grids happen to contain.
     *Check:* the five known cells still found; anything **new** is a
     finding, not a regression.
   - *the gate/checker count divergence* (quirk
     `S6_GATES_COUNT_FIRST_PREFERENCES_ONLY`, found by the step-1 sweep)
     — express it as a property rather than a narrative: does the gates'
     selection count ever differ from the checker's, and where it does,
     what reaches the voter? Its two known consequences (a ranked ballot
     hard-blocked as blank; a `WARN_AND_ALERT` under-vote alert with no
     dialog) should fall out of that property, not be asserted alongside
     it. Only then write it up as a suspect.
     *Check:* the property finds both known consequences unaided.

8. **Docs.** `characterization/README.md`'s Coverage section rewritten
   around the two layers (the three-mechanism preamble survives; the
   "which runners carry coverage" paragraph disappears with its subject);
   root README "What's next" item 2 replaced by the outcome;
   `VALIDATION_LOGIC_DISTILLATION.md` §5.3 status; `CLAUDE.md` gotchas
   re-checked.

## Decisions to make during the work

1. **What `dom-validate` compares against.** Today: inline derived from
   *observed* emissions, so a filter failure is localized to the filter.
   After: inline from the spec, so a failure could be an emissions error
   instead. The sweep certifies emissions independently, so this is
   sound — but it trades fault localization for a shorter story. Step 3's
   in-swept-domain assertion is what keeps it honest.
2. **Do the seven per-rule tables survive as generated views?**
   Recommended yes: they are how a human reads per-rule behaviour, and as
   projections of the sweep's recording they cost nothing evidentially.
   The alternative is losing a genuinely useful reader artifact.
3. **Does `classifier-table` stay separate or fold into the sweep** as a
   direct-`classify` section? Either works; separate keeps the "synthetic
   inputs, deliberately not booth-produced" boundary visible.
4. **Sweep recording size.** `headless-sweep.recorded.json` is already
   2.1 MB; adding preferential grows it. Decide what the artifact stores
   (disagreements + quotient inventory, versus per-cell rows) before the
   file becomes unreviewable in a diff.

## What this does not change

- The **unset-policy divergence** is a production fact: the Rust spec
  defaults unset `invalid_vote_policy` to `allowed`, the TypeScript
  filter reads it raw. Restructuring cannot resolve it; adjudication can.
- The **booth fixture gaps** — no contest offering both markers, no
  generic IRV booth recipe — are unaffected. Note the IRV item is
  specifically a *booth* recipe; step 1 closes its headless half.
- The **findings** (S1/S2/S5) and their end-to-end confirmation.
- **Coverage itself.** Nothing here widens what is validated; it changes
  which artifact the evidence attaches to, and how many moving parts the
  story needs.
