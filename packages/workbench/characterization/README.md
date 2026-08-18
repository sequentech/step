<!--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Validation-behaviour characterization

Step 1 of the migration path in
[`docs/VALIDATION_LOGIC_DISTILLATION.md`](../docs/VALIDATION_LOGIC_DISTILLATION.md)
§5.3: enumerate the validation input space through the **real
implementation** and record the observed effects.
Recorded tables are *characterizations* (what the code does). Surprising behaviours are
**suspects** — recorded precisely, escalated for consultation in
[`../docs/UPSTREAM_FINDINGS.md`](../docs/UPSTREAM_FINDINGS.md), and
adjudicated (blessed or filed as defects) only by the parties who hold
design authority. Disagreements between the recording, the docs and
expectations are the product here, not noise; intuitions about them guide
attention (the silent-discount families get their own report because we
strongly suspect a defect) but never adjudicate.

## Vocabulary

Defined here once, because the rest of this document uses them freely.
Each names a concrete piece of code or a recorded column.

- **cell** — one concrete (contest-configuration × vote-state)
  combination: `blank_vote_policy = warn` × `invalid_vote_policy =
  allowed` × an empty ballot, say. Every recorded table has one row per
  cell.
- **rule** — one checker function in sequent-core's
  `ballot_codec/checker.rs`, plus everything downstream keyed to its
  message and policy: its gate clauses, its keep-list carve-out, its
  alert-visibility rule. There are seven, and this suite runs one runner
  and one grid per rule. (Not "one policy": min-vote's knob is the
  `min_votes` bound and it has no policy at all.)
  VALIDATION_LOGIC_DISTILLATION.md §2 enumerates the seven and explains
  why this carving is a choice rather than a given.
- **grid** — a rule's full cross-product of the dimensions its runner
  varies (blank-rule: 4 blank policies × 4 invalid policies × 4 vote
  states = 64 cells).
- **effect category** — one of the four things the mapping yields for a
  cell: the **inline** messages, the **dialog**, **reachability**
  (whether the booth lets the vote state form at all — a greyed control,
  a marker that clears a co-selection), and the **tally** class.
- **checker emissions** and **gate pair** — the two *checkable
  intermediates*: values computed on the way to those effects and checked
  here without being effects themselves. Namely, the errors/alerts
  `checker.rs` records, and the two submission gates (hard = blocking,
  soft = dismissible dialog — the dialog effect is their projection). The
  **inline filter** is the TypeScript that turns emissions into what
  actually renders (`filterErrorList`, `InvalidErrorsList.tsx`).
- **observation point** — where an inline effect is read: the untouched
  voting screen, the touched voting screen, or the review screen.
  Production factors this as two screens × the `isTouched` flag; the
  untouched-review combination does not arise, so there are three. It
  indexes the mapping's *output*, not its input.

## Intent: all behaviour, and what "all" requires

