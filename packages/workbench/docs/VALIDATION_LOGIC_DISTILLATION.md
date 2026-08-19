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

The system responds to a voter's selections with a small set of
**effects** — observable behaviours: an inline warning under the
contest, a dialog on clicking Next, inputs that refuse to form a
selection state at all, and (after casting) the tally's classification
of the ballot. This document groups them into four **effect
categories** — inline, dialog, reachability, tally — and the claim it
makes precise is one of **totality and determinism**: every
(contest-configuration × vote-state) cell determines exactly one value
in every category, drawn from the sets below. A category does not care
whether the cell's state was actually formed in a booth or arrived as a
decoded or hand-built record — the mapping associates cells with
effects, nothing more.

One category's value depends on the **observation context** — *when and
where we look*: inline content differs between the voting screen and
the review screen, and an untouched contest shows nothing. The context
is not an input to the mapping; it indexes the inline *value* (one set
of message keys per observation point). The other categories carry no
such index: the dialog appears at one fixed moment (the Next/review
transition), reachability is settled by the interaction attempt, and
the tally class is assigned once at count.

**The set's closedness is a checkable claim, not an assumption.** It is
closed relative to a **consumer census**: the enumeration of every read
site of the validation state (`invalid_errors` / `invalid_alerts`, the
policy fields, the marker and decline flags) in the codebase, each mapped
to an effect category here, an explicit out-of-scope entry, or a named gap.
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

### The effect categories

Each category maps to one function of the executable spec
([`../validation-spec/`](../validation-spec/)) and to
named columns of the recorded tables:

| effect category | what it is | value | spec field | recorded in |
|---|---|---|---|---|
| **inline** | the warning boxes rendered under the contest (each carries a `data-warn-id`) | one set of message keys **per observation point** — `votingUntouched` (constantly empty: the untouched-clear), `voting`, `review`; 1a when non-empty | `inlineViews` | *inline (voting)* / *inline (review)* columns, `dom-validate.md`; the untouched constant asserted empty per cell by `dom-validate.mjs` |
| **dialog** | the dialog that may open on clicking Next / entering review | none / dismissible (1b) / blocking (1c) — a projection of the gate pair (see the intermediates note below): hard → blocking, else soft → dismissible, else none | `f().dialog` | the observed dialog per cell in `dom-validate.recorded.json` |
| **reachability** | whether the booth UI will form the cell's vote state at all — a greyed control (1d), a marker that clears a co-selection. Effects of unformable cells are still real: decoded or hand-built records run the checkers (the §2 pruning caution), which is why the mapping stays total | yes / inputs_disabled / marker_cleared | `reachability` | *reachable* column, `dom-validate.md` |
| **tally** | the cast ballot's class in the results (it reaches the world through aggregates) | exactly one of the six classes (2a–2f) | `classify` | *tally* column of every rule table; `classifier-table.md` |

"Silent" (1e) is not a category of its own: it is the empty value on
**inline** and **dialog** together — nothing rendered at either casting
point, no dialog. That derived condition, on a reachable cell whose
tally is `ImplicitInvalid`, is exactly what the silent-discount
property (§4.5) tests.

**Checkable intermediates — the validation apparatus, not the spec.**
The executable spec's output (the `Effects` struct in
[`../validation-spec/`](../validation-spec/))
carries two more components that are *not* effects — nothing in the
system exposes them directly; they are intermediate data the effects
are computed from, and they ride in the output so the validation suite
can pin the internal stages against the real WASM per cell rather than
only the end effects:

- `emissions` — the checker record (`invalid_errors` / `invalid_alerts`
  as checker.rs produces them): consumed by inline (a filtered view of
  it), the gates (its errors), and the tally (whether any error
  exists); checked per cell as the *errors* / *alerts* columns of
  every rule table.
- `gate` — the pair (hard, soft) of review-transition gate functions.
  Production evaluates two independent functions and both can fire at
  once (the `not-allowed` rows of
  `../characterization/invalid-rule.md`); the observable effect is the
  dialog they project to — both-fire and hard-alone are
  indistinguishable in the booth. The pair is checked per cell as the
  *hard gate* / *soft gate* columns of every rule table.

