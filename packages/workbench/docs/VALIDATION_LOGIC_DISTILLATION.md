<!--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Validation Logic Distillation

A formal decomposition of vote-validation behaviour into observable effects,
input dimensions, and a declarative mapping — with the goal of enabling
alternative implementations that reduce accidental complexity.

This document emerges from [VOTE_VALIDATION.md](./VOTE_VALIDATION.md) but
focuses on the *specification* layer rather than the *implementation* layer.

---

## 1. Observable Effects Taxonomy

The system responds to a voter's selections through a small set of
**surfaces** — concrete places where a response can be perceived: the
inline warning box under a contest, the dialog that may open on clicking
Next, the enabled/disabled state of the selection inputs themselves,
and (after casting) the tally's classification of the ballot. What
appears on each surface depends not only on the configuration and the
selections but also on the **observation context** — *when and where we
look*: the voting screen versus the review screen, and whether the
voter has touched the contest yet.

In those terms: every (voter-state × contest-configuration ×
observation-context) tuple produces one **observable effect per
surface**, drawn from the following set — see the per-surface
refinement at the end of this section for why "exactly one effect"
holds per surface rather than per tuple.

**The set's closedness is a checkable claim, not an assumption.** It is
closed relative to a **consumer census**: the enumeration of every read
site of the validation state (`invalid_errors` / `invalid_alerts`, the
policy fields, the marker and decline flags) in the codebase, each mapped
to an effect surface here, an explicit out-of-scope entry, or a named gap.
No amount of input-cell enumeration can establish this — a channel the
harness doesn't record stays invisible at every cell. The census lives in
[`../characterization/README.md`](../characterization/README.md); its
first run found two channels this taxonomy did not list (the standalone
ballot-verifier's display of decoded ballots, and velvet's rendered
ballot images). Both were subsequently scoped **out** of the census by
decision — the verifier until it is ever lifted into the workbench,
ballot images unless that functionality ever arrives here — and the
defects found in the ballot-images consumer are recorded in
[`UPSTREAM_FINDINGS.md`](./UPSTREAM_FINDINGS.md) for reporting. The
census's value is precisely that these are now *named decisions* rather
than unknown unknowns.

### Casting-time effects (booth UI)

| # | Effect | Description |
|---|--------|-------------|
| 1a | Inline message displayed | Voter sees text below the contest (warning or error) |
| 1b | Dismissible dialog shown | Modal informs voter; voter may dismiss and continue |
| 1c | Non-dismissible dialog shown | Modal blocks; voter must fix the selection to proceed |
| 1d | Input constraint applied | UI prevents reaching the state (e.g. checkboxes disabled) |
| 1e | Silent | No signal — voter is unaware of any policy concern |

### Tally-time effects

The tally effect is the ballot's **class** — one of six, assigned by
`classify_ballot`
([`velvet-core/src/counting/extended_metrics.rs`](../velvet-core/src/counting/extended_metrics.rs))
with a strict precedence (decline → invalid → blank-marker → implicit-blank
→ valid; see VOTE_VALIDATION.md "Tally-Time Classification" for the exact
rules, including that a non-empty declined ballot and a marker-mixed ballot
both land in `ImplicitInvalid`):

| # | Class | Aggregation consequence |
|---|-------|-------------------------|
| 2a | `Valid` | Contributes to candidate tallies; counts in `total_valid_votes` |
| 2b | `ExplicitInvalid` | `invalid_votes.explicit`; excluded from valid |
| 2c | `ImplicitInvalid` | `invalid_votes.implicit`; excluded from valid |
| 2d | `ExplicitBlank` | `blank_votes.explicit`; **counts in `total_valid_votes`** |
| 2e | `ImplicitBlank` | `blank_votes.implicit`; **counts in `total_valid_votes`** |
| 2f | `Declined` | `total_declined_to_vote`; **excluded from `total_valid_votes`** |