The goal is to capture **the whole behaviour of the validation subsystem**,
not a subset. Every runner predicts from one shared spec — the single
mapping `f(config, voteState)`; a rule's `predict()` is a thin call into
it (see [Harness](#harness)). That claim rests on two independent
completeness arguments, and only the first is delivered by running more
cells:

1. **Input completeness** — every cell of the input domain reached by
   some mechanism: exhaustively where production can be driven headlessly,
   by equivalence class where only the booth will do, cell by cell for the
   rest (see [Coverage](#coverage) below, which names which mechanism
   carries which role).
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
| `InvalidErrorsList.tsx`, `VotingScreen.tsx`, `ballotSelectionsSlice.ts` | booth inline messages, dialogs, persisted record | **covered** (checker emissions → gate pair → inline filter) |
| `voting_screen.rs` | gates | **covered** (the gate pair) |
| `normalize_vote.rs`, `character_map.rs`, codec internals | round-trip plumbing | **covered** implicitly by the checker emissions (they run inside the recorded decode) |
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
| **multi_ballot codec** — the multi-contest decode (six checkers, the ballot-level decline bit, the 30-byte capacity) | out of scope: the workbench encrypts per-contest only (`raw_ballot`), so no browser runner can reach it (README, Known gaps). The classifier's decline cells are characterized headlessly (`classifier-table.md`) | the `multi_ballot` encrypt/decrypt lift — the same lift as the decline booth flow (README "What's next"). First target when it opens: upstream's own single/multi divergence report at n = 0 (meta#8235 error 5: blank warn in one codec, undervote warn in the other) |
| **encoding-error emissions** — write-in overflow (`writeInCharsExceeded`), `invalidMinVotes` / `invalidMaxVotes` | out of scope: no bundled fixture carries a write-in candidate (FIXTURE_VARIANCE.md §8) and every grid's bounds are sane. The spec lists the keys only for hard-gate faithfulness (`EXPLICIT_OR_ENCODING`) | a write-in fixture, a malformed-bounds slice, or an adjudication question that turns on these cells |
| **config-rejection outcomes** — `check_contest_configuration` refuses a contest with two explicit-blank (or two explicit-invalid) marker candidates, before any per-ballot rule runs | out of scope: a distinct outcome class ("config rejected"), headlessly reachable but on no grid | a cheap headless slice (construct the two-marker contest) whenever that class needs pinning |
| **gate composition across contests / pages** | the spec and every grid are per-contest. Production's gates iterate all contests and fire if ANY matches (VOTE_VALIDATION.md, "Interaction with pagination") — plain OR-composition over the per-contest predicate the spec models; the composition itself is unvalidated | a multi-contest grid (e.g. on `mixed-3contests`) if the OR-composition is ever in doubt |
| **message parameters** — every checker warning is an `InvalidPlaintextError` carrying a `message` (the i18n key identifying the warning, e.g. `errors.implicit.selectedMax` — what the tables' *errors*/*alerts* cells show and what `data-warn-id` exposes in the DOM) and a `message_map` (the values interpolated into the translated text, e.g. numSelected/min/max — the booth renders `t(message, message_map)`, `InvalidErrorsList.tsx`) | comparisons are over the `message` keys only — in the checker record, at the gates, and in the rendered DOM alike; the interpolated `message_map` values are never checked | a finding that turns on a wrong interpolated value (say, the wrong maximum printed in an otherwise-correct warning) rather than a wrong message key |
| **marker preconditions as fixture choice** — `has_explicit_blank/invalid_candidate` | not spec inputs: they gate which vote-states are REACHABLE (a marker state needs a marker candidate), not what the mapping says about a state that exists. Each grid picks a fixture carrying its rule's preconditions | a rule whose EFFECTS (not merely reachability) turn out to depend on marker presence |
| **untouched voting view** — the voting screen before the voter's first selection in the contest (`isTouched`, Question.tsx state, armed when a selection appears). While untouched, `filterErrorList` unconditionally empties BOTH warning lists, so nothing renders inline even when the checker has already emitted warnings (e.g. `selectedMin` on a just-opened screen with `min_votes: 1`); the spec carries the view as the constant `votingUntouched: []` | validated as a constant, per cell: before arming the touch, `dom-validate.mjs` asserts the untouched view renders NOTHING on every one of its 229 cells — whatever the checker emitted — and fails the run otherwise | any code path that renders inline content on an untouched contest (a warning exempted from the clear, or the clear gaining a condition) — the constant would stop being one, and the view would need per-cell treatment like the other two |

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
3. **Observations are pure per observation point.** An inline-visibility
   cell records only what is visibly rendered there. "The alert exists but
   is hidden" is a *derived* fact — the join of the checker emissions
   (emitted) with the inline view (not visible) — and belongs in a
   combined view labelled as such, never in a raw observation cell.
4. **Roles of the artifacts.** The recorded JSON is the *characterization*
   (evidence, and after adjudication the regression oracle); the
   *specification* is the typed Rust crate
   [`../validation-spec/`](../validation-spec/) —
   the whole mapping as one function `f(config, voteState)`, every
   category and intermediate above — with the per-cell meaning of each
   rule's grid defined once in
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
6. **A tally must follow a decode.** The tally classifies *decoded*
   ballots — decode is what populates the `invalid_errors` that
   `is_invalid()` reads, so feeding a hand-built selection straight into
   `tally_decoded_ballots` classifies a checker-clean ballot. Any harness
   step that tallies must run the encode → decode round trip first,
   exactly as the production pipeline does.

## Harness

Four kinds of instrument: one headless runner per rule, one shared browser
validator, the dependency pipeline, and the end-to-end crypto runners.
The first two observe the mapping directly, and split where the production
code does — Rust that Node can call, versus TypeScript that only a browser
runs. (Which production *roles* this covers is the table in
[Coverage](#coverage), against the functional model in
[`docs/VOTE_VALIDATION.md`](../docs/VOTE_VALIDATION.md).)

- **Checker emissions + gate pair, headless — per rule.** `harness.mjs`
  loads the sequent-core wasm package directly in Node — no browser, no dev
  server — and calls the same entry points the booth calls
  (`test_contest_reencoding_js`, `check_voting_not_allowed_next`,
  `check_voting_error_dialog`). Requires `packages/sequent-core/pkg/` (built
  by the workbench's `predev`, or `yarn build:sequent-core`).
- **Inline filter, input constraint and dialog wiring, browser — shared.**
  [`dom-validate.mjs`](dom-validate.mjs) drives every cell of every rule
  through the real booth via Playwright against the dev server on :5173,
  setting config through the **Policy-overrides panel** (the reviewer path)
  and navigating reload-free. Inline visibility is observed at two of the
  three observation points — the touched voting screen (a deterministic
  tick-untick arms the touch per cell) and the review screen — through the
  `data-warn-id` attribute every WarnBox carries (upstream #2832), which
  yields raw message keys — no i18n ambiguity. The third, the untouched
  view, is asserted empty on every cell instead of recorded.

Every prediction comes from **the spec** —
[`../validation-spec/`](../validation-spec/), a typed Rust crate exposing
one function `f(config, voteState)` that covers all four effect categories
plus both checkable intermediates: emissions, gate pair, inline visibility
at each observation point, reachability, and the tally class. Runners
reach it through [`rust-spec.mjs`](rust-spec.mjs), which batches cells
through the crate's `emit-grid` binary; the cell definitions live in
[`rule-specs.mjs`](rule-specs.mjs) (`specConfig` / `voteState` / `RULE_ROWS`
— what each row means in spec terms). The crate is **bug-compatible**, with
every surprising behaviour carried as a named **quirk** (`quirks()` in its
`lib.rs`, each tied to its UPSTREAM_FINDINGS.md suspect — toggling one is an
adjudication decision, not a refactor).

### Two layers: evidence, then analysis

The suite divides by what a tool is *for*, and the division is the reason
its story fits in one head.

**The evidence layer** establishes one claim: **production ≡ the spec**.
Everything in it touches real production code — the sweep headlessly,
`dom-validate` / `quotient-validate` / `browser-witnesses` in a real
browser, `classifier-table` on the tally classifier. It says nothing about
whether the behaviour is *good*.

**The analysis layer** consumes the certified spec and never touches
production at all. `effect-dependencies` asks what each effect depends on;
`effect-map` renders that for humans; `no-silent-discount` evaluates a
property over the spec's certified domain. A finding is something derived
here, not something noticed somewhere.

**Documentation** is a third thing again: the seven per-rule tables
(`rule-tables.mjs`) render the certified spec for a reader. They cannot
fail and they are not evidence.

The spec used to be written **twice** — a JS transcription that carried all
the production evidence, and the Rust port, which inherited it
transitively. That is gone: the runners target the Rust crate directly, so
the artifact this project produces is the one the evidence is attached to,
and the chain is one link rather than two. What the JS spec bought — two
independent transcriptions agreeing — was corroboration between specs, never
production evidence, and it was traded deliberately for evidence that
attaches where it matters (`../docs/EVIDENCE_RESTRUCTURE.md`).

**What can validate each part of the spec.** The halves are unequally
served, because production splits them. The spec's emissions, gates and
classifier describe Rust that IS compiled to wasm, so they are
**wasm-checkable** against the real wasm on every cell of the swept domain.
Its inline views and reachability describe TypeScript that is NOT callable
headlessly (`filterErrorList`; the input disable; the blank-marker
clearing), so they are **browser-only** — validated against the real DOM by
a browser runner, and never against a re-computation of themselves (that
check would be tautological). The per-cell DOM validator is
[`dom-validate.mjs`](dom-validate.mjs), which drives every cell of **all seven
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

Beyond the grids, the **dependency-validation pipeline** closes the
coverage question the per-rule slices cannot answer (they validate only
their own slice of the input space): `effect-dependencies.mjs` enumerates
every dependence and independence claim the spec makes, with an
executable **witness** per dependence — a concrete pair of cells that
differ in that one input and produce different values, which is what
settles an existential claim; `headless-sweep.mjs` discharges the
headless independence claims by exhaustion (production ≡ spec on all
248,320 **representable** cells — those a bundled fixture can actually
drive; `cell.mjs`'s `representable()` names the rest, with the
reason each is out of reach) and emits the **quotient inventory** — the
classes defined just below, with one representative cell each;
`browser-witnesses.mjs` booth-confirms the browser-side dependence
witnesses; `quotient-validate.mjs` discharges the browser-side
independence claims by **sufficiency** — the filter reads the inputs only
through a computed summary, so cells sharing that summary must behave
alike and one booth run settles the whole class (one run per reachable
emissions × consulted-policies class — 130,048 cells covered via 2,208
classes). Zero disagreements on every stage; everything unreachable is
labelled with its unblocking condition.

## Running the analysis

All commands run from `packages/workbench`; each tool writes its artifacts
next to itself in `characterization/`. They are grouped by what they are
FOR — the two layers, plus documentation — and then by whether they need a
dev server.

### Evidence — headless (needs the sequent-core wasm and `cargo`)

The wasm lives at `packages/sequent-core/pkg/` (built by the workbench's
`predev`, or `yarn build:sequent-core`). These compare **real production**
against the spec.

| command (`node characterization/…`) | produces |
|---|---|
| `headless-sweep.mjs` | `headless-sweep.md` + `.recorded.json` — production (real WASM checker/gates/tally) vs the Rust spec on **every** cell of the representable headless domain: all six policies × sane bounds × plurality states × the reachable preferential states; **248,320 cells, ~1 min**. This is the exhaustive fidelity check, and it emits the quotient inventory the browser stage consumes |
| `classifier-table.mjs` | `classifier-table.recorded.json` + `.md` — velvet-wasm's real tally vs the spec's `classify` on the classifier's own 32-cell cross-product, including the decline inputs the decode cannot reach |

### Evidence — browser (dev server on :5173)

Start the dev server first (its `predev` builds the wasm):

```
corepack yarn workspace "@sequentech/workbench-app" dev
```

| command (`node characterization/…`) | produces | what it does |
|---|---|---|
| `dom-validate.mjs` | `dom-validate.md` + `.recorded.json` | the **complete** tables — the Rust spec's predictions vs the real DOM across every cell of all seven rules: inline visibility at **both** observation points (touched voting screen and review), reachability, and the untouched view asserted empty per cell; 229/229, ~9 min |
| `quotient-validate.mjs` | `quotient-validate.md` + `.recorded.json` | the browser-side *independence* claims discharged by sufficiency — one booth-formable member per quotient class from `headless-sweep.md`, inline compared at both observation points, re-observed up to 3× on mismatch; the license (the filter's props boundary) is source-verified and stated with its re-entry condition; classes with no formable member labelled. ~1 h |
| `browser-witnesses.mjs` | `browser-witnesses.md` + `.recorded.json` | every browser-pending dependence witness from `effect-dependencies.md` (inline-view and reachability components) driven through the real booth on a generic per-cell recipe, both cells compared against the spec; unobservable witnesses labelled |

### Evidence — the findings' end-to-end chain (dev server)

| command (`node characterization/…`) | produces | what it does |
|---|---|---|
| `reproduce-verify.mjs` | `reproduce-verify.recorded.json` | runs the three runners below in sequence and aggregates one pass/fail |
| `overvote-e2e-pipeline.mjs` | `overvote-e2e-pipeline.recorded.json` | S1 over-vote: booth → cast → decrypt → decode → tally in one continuous run |
| `minvote-e2e-pipeline.mjs` | `minvote-e2e-pipeline.recorded.json` | S2 below-min (all four cells), same full pipeline |
| `invalid-latent-choices-e2e.mjs` | `invalid-latent-choices-e2e.recorded.json` | S5 null-vote choice leakage, same full pipeline |

### Analysis (needs `cargo` only — no wasm, no browser)

These consume the certified spec. None of them observes production.

| command (`node characterization/…`) | produces |
|---|---|
| `effect-dependencies.mjs` | `effect-dependencies.md` + `.recorded.json` — the effect-first decomposition: per effect component, its support, its conditional-independence restrictions, and one executable witness per dependence, computed exhaustively over the full modelled domain (~29M evaluations). Each witness is checked for membership of the swept domain, so its production backing is inherited from the sweep rather than re-observed; anything outside is reported and fails the run. ~2 min |
| `effect-map.mjs` | `effect-map.md` — the human projection of the dependency ledger: the mapping as a causal diagram (Mermaid; topology checked against the ledger on every run), the functional-cancellations table, and per-knob cards. Pure JSON → markdown; instant |
| `no-silent-discount.mjs` | `no-silent-discount.md` + `.report.json` — the no-silent-discount property, evaluated over every cell the sweep certifies. Fails if a violation already escalated as S1/S2 stops being found |

### Documentation

| command (`node characterization/…`) | produces |
|---|---|
| `rule-tables.mjs` | the seven per-rule tables (`blank-rule.md`, `overvote-rule.md`, …) — a rendering of the certified spec across each rule's grid. Cannot fail; not evidence |

Every evidence and analysis runner exits nonzero on failure, so they
compose in CI. Note that piping one through `tail` masks that: a shell
pipeline reports the LAST command's status.

`harness.mjs`, `rust-spec.mjs`, `rule-specs.mjs`, `browser-harness.mjs`,
`cell.mjs`, and `booth-cell.mjs` are shared modules (imported, not run).

## Coverage

Coverage does not come from one mechanism. **Three** carry it, and which
one applies is decided by what production makes reachable, not by
preference:

1. **Exhaustion.** [`headless-sweep.mjs`](headless-sweep.mjs) enumerates
   the input domain directly — all six policies × sane bounds ×
   plurality vote states — and compares production against the spec on
   every one of its **248,320** cells. It is rule-agnostic: nothing in it
   knows what a rule is.
2. **Sufficiency.** Where the booth is far too slow to enumerate,
   [`quotient-validate.mjs`](quotient-validate.mjs) covers by equivalence
   class instead. The inline filter reads its inputs only through
   (emissions, the four consulted policies, observation point), so one
   booth run settles every cell sharing that summary: **130,048** cells
   via **2,208** runs, on a license that is source-verified and carries a
   re-entry condition.
3. **Cell by cell.** For what neither reaches, three instruments:
   [`dom-validate.md`](dom-validate.md) drives all **229** grid cells
   through the real booth on the reviewer path; the two preferential
   grids ([`duprank-rule.md`](duprank-rule.md),
   [`prefgaps-rule.md`](prefgaps-rule.md)) carry the **8** rule-triggering
   cells apiece that the sweep's plurality-only domain excludes; and
   [`browser-witnesses.md`](browser-witnesses.md) settles each
   existential dependence claim from
   [`effect-dependencies.md`](effect-dependencies.md) with one cell pair.

The seven per-rule **tables** carry no coverage at all, and no longer
pretend to. All 248 of their cells fall inside the sweep's domain
(verified), so the sweep observes production on every one of them; the
tables are a rendering of the spec for a reader, not a separate check
(`rule-tables.mjs`).

The functional model in VOTE_VALIDATION.md has **six roles**, and full
behaviour — `f(config, voteState) → one value per effect category` — is
characterized only when all six are. Zero disagreements anywhere; each
artifact's own legend carries its labels and residues.

| Role | Covered by | Status |
|---|---|---|
| Checkers | exhaustion | production ≡ spec on every swept cell (`headless-sweep.md`, **248,320**, plurality and preferential). One sweep serves **both** bands: the tally decode runs the identical function, so this is also the tally-side checker characterization |
| Gates | exhaustion | both gates **and** the dialog projection on all 248,320 swept cells (`headless-sweep.md`) |
| Filter | per-cell, then sufficiency | `dom-validate.md`: every grid cell through the real booth via the reviewer path, inline observed at both observation points, the untouched view asserted empty per cell — **229/229**. Beyond the grids, the independence claims are discharged by sufficiency (`quotient-validate.md`, 2,208 classes / 130,048 cells) |
| Input constraint | per-cell | the `reachable` column of `dom-validate.md` across every rule cell (the state forms or it does not), **plus** direct evidence for both prevention mechanisms: the over-vote `disable` policy (`no (disabled)`, from probing the (max+1)th control's `disabled` attribute) and blank-marker exclusivity (`no (cleared)`, the marker collapsing a co-selected regular) |
| Marker exclusivity (prevention) | per-cell, plus the crypto chain | characterized by *attempting* each state through the UI and recording whether it forms. Both directions recorded in `dom-validate`: the invalid marker does **not** clear (`marker_plus` forms — reachable `yes`, confirmed end-to-end by `invalid-latent-choices-e2e.mjs`), the blank marker **does** (`regular_then_marker` collapses to {marker only} — `no (cleared)`). Open: the decline booth flow |
| Tally classifier | exhaustion, plus a direct decision table | the `tally` column on every swept cell, plus the standalone 32-cell six-class table (`classifier-table.md`, 32/32 matching the documented precedence — and the only production evidence for decline) |

Prevention produces no *message*, but it is not outside the mapping: the
last two roles are how production realizes the **reachability** effect
category, which `f` returns like any other (`yes` / `inputs_disabled` /
`marker_cleared`). Their output is therefore a reachability table
(state × config → forms / does-not-form) — which is also exactly the data
needed to justify, or refuse, pruning cells from the other enumerations.

Two things sit outside this table because they are not role coverage:
the analysis layer ([`effect-dependencies.md`](effect-dependencies.md),
[`effect-map.md`](effect-map.md),
[`no-silent-discount.md`](no-silent-discount.md)), which consumes the
certified spec rather than observing production, and the
three `*-e2e` pipelines orchestrated by `reproduce-verify.mjs`, which
confirm the *findings* — the five silent-discount cells and S5 — booth →
encrypt → cast → decrypt → decode → tally.

### no-silent-discount — the standing property report

`no-silent-discount.mjs`, observation-based end to end: 248 recorded
cells → 7 candidates (`tally = ImplicitInvalid` ∧ no gate, both real WASM
observations) → **5 booth-confirmed silent discounts** in two families
(over-vote `allowed × allowed`; min-vote under `invalid = allowed`), all
requiring `invalid_vote_policy = allowed`. Escalated as S1/S2 in
[`../docs/UPSTREAM_FINDINGS.md`](../docs/UPSTREAM_FINDINGS.md);
click-by-click recipes in [`../docs/REPRODUCE.md`](../docs/REPRODUCE.md);
the admin-lint-shaped configuration table in
[`no-silent-discount.md`](no-silent-discount.md).

## Marker-inclusive counting caveat

The vote-state classes are defined over `num_selected_with_markers`: the
`marker_only` state (explicit-blank marker selected, nothing else) counts
**1**, so it is *not* blank at the booth — verified in the recording (no
blank checker output, no gate) — while classifying as `ExplicitBlank` at
tally. See VOTE_VALIDATION.md "Selection counting and marker candidates".

## Adding a rule

1. **Spec.** Add the rule's checker emissions to
   [`../validation-spec/src/lib.rs`](../validation-spec/src/lib.rs)
   (transcribed from checker.rs, in decode order); any surprising
   behaviour it carries gets a named entry in that crate's `quirks()`.
   There is one spec now, so there is one place to do this.
2. **Cells.** Add a `RULE_SPECS` entry in
   [`rule-specs.mjs`](rule-specs.mjs) (`specConfig` / `voteState` — what a
   row means in spec terms) and its grid to `RULE_ROWS`. Pick or extend a
   bundled fixture whose contest carries the rule's preconditions (see
   FIXTURE_VARIANCE.md §13.2 for why marker candidates are preconditions,
   not policies).
3. **Fidelity.** If the rule's states are reachable headlessly, extend
   `cell.mjs` so the sweep can drive them — that is what puts the rule's
   behaviour under the exhaustive production check, and it is the only
   step that produces evidence.
4. **Browser.** Extend the `RULE_SPECS` entry with the browser-driving
   half (contest selector, panel config, selection clicks, landmark,
   reachability checks) and add a `RULES` entry in
   [`dom-validate.mjs`](dom-validate.mjs). The rule's cells then join the
   DOM-validated complete tables.
5. **Documentation.** Add the rule to `LEGENDS` in
   [`rule-tables.mjs`](rule-tables.mjs) to get its per-rule table.

## Open work

The suite's own backlog — changes to *these tools*. Decisions for other
people (consultation on the findings, whether to migrate the runners onto
the Rust spec) live in the root [README](../README.md)'s "What's next".

Every residue below is already labelled in the artifact that carries it;
this is the one place they are collected, ordered by how much each would
unblock.

- **A both-markers fixture.** Purely a *booth* gap: no bundled contest
  gives a voter a route to both markers at once — the explicit-invalid
  flag needs the Council contest's null-vote marker, the blank marker
  needs Referendum — so cells needing both cannot be formed by clicking.
  That defers 13 dependence witnesses
  ([`browser-witnesses.md`](browser-witnesses.md)) and part of the 4,384
  deferred quotient classes
  ([`quotient-validate.md`](quotient-validate.md); most of those are
  preferential, awaiting the IRV booth recipe below). Headlessly the
  combination is already covered: the sweep varies `blankMarker ×
  explicitInvalid` together, since Referendum accepts the flag without
  needing a marker candidate. The single widest unblock.
- **A generic IRV booth recipe.** The headless half of this is closed:
  `cell.mjs` routes ranked cells to the IRV fixture and the sweep now
  covers them. What remains is the booth — the witness and quotient
  stages have no generic way to rank candidates by clicking, so 8
  witnesses defer.
- **Witness preference.** `analyze_deps`'s `representability_score`
  prefers cells the harness can drive, but does not penalize
  marker + flag; teaching it to would shrink the zero-evidence witness
  set with no new fixture — the cheapest item here.
- **no-silent-discount as a spec property.** The query pre-filters
  recorded cells, then confirms candidates in the booth. Now that the
  spec is production-certified over the swept subdomain, the property
  could be evaluated over its full domain instead, with the booth
  reserved for confirming hits — turning a sampled query into an
  exhaustive one.
- **The decline-to-vote booth flow** — the classifier's decline cells are
  recorded headlessly (`classifier-table.md`), but no booth-side runner
  drives a declined ballot. Not a suite change: it is blocked on adding a
  `multi_ballot` encrypt/decrypt path, so it is a feature lift (root
  README, Known gaps).