So: six output fields = four effect categories + two checkable
intermediates.

**Key principle:** Effects are *atomic observables*. Timing and location
are not part of the effect taxonomy — they index the inline category's
value. For example, `WARN_ONLY_IN_REVIEW` is not a distinct effect — it
is the same effect (1a), present at one observation point and absent at
another:

```
inline(undervote, WARN_ONLY_IN_REVIEW) = {voting: —, review: underVote}
```

**Refinement — categories are independent, not exclusive.** A single
cell can legitimately produce an inline message (1a) *and* a dialog
(1b/1c) — e.g. an overvote under `NOT_ALLOWED_WITH_MSG_AND_ALERT` shows
inline text during voting and a blocking dialog on transition. "Exactly
one value" holds per category, and the casting-time part of the mapping
is therefore a product:

```
CastingEffect = (inline: observation_point → Set<Message>,
                 dialog: none | dismissible | blocking)
```

Two amendments to an earlier version of this type, both forced by what
the characterization verified (`../validation-spec/` is the
executable form):

- **The dialog is a projection of two independent booleans, not a
  primitive three-valued outcome.** Production evaluates two separate
  functions (`check_voting_not_allowed_next_util` and
  `check_voting_error_dialog_util`), and both can be true at once
  (recorded: the `not-allowed` rows of
  `../characterization/invalid-rule.md` trip both). The effect the cell
  is associated with is the projection — `hard ? blocking : soft ?
  dismissible : none` — which is what `dom-validate.mjs` checks against
  the real DOM; the pair itself rides along as a checkable intermediate
  (the note above).
- **The input constraint (1d) is not part of the casting product.**
  Prevention prunes which states the booth UI can *produce*; it does not
  change the mapping over states that exist anyway — hand-built or
  decoded records still run through the checkers as defense-in-depth
  (the §2 pruning caution). It is the reachability category — its own
  total function over the same inputs:

  ```
  reachability(config, vote_state) → yes | inputs_disabled | marker_cleared
  ```

  with two prevention mechanisms: the DISABLE over-vote policy disabling
  further inputs at max (`inputs_disabled` — 1d names what the voter
  sees of it), and the blank marker clearing co-selected regulars
  (`marker_cleared` — observed as `no (cleared)` in
  `../characterization/dom-validate.md`; the invalid marker deliberately
  does not clear — finding S5, imperceptible until tried).

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

### Observation context (indexes the inline value only)

These dimensions parameterize nothing but the inline effect category:
the dialog appears at one fixed moment (the Next/review transition), and
the tally is observed after casting. (`on_transition` is not an
observation point: nothing inline is read there — it is the moment the
gates are consulted.)

| Dimension | Domain |
|-----------|--------|
| `observation_point` | {during_voting, on_review} — the two screens inline warnings render on |
| `is_touched` | {true, false} — during_voting only; an untouched contest renders nothing (the untouched-clear) |

### The seven rules — the decomposition, made explicit

The checker side of the mapping decomposes into seven per-condition
validation **rules**. Concretely, one rule is one checker function in
sequent-core's `ballot_codec/checker.rs` plus everything downstream keyed
to its message and policy — its gate clauses, its keep-list carve-out,
its alert-visibility rule — and the characterization runs one runner and
one grid per rule. The table lists them in production's decode call
order (`raw_ballot.rs`); each row states everything the rule reads, so
the decomposition claim ("each rule reads at most three dimensions") is
checkable row by row. The *rule-specific downstream* column collects the
rule's fingerprints inside the shared machinery — every clause of the
gates, the filter, and reachability keyed to this rule's policy or
message (mostly by *re-deriving* the rule's condition independently, the
drift-prone pattern S4 exemplifies; the generic any-error conditions are
excluded — they belong to the shared machinery, below). Do not confuse
this column with what the runners *observe*: every rule's cells are
recorded on every effect category — the uniform columns of all seven
tables — and a rule with no gate clause of its own still shows gate
entries in its table, produced by the generic machinery (min-vote's
gates vary only with `invalid_vote_policy`, never with its own knob).
This column names which effects the rule produces through clauses of its
own; an empty cell is itself information: nothing rescues that rule's
signal once the generic mute applies. Throughout, `n` is the marker-inclusive selection
count (`selections_with_markers` = regulars + blank marker + invalid
flag).