Note the aggregation column: "valid" and "contributes to candidate tallies"
are distinct observables (blanks are valid but contribute to no candidate),
and the blank/declined distinction is precisely a disagreement about the
valid total. (The six-class taxonomy dates from the 2026-08 merge —
explicit blanks #2842, decline-to-vote #2687.)

**Key principle:** Effects are *atomic observables*. Timing and location are
part of the *input space*, not the effect taxonomy. For example,
`WARN_ONLY_IN_REVIEW` is not a distinct effect — it is the same effect (1a)
mapped from a different observation point:

```
(undervote, WARN_ONLY_IN_REVIEW, during_voting)   → 1e (silent)
(undervote, WARN_ONLY_IN_REVIEW, on_review)        → 1a (inline message)
```

**Refinement — casting effects are per-surface, not exclusive.** A single
input tuple can legitimately produce an inline message (1a) *and* a dialog
(1b/1c) simultaneously — e.g. an overvote under
`NOT_ALLOWED_WITH_MSG_AND_ALERT` shows inline text during voting and a
hard dialog on transition. "Exactly one effect" holds only per surface.
The casting codomain is therefore a product:

```
CastingEffect = (inline: Set<Message>,
                 gates: (hard: bool, soft: bool))   // dialog = a projection
```

Two amendments to an earlier version of this type, both forced by what
the characterization verified (`../characterization/spec.mjs` is the
executable form):

- **The gate surface is two independent booleans, not one three-valued
  outcome.** Production evaluates two separate functions
  (`check_voting_not_allowed_next_util` and
  `check_voting_error_dialog_util`), and both can be true at once
  (recorded: the `not-allowed` rows of
  `../characterization/invalid-rule.md` trip both). The dialog the voter
  actually meets is a projection — `hard ? blocking : soft ? dismissible
  : none` — and that projection is what `dom-validate.mjs` checks
  against the real DOM. Typing the surface as a single
  `{none | dismissible | blocking}` value would erase the
  both-gates-fire fact.
- **The input constraint (1d) is no longer part of the effect product.**
  Prevention prunes which states the booth UI can *produce*; it does not
  change the mapping over states that exist anyway — hand-built or
  decoded records still run through the checkers as defense-in-depth
  (the §2 pruning caution). It is therefore modelled as a separate total
  function over the same inputs:

  ```
  reachability(config, vote_state) → yes | inputs_disabled | marker_cleared
  ```

  with two prevention mechanisms: the DISABLE over-vote policy disabling
  further inputs at max (`inputs_disabled`), and the blank marker
  clearing co-selected regulars (`marker_cleared` — observed as
  `no (cleared)` in `../characterization/dom-validate.md`; the invalid
  marker deliberately does not clear — finding S5). Effect 1d in the
  table above names the voter-perceived face of the first mechanism; the
  second is imperceptible until tried.

The tally codomain *is* single-valued: exactly one class per ballot.

---

## 2. Input Space Dimensions

### Contest configuration (static per election)

| Dimension | Domain |
|-----------|--------|
| `min_votes` | ℕ |
| `max_votes` | ℕ (≥ min_votes) |
| `invalid_vote_policy` | {ALLOWED, WARN, WARN_INVALID_IMPLICIT_AND_EXPLICIT, NOT_ALLOWED} |
| `over_vote_policy` | {ALLOWED, ALLOWED_WITH_MSG, ALLOWED_WITH_MSG_AND_ALERT, NOT_ALLOWED_WITH_MSG_AND_ALERT, NOT_ALLOWED_WITH_MSG_AND_DISABLE} |
| `under_vote_policy` | {ALLOWED, WARN, WARN_ONLY_IN_REVIEW, WARN_AND_ALERT} |
| `blank_vote_policy` | {ALLOWED, WARN, WARN_ONLY_IN_REVIEW, NOT_ALLOWED} |
| `duplicated_rank_policy` | {ALLOWED_WARN_AND_DIALOG, NOT_ALLOWED_WARN_AND_DIALOG} |
| `preference_gaps_policy` | {ALLOWED_WARN_AND_DIALOG, NOT_ALLOWED_WARN_AND_DIALOG} |
| `counting_algorithm` | {Plurality, Preferential} (simplified) |
| `has_explicit_invalid_candidate` | {true, false} — marker presence is a **precondition**: without it `is_explicit_invalid` is unreachable from the booth |
| `has_explicit_blank_candidate` | {true, false} — same precondition role for the blank dimensions; also changes the wire layout (adds the `choices[1]` flag slot) |
| `decline_to_vote_policy` | {DISABLED, ENABLED} — election-level; ENABLED adds the decline bit to the multi-ballot layout |

Configuration validity is itself an input class: `check_contest_configuration`
rejects a contest with more than one explicit-blank marker, producing a
"config rejected" outcome before any per-ballot rule runs.

### Vote state (dynamic, per voter action)

| Dimension | Domain |
|-----------|--------|
| `num_selected_class` | {0, (0..min), [min..max), max, (max..∞)} — 5 equivalence classes over the **marker-inclusive** count (`num_selected_with_markers`): a selected marker counts as a selection, so an explicit-blank marker alone is in class `[min..max)` for `min_votes: 1`, not class 0 |
| `is_explicit_invalid` | {true, false} |
| `is_explicit_blank_selected` | {true, false} — and, when true, whether a regular candidate is *also* selected (the mix rule flips the tally class to `ImplicitInvalid`) |
| `is_decline_to_vote` | {true, false} — multi-ballot only; interacts with emptiness (declined + non-empty → `ImplicitInvalid`) |
| `has_duplicate_ranks` | {true, false} (preferential only) |
| `has_rank_gaps` | {true, false} (preferential only) |
| `has_encoding_error` | {true, false} — write-in corruption / capacity overflow; the hard gate's first condition fires on `EncodingError`, which no combination of the other dimensions can express |

### Observation context

| Dimension | Domain |
|-----------|--------|
| `observation_point` | {during_voting, on_review, on_transition} |
| `is_touched` | {true, false} |

### Combinatorial size

The naive worst-case product is large and not the useful number — the
config side alone is 4·5·4·4·2·2·2·2·2·2 = 20,480 combinations before the
vote-state and context factors.

The useful observation is that the mapping **decomposes**: each rule reads
at most three dimensions (its own policy, one or two vote-state fields, and
the observation point), so the table factors into per-rule slices of a few
dozen cells each, plus the classifier's six-class decision table over
(decline, invalid, blank-marker, emptiness). Exhaustive enumeration is
tractable *per rule*; the full cross-product never needs to be materialised.
Cross-rule interactions that do exist (the blank checker's dependence on
`is_explicit_invalid`, the master filter's dependence on three policies at
once, the mix rule) are exactly the cells worth enumerating jointly — and
there are few of them.

Two pruning cautions:

- **Do not prune "unreachable" cells.** `num_selected > max` under
  `NOT_ALLOWED_WITH_MSG_AND_DISABLE` looks unreachable, but that is
  *prevention* — UI-enforced, hence fragile. Upstream bug `fdc7f92db5`
  ("decline-to-vote with overvote-disable still allows selecting an
  additional candidate") is a recent, real instance of a prevented state
  being reached. The checkers validate these states as defense-in-depth;
  the specification must define their effects too.
- Preferential-only flags genuinely don't apply to plurality (the codec
  never evaluates them) — that pruning is structural, not preventive, and
  is safe.

---

## 3. The Mapping (specification)

The system's behaviour is fully characterised by a pure function:

```
f(config, vote_state, observation_context) → Effect
```

Today this function is *implicitly* encoded across multiple code paths:

- `checker.rs` (9 checker functions producing errors/alerts — 8 decode-time
  plus the config-level `check_contest_configuration`)
- `voting_screen.rs` (gating utilities computing "can proceed?", with their
  own marker-inclusive selection count)
- `InvalidErrorsList.tsx` (filtering errors by `isReview` and by the
  `InvalidVotePolicy` master filter)
- `Question.tsx` (disabling inputs based on policy)
- `VotingScreen.tsx` (wiring gate results to dialog/button state)
- `ballotSelectionsSlice.ts` (the marker exclusivity rule — prevention
  implemented in a Redux reducer)
- `velvet-core/src/counting/extended_metrics.rs::classify_ballot` (tally
  classification — **already declarative**; see §5.2)

A **declarative specification** would make this function explicit: a table
(or set of rules) that, given the input tuple, returns the effect. The
implementation would then be a single interpreter that evaluates the table.

---

## 4. Design Observations

### 4.1 Enforcement vs. Prevention

Two qualitatively different mechanisms exist:

- **Prevention** (effect 1d): The voter literally cannot reach the invalid
  state — inputs are constrained (e.g. checkboxes disabled at max_votes).
  The invalid condition is eliminated from the state space.
- **Enforcement** (effects 1b, 1c): The voter *can* reach the invalid state
  but is blocked from proceeding past a gate. The state exists transiently.

These are not interchangeable. Prevention is stronger (no invalid state ever
exists) but less flexible (voter cannot explore "what if?").

### 4.2 Booth cannot guarantee tally-time validity

The booth validates and warns/blocks, but a voter who dismisses a 1b dialog
can cast a ballot that will be classified 2c (implicit-invalid) at tally.
This is by design — the policies control *enforcement strictness*, not
*tally-time validity*. Specifically:

- `OverVotePolicy::ALLOWED_WITH_MSG_AND_ALERT` → voter CAN cast an overvote
  (after dismissing dialog) → at tally, vote is classified implicit-invalid.
- `InvalidVotePolicy::ALLOWED` **+ `OverVotePolicy::ALLOWED`** → voter sees
  NO warnings → at tally, errors still present → vote classified
  implicit-invalid. (Both policies must be fully permissive for total
  silence: under the *default* overvote policy the master filter still lets
  the overvote error through — see VOTE_VALIDATION.md, "UI filter behavior".)

The silent-cell combination is therefore narrow, but it exists, and the
design question stands: if
a voter receives zero indication (1e) that their vote will be invalid at
tally (2c), is that intentional? The answer depends on whether "ALLOWED"
means "allowed to be cast" or "counted as valid." Current code: it means
"allowed to be cast" — the tally classifies independently.

### 4.3 `InvalidVotePolicy` conflates three concerns

From first principles, these are independent controls:

1. "Allow explicit-invalid candidate?" (boolean)
2. "Show structural error messages to voter?" (visibility enum)
3. "Block submission on structural errors?" (strictness enum)

Packing them into a single enum (ALLOWED / WARN / WARN_INVALID_IMPLICIT_AND_EXPLICIT /
NOT_ALLOWED) creates non-orthogonal combinations. You cannot configure
"show errors but don't block" + "explicit-invalid allowed" without
choosing WARN, which doesn't warn about explicit-invalid at the checker level.

### 4.4 Asymmetry of undervote/overvote

Despite symmetric naming, `under_vote_policy` and `over_vote_policy` govern
different sides of the validity boundary:

- **Undervote zone** (`min ≤ n < max`): structurally valid. The policy is
  purely cosmetic — no variant promotes an undervote to invalid.
- **Overvote zone** (`n > max`): structurally invalid. The policy is also
  purely cosmetic — no variant demotes an overvote to valid. The
  `ALLOWED_*`/`NOT_ALLOWED_*` names describe UI wording, not validity.

The validity boundary is set by `min_votes`/`max_votes`, not by the policy
enums. The enums are "dimmer switches" for the UI on either side of a wall
whose position they cannot move.

### 4.5 The silent-discount criterion (derived from characterization)

The `no-silent-discount` query (run over all seven characterized rules)
yields a precise structural criterion for *which* rules can silently
discard a vote. A rule is **silent-discount-prone iff** it has a
configuration where **all three** hold:

1. its checker emits an `invalid_error` in that configuration — only
   errors reach the tally's `is_invalid()`; alerts have no tally
   consequence, so without an error there is nothing to discount;
2. *its own* policy has a fully **signal-free** variant — one that
   emits no inline alert, retains nothing on the master filter's
   keep-list, and fires no rule-specific dialog. Over-vote `allowed` is
   such a variant; `allowed-with-msg` is *not*, despite not gating — it
   emits an inline alert and the keep-list preserves `selectedMax`
   whenever the policy ≠ `allowed`. A rule with no policy at all
   (min-vote is a fixed `n < min_votes` check) is signal-free by
   default; **and**
3. `invalid_vote_policy = allowed`, which switches off both generic
   surfaces: the generic dialog gate and generic error-visibility (the
   master filter hides every `invalid_error` not on its two-entry
   keep-list — `selectedMax` iff `over_vote_policy ≠ allowed`,
   `blankVote` iff `blank_vote_policy = not-allowed`).

When all three hold, an error the checker produces internally reaches the
tally (via `is_invalid()`) with no booth surface showing or blocking it.
The over-vote and min-vote families are exactly the rules that meet all
three.

The other five rules each fail a specific condition. The preferential
rules (`duplicated_rank`, `preference_gaps`) fail condition 2 **by
construction**: their enums have *only* `*_WARN_AND_DIALOG` variants — no
silent `allowed` — so a gate always fires when their error is present,
whatever `invalid_vote_policy` is. Under-vote fails condition 1: its
checker emits only alerts, never errors, so an under-voted (but
above-min) ballot stays `Valid` at tally. Blank fails conditions 1∧2
jointly: it emits an error only under `not-allowed`, and `not-allowed`
hard-gates. The invalid rule fails condition 1 in the only
configurations condition 3 permits: under `invalid_vote_policy =
allowed` the marker sets a flag, not an error — and its discard class
(`ExplicitInvalid`) is excluded from the property by definition, a null
vote's exclusion from the count being voter-intended. This is also the
shape of a candidate fix for the prone rules: give every error-producing
rule a mandatory dialog variant (remove the silent `allowed` from
over-vote; give min-vote a policy), so condition 2 becomes
unsatisfiable. (Whether to fix at all is a suspect for consultation —
see [`UPSTREAM_FINDINGS.md`](./UPSTREAM_FINDINGS.md) S1/S2 — but the fix
*shape* falls out of the criterion.)

---

## 5. Toward a Declarative Implementation

### 5.1 Goals

1. **Single artifact** that both booth and tally consult (today: checker.rs
   and the gate utilities are shared Rust, but error *visibility* is
   TypeScript (`filterErrorList`), the marker exclusivity rule is a Redux
   reducer, and the tally classifier lives in a different crate).
2. **Exhaustive, inspectable mapping** — every cell of the input space has
   a declared effect, verifiable by enumeration.
3. **Reduced accidental complexity** — no `filterErrorList` / `isReview`
   branching scattered across UI components; the table already encodes which
   observation-points produce which effects.
4. **Testable in isolation** — the pure function `f` can be property-tested
   against the table without rendering any UI.

### 5.2 Sketch

```rust
/// The declarative specification. Returns the casting-time effect for a
/// given input tuple.
fn casting_effect(
    config: &ContestConfig,
    vote: &VoteState,
    ctx: &ObservationContext,
) -> CastingEffect {
    // Pattern-match on equivalence classes, not imperative if-chains.
    match (vote.num_selected_class(config), ctx.point) {
        (Zero, _) if config.blank_vote_policy == NotAllowed
            => CastingEffect::NonDismissibleDialog,
        (Zero, OnReview) if config.blank_vote_policy == WarnOnlyInReview
            => CastingEffect::InlineMessage,
        (Zero, DuringVoting) if config.blank_vote_policy == WarnOnlyInReview
            => CastingEffect::Silent,
        // ...
    }
}

/// Tally-time classification — depends only on config + vote_state.
///
/// THIS FUNCTION NOW EXISTS. The 2026-08 merge shipped it as
/// `classify_ballot` in velvet-core/src/counting/extended_metrics.rs —
/// a pure function over (decline, invalid, blank-marker, emptiness) with
/// exactly the shape sketched here, unit-tested upstream. The tally half
/// of this distillation is therefore already implemented; what remains is
/// the casting half. Actual shape (abridged):
fn classify_ballot(vote: &DecodedVoteContest, blank_ids: &HashSet<String>)
    -> BallotClass
{
    if vote.is_decline_to_vote() {
        if vote.is_blank() { Declined } else { ImplicitInvalid }
    } else if vote.is_invalid() {
        if vote.is_explicit_invalid { ExplicitInvalid } else { ImplicitInvalid }
    } else {
        match (has_explicit_blank, has_regular_selection) {
            (true, true)  => ImplicitInvalid,   // mix rule
            (true, false) => ExplicitBlank,
            _ if vote.is_blank() => ImplicitBlank,
            _ => Valid,
        }
    }
}
```

The booth UI would become a thin renderer:

```typescript
// Pseudocode — the UI consults the specification, doesn't re-derive logic.
const effect = wasmModule.casting_effect(contestConfig, voteState, {
    point: isReview ? "on_review" : "during_voting"
});

switch (effect) {
    case "inline_message": return <WarnBox messages={...} />;
    case "dismissible_dialog": return <Dialog dismissible />;
    case "non_dismissible_dialog": return <Dialog />;
    case "input_constraint": /* already handled at render */ break;
    case "silent": break;
}
```

### 5.3 Migration path

This is not a rewrite proposal. The path is incremental:

> **Status (2026-08-13):** step 1 is complete for the seven rules —
> [`../characterization/`](../characterization/README.md) holds the
> harness, seven recorded rule tables (blank, over-vote, under-vote,
> min-vote, duplicated-rank, preference-gaps, invalid), the tally
> classifier's own decision table, and the single-sourced spec
> ([`../characterization/spec.mjs`](../characterization/spec.mjs) — the
> whole mapping as one function `f(config, voteState, context)`: checker
> emissions, both gates, the classifier, the message filter, and
> reachability — the embryonic declarative table of step 3; each runner
> supplies only its experiment grid, with the per-cell meaning defined
> once in `rule-specs.mjs`).
> The spec is validated in two lanes: the partial (headless) tables check
> its gates/classifier against the real WASM on every cell (`pred?`), and
> the **complete** tables (`dom-validate.mjs`) drive every cell of all
> seven rules through the real booth — panel-driven config, reload-free
> (one snapshot load per rule, then client-side navigation; ~2 s/cell,
> the full 229-cell grid in ~8 min) — observing inline visibility at the
> review screen and reachability, with direct constraint evidence
> (`no (disabled)` from probing the (max+1)th control's `disabled`
> attribute; `no (cleared)` from the blank marker clearing a co-selected
> regular): **229/229 matching the spec**. The §4.5 query
> (`no-silent-discount`) is observation-based end to end: 248 recorded
> cells → 7 candidates (`tally = ImplicitInvalid` ∧ no gate) → **5
> browser-confirmed** silent discounts in two families — the over-vote
> case §4.2 predicted, and a min-vote family the pass discovered
> (`selectedMin` is suppressed under `invalid=allowed`, so a
> below-minimum ballot is silently discarded). Both families require
> `invalid_vote_policy = allowed`; every violating cell is confirmed
> through one continuous booth → encrypt → cast → decrypt → decode →
> tally run (`reproduce-verify.mjs` orchestrates the three e2e runners),
> with click-by-click reviewer recipes in `REPRODUCE.md` and
> policy-intent evidence in `INVALID_VOTE_POLICY_INTENT.md`. The one
> open cell: the decline-to-vote **booth flow** — the classifier's
> decline cells are recorded headlessly, but no booth runner drives a
> declined ballot.

1. **Enumerate the current mapping** — exercise every cell of the input
   space through the existing code and record the observed effects. Include
   the *prevention-guarded* cells (see the pruning caution in §2) and the
   decode-error cells. Most of this needs no UI: the checkers and both gate
   functions are already WASM exports (and `voting_screen.rs` even ships a
   `get_contest_plurality` fixture builder), and `classify_ballot` is
   natively unit-testable in velvet-core. The only layer that resists
   headless enumeration is `filterErrorList`, which is component-internal
   TypeScript — covered instead by the browser lane (`dom-validate.mjs`),
   which observes it per cell in the real booth.
2. **Distinguish three states, not two.** The recording in step 1 is a
   **characterization** — a description of what the code does, bugs
   included (`fdc7f92db5` is a recent example of a bug that enumeration
   would have faithfully frozen). A surprising behaviour is then a
   **suspect**: precisely recorded, but whether it is *intended* is a
   design question that neither the workbench work nor its operator has
   the authority to answer — suspects are escalated for consultation in
   [`UPSTREAM_FINDINGS.md`](./UPSTREAM_FINDINGS.md), with confidence
   intuitions noted (they guide attention; they do not adjudicate). Only
   after consultation does a cell become **adjudicated** — blessed into
   the specification or filed as a defect. The *suspects are the most
   valuable output of the whole exercise* — do not reconcile them
   silently, and do not promote them by intuition alone.
   Characterization without verdicts is already productive (doc
   corrections, upstream defects, the silent-discount families all
   predate any adjudication); the pipeline does not stall waiting for
   blessings.
3. **Express the blessed table declaratively** — either as a Rust match
   expression or as a data structure the workbench can load and visualize.
4. **Verify equivalence** — the declarative version must produce identical
   outputs to the blessed table for every cell; property-test it against
   the live implementation and treat divergences as regressions (or newly
   discovered characterization gaps).
5. **Optionally refactor** — once the declarative version is proven
   equivalent, the scattered imperative code (filter functions, dialog
   utilities, disable-prop wiring) can be replaced by a single interpreter
   that reads the specification. This is optional and can be done
   incrementally per-checker. The tally side demonstrates the end state:
   `classify_ballot` already is this artifact for its half of the mapping.

---

## 6. Relationship to Other Documents

- [VOTE_VALIDATION.md](./VOTE_VALIDATION.md) — describes the current
  implementation architecture (call chains, data flow, component roles).
  This document describes the *desired specification* that the implementation
  should converge toward.
- [FIXTURE_VARIANCE.md](./FIXTURE_VARIANCE.md) — identifies which fixture
  dimensions exercise which code paths, and which combinations currently
  have bundled-fixture inhabitants. Its §13 (post-merge dimensions: marker
  candidates, decline-to-vote, tally sheets) fed directly into §2 above.
  The relationship is complementary, not subsumption: FIXTURE_VARIANCE
  answers "what data exists / is reachable" (sense 1), this document
  specifies "what behaviour is correct over that data" (sense 2).
  Notably, its marker-precondition finding is why §2 carries the
  `has_explicit_*_candidate` config dimensions: a policy dimension without
  its marker precondition is untestable no matter how many fixtures set
  the policy.