| rule | checker (`checker.rs`) | own knob | reads (vote state) | emits | rule-specific downstream | grid |
|---|---|---|---|---|---|---|
| **invalid** | `check_invalid_vote_policy` | `invalid_vote_policy` | the `is_explicit_invalid` flag | error `explicitNotAllowed` (not-allowed); alert `explicitAlert` (warn-invalid-implicit-and-explicit) | soft gate fires on the flag under warn-invalid-implicit-and-explicit; the error's Explicit type trips the hard gate's fast path. **The same knob also acts globally** — the master filter's mute and the generic gate conditions read it for *every* rule (the cross-rule interaction below) | `invalid-rule.mjs`: 4 policies × 5 states = 20 |
| **over-vote** | `check_over_vote_policy` | `over_vote_policy` | `n` vs `max_votes` | error `selectedMax` (n > max, unconditional); alert `selectedMax` (n > max ∧ over ≠ allowed); alert `overVoteDisabled` (n = max under DISABLE) | hard gate (n > max ∧ NOT_ALLOWED_WITH_MSG_AND_ALERT); soft gate (n > max ∧ ALLOWED_WITH_MSG_AND_ALERT); keep-list carve-out (`selectedMax` survives the mute iff over ≠ allowed); `overVoteDisabled` hidden at review; DISABLE → reachability `inputs_disabled` | `overvote-rule.mjs`: 5 policies × 4 invalid × 3 states = 60 |
| **min-vote** | `check_min_vote_policy` | the `min_votes` bound (no policy) | `n` vs `min_votes` | error `selectedMin` (n < min, unconditional) | **nothing of its own** — no gate clause, no carve-out, no alert. With `invalid = allowed` muting the generic signals, this row's emptiness *is* the S1 min-vote family | `minvote-rule.mjs`: min ∈ {1, 2} × 4 invalid × 3 states = 24 |
| **under-vote** | `check_under_vote_policy` | `under_vote_policy` | `n` vs `min_votes` / `max_votes` | alert `underVote` (min ≤ n < max ∧ under ≠ allowed; the zone includes n = 0 — S4's checker half) | soft gate re-derives the zone with an extra n > 0 guard (S4's gate half); WARN_ONLY_IN_REVIEW hides the alert during voting | `undervote-rule.mjs`: 4 policies × 4 invalid × 3 states = 48 |
| **blank** | `check_blank_vote_policy` | `blank_vote_policy` | `n` = 0; skipped when the invalid flag is set (a cross-rule read) | error `blankVote` (not-allowed) *or* alert `blankVote` (warn / warn-only-in-review), at n = 0 ∧ ¬flag | hard gate (n = 0 ∧ not-allowed) and soft gate (n = 0 ∧ warn) re-derive emptiness; keep-list carve-out (`blankVote` survives iff not-allowed); WARN_ONLY_IN_REVIEW hides during voting | `blank-rule.mjs`: 4 policies × 4 invalid × 4 states = 64 |
| **preference-gaps** | `check_preference_gaps_policy` | `preference_gaps_policy` | the ranking skips a rank (preferential only) | error `preferenceOrderWithGaps` (unconditional on a gap) | the policy decides only *which* gate reacts (dismissible vs blocking) — both variants gate, so the rule cannot be configured silent (§4.5, condition 2) | `prefgaps-rule.mjs`: 2 policies × 4 invalid × 2 states = 16 |
| **duplicated-rank** | `check_duplicated_rank_policy` | `duplicated_rank_policy` | two candidates share a rank (preferential only) | error `duplicatedPosition` (unconditional on a duplicate) | same shape as preference-gaps: both variants gate | `duprank-rule.mjs`: 2 policies × 4 invalid × 2 states = 16 |

The seven grids sum to the 248 recorded cells. Every grid crosses the
rule's own knob with `invalid_vote_policy` because of that knob's global
role (the mute and the generic gate conditions) — that is a property of
the shared machinery, not of the rules, which is why it is enumerated in
every grid rather than counted as a per-rule dimension.

What is deliberately **not** a rule: the master filter and the gates'
generic conditions (shared machinery consuming every rule's output); the
tally classifier (its own 32-cell decision table,
`../characterization/classifier-table.md`); the booth reducers' marker
exclusivity (reachability's `marker_cleared` — reducer behaviour, no
checker); and the eighth decode-time checker,
`check_max_min_votes_policy`, which validates the *configuration's*
bounds rather than a vote (its emissions are encoding errors — a named
scope boundary, `../characterization/README.md`).

### Combinatorial size

The naive worst-case product is large and not the useful number — the
config side alone is 4·5·4·4·2·2·2·2·2·2 = 20,480 combinations before the
vote-state and context factors.

The useful observation is that the mapping **decomposes** into the seven
rules above: per the "reads" columns of that table, each rule reads at
most three dimensions (its own knob, one or two vote-state comparisons,
and — for its inline visibility only — the observation point), so the
table factors into the per-rule grids listed there (a few dozen cells
each) plus the classifier's six-class decision table over (decline,
invalid, blank-marker, emptiness). Exhaustive enumeration is tractable
*per rule*; the full cross-product never needs to be materialised.
Cross-rule interactions that do exist (the blank checker's dependence on
`is_explicit_invalid`, the master filter's dependence on three policies
at once, the mix rule) are exactly the cells worth enumerating jointly,
and there are few of them.

The effect-first counterpart of this decomposition — for every effect
*component*, which inputs it depends on, which it provably never reads,
and under what conditions each dependence is live (the
conditional-independence view) — is generated, not asserted:
[`../characterization/effect-dependencies.md`](../characterization/effect-dependencies.md)
computes it exhaustively on the executable spec over the full modelled
domain and re-runs each dependence's witness cells through the real WASM
where the fixtures can represent them, labelling the rest. The rule
decomposition above is the *encoding-side* view (production's units, the
transcription-fidelity units); the dependency analysis is the
*behaviour-side* view — coverage questions belong to the latter.

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

Input dimensions declared in this section but exercised by no runner
(`has_encoding_error`; configuration validity) are tracked as named
scope decisions — each with a re-entry condition — in
[`../characterization/README.md`](../characterization/README.md),
"Scope boundaries".

---

## 3. The Mapping (specification)

The system's behaviour is fully characterised by a pure function
returning one value per effect category — the executable form is
[`../validation-spec/`](../validation-spec/), `f`:

```
f(config, vote_state) → ( emissions,      // checkable intermediate (checker record)
                          inline: observation_point → Set<Message>,  // effect
                          gate: (hard, soft),   // checkable intermediate (dialog's inputs)
                          dialog,                                    // effect
                          reachability,                              // effect
                          tally: BallotClass )                       // effect
```

Six components in the struct's field order — the four effect categories
plus the two checkable intermediates (§1, "The effect categories",
states each role). The observation point appears inside the inline
component of the output, not as an input to the mapping. Today this function is *implicitly* encoded across
multiple production code paths:

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
   signals: the generic dialog gate and generic error-visibility (the
   master filter hides every `invalid_error` not on its two-entry
   keep-list — `selectedMax` iff `over_vote_policy ≠ allowed`,
   `blankVote` iff `blank_vote_policy = not-allowed`).

When all three hold, an error the checker produces internally reaches the
tally (via `is_invalid()`) with nothing inline, no dialog, and no block.
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

### 5.2 The declarative artifact (realized)

What §5.1 asks for now exists, twice, checked against itself and against
production (§5.3 status):

- **Casting half** — [`../validation-spec/`](../validation-spec/): the
  typed pure `f(config, vote_state) → Effects` exactly as §3 types it
  Bug-compatible; the
  accidental complexity is enumerated as the quirk registry, each entry
  an adjudication decision in waiting.
- **Tally half** — production already ships it: `classify_ballot`
  (velvet-core `extended_metrics.rs`) is a pure function over
  (decline, invalid, blank-marker, emptiness), unit-tested upstream —
  the demonstration of the end state the casting half should reach.

Step 5's end state is unchanged: the booth UI as a thin renderer that
consults the specification instead of re-deriving logic
(`switch (f(config, state).dialog) …`), each effect computed from its
proven support (the signatures in `../characterization/effect-map.md`).

### 5.3 Migration path

This is not a rewrite proposal. The path is incremental:

> **Status (2026-08-18).** Step 1 is complete, steps 3–4 are delivered,
> the spec's claim ledger is validated, and the apparatus has been
> restructured so the evidence attaches to the shipped artifact:
>
> - **Artifacts.** ONE executable spec — the typed Rust crate
>   [`../validation-spec/`](../validation-spec/), bug-compatible, its
>   quirk registry the adjudication work list. The JS transcription that
>   used to carry the evidence is gone; every runner that compares
>   against production now targets the crate directly, so the chain is
>   one link, not two (`EVIDENCE_RESTRUCTURE.md`). Per-rule tables
>   survive as documentation rendered from the spec.
> - **Validation (the evidence layer).** Production ≡ the spec
>   **exhaustively** on the representable headless domain — 276,480
>   cells, plurality and preferential, zero disagreements
>   (`headless-sweep.md`); per cell in the real booth (229/229,
>   `dom-validate.md`); by sufficiency for the browser-side independence
>   claims (2,208 quotient classes covering 156,416 cells,
>   `quotient-validate.md`, under the source-verified props-boundary
>   license and its re-entry condition); and by witness for the
>   browser-side dependence claims (`browser-witnesses.md`).
> - **Analysis (over the certified spec, never touching production).**
>   The dependency ledger and its effect map; the no-silent-discount
>   property. Witness production-backing is now *inherited* from the
>   sweep by a checked containment argument rather than re-observed.
> - **Findings, now derived rather than noticed.** Each is a property the
>   analysis layer evaluates over the whole certified domain, with an
>   acceptance test so the derivation cannot lose it.
>   *no-silent-discount*: 3,168 cells in exactly the two known families
>   (`selectedMin`, `selectedMax`) — no new families — and all 80
>   permitting configurations require `invalid_vote_policy = allowed`,
>   now exhaustively rather than by argument. Five representatives are
>   confirmed booth → encrypt → cast → decrypt → decode → tally
>   (`reproduce-verify.mjs`). *gate/checker count agreement*: 6,200 cells
>   where the dialog the voter meets differs from the one the ballot
>   warrants, in five shapes — filed as S6. Recipes in `REPRODUCE.md`,
>   escalation in `UPSTREAM_FINDINGS.md`, policy intent in
>   `INVALID_VOTE_POLICY_INTENT.md`.
> - **New this round.** Widening the sweep to ranked ballots surfaced a
>   production defect the plurality-only domain could not see: the
>   submission gates count only first preferences where the checker counts
>   every ranked selection (quirk
>   `S6_GATES_COUNT_FIRST_PREFERENCES_ONLY`). The spec is bug-compatible
>   with it, the behaviour is derived as a property, and it is filed as
>   S6. Extending the domain again to WELL-FORMED rankings — the ordinary
>   ranked ballot — is what let the property derive its two sharpest
>   consequences instead of leaving them to ad-hoc probes.
> - **Open.** The decline booth flow (a `multi_ballot` feature lift); a
>   generic IRV booth recipe — the headless half is closed, but the booth
>   half leaves 4,288 quotient classes' inline behaviour spec-only; a
>   both-markers fixture; step 5 — a production interpreter — remains
>   adjudication-gated, and is the next milestone's main objective.
>
> On the representable subdomain a transcription hole of either
> polarity — a wrong clause or a missing one — cannot hide: coverage,
> not argument. Outside it, the labels and scope boundaries say exactly
> what is not covered and what would unlock it.

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
