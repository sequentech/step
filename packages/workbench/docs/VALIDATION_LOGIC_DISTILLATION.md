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

Every (voter-state × contest-configuration × observation-context) tuple
produces exactly one **observable effect** drawn from the following closed set.

### Casting-time effects (booth UI)

| # | Effect | Description |
|---|--------|-------------|
| 1a | Inline message displayed | Voter sees text below the contest (warning or error) |
| 1b | Dismissible dialog shown | Modal informs voter; voter may dismiss and continue |
| 1c | Non-dismissible dialog shown | Modal blocks; voter must fix the selection to proceed |
| 1d | Input constraint applied | UI prevents reaching the state (e.g. checkboxes disabled) |
| 1e | Silent | No signal — voter is unaware of any policy concern |

### Tally-time effects

| # | Effect | Description |
|---|--------|-------------|
| 2a | Vote counted normally | Contributes to candidate tallies |
| 2b | Vote classified explicit-invalid | Counted in `explicit_invalid_votes` |
| 2c | Vote classified implicit-invalid | Counted in `implicit_invalid_votes` |

**Key principle:** Effects are *atomic observables*. Timing and location are
part of the *input space*, not the effect taxonomy. For example,
`WARN_ONLY_IN_REVIEW` is not a distinct effect — it is the same effect (1a)
mapped from a different observation point:

```
(undervote, WARN_ONLY_IN_REVIEW, during_voting)   → 1e (silent)
(undervote, WARN_ONLY_IN_REVIEW, on_review)        → 1a (inline message)
```

---

## 2. Input Space Dimensions

### Contest configuration (static per election)

| Dimension | Domain |
|-----------|--------|
| `min_votes` | ℕ |
| `max_votes` | ℕ (≥ min_votes) |
| `invalid_vote_policy` | {ALLOWED, WARN, WARN_IMPLICIT_AND_EXPLICIT, NOT_ALLOWED} |
| `over_vote_policy` | {ALLOWED, ALLOWED_WITH_MSG, ALLOWED_WITH_MSG_AND_ALERT, NOT_ALLOWED_WITH_MSG_AND_ALERT, NOT_ALLOWED_WITH_MSG_AND_DISABLE} |
| `under_vote_policy` | {ALLOWED, WARN, WARN_ONLY_IN_REVIEW, WARN_AND_ALERT} |
| `blank_vote_policy` | {ALLOWED, WARN, WARN_ONLY_IN_REVIEW, NOT_ALLOWED} |
| `duplicated_rank_policy` | {ALLOWED_WARN_AND_DIALOG, NOT_ALLOWED_WARN_AND_DIALOG} |
| `preference_gaps_policy` | {ALLOWED_WARN_AND_DIALOG, NOT_ALLOWED_WARN_AND_DIALOG} |
| `counting_algorithm` | {Plurality, Preferential} (simplified) |

### Vote state (dynamic, per voter action)

| Dimension | Domain |
|-----------|--------|
| `num_selected_class` | {0, (0..min), [min..max), max, (max..∞)} — 5 equivalence classes |
| `is_explicit_invalid` | {true, false} |
| `has_duplicate_ranks` | {true, false} (preferential only) |
| `has_rank_gaps` | {true, false} (preferential only) |

### Observation context

| Dimension | Domain |
|-----------|--------|
| `observation_point` | {during_voting, on_review, on_transition} |
| `is_touched` | {true, false} |

### Combinatorial size

Worst-case: 4 × 5 × 4 × 4 × 2 × 2 × 2 (policies) × 5 (num_selected) ×
2 (explicit_invalid) × 2 (dup_ranks) × 2 (gaps) × 2 (review) × 2 (touched)
× 2 (action) ≈ **40,960 cells**.

Most are redundant (preferential-only flags don't apply to plurality,
`num_selected_class > max` is unreachable under `NOT_ALLOWED_WITH_MSG_AND_DISABLE`,
etc.). The *effective* space is tractable for exhaustive enumeration.

---

## 3. The Mapping (specification)

The system's behaviour is fully characterised by a pure function:

```
f(config, vote_state, observation_context) → Effect
```

Today this function is *implicitly* encoded across multiple code paths:

- `checker.rs` (8 checker functions producing errors/alerts)
- `voting_screen.rs` (gating utilities computing "can proceed?")
- `InvalidErrorsList.tsx` (filtering errors by `isReview`)
- `Question.tsx` (disabling inputs based on policy)
- `VotingScreen.tsx` (wiring gate results to dialog/button state)
- Tally aggregation (classifying decoded ballots)

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
- `InvalidVotePolicy::ALLOWED` → voter sees NO warnings → at tally, errors
  still present → vote classified implicit-invalid.

This raises a design question: if a voter receives zero indication (1e) that
their vote will be invalid at tally (2c), is that intentional? The answer
depends on whether "ALLOWED" means "allowed to be cast" or "counted as valid."
Current code: it means "allowed to be cast" — the tally classifies
independently.

### 4.3 `InvalidVotePolicy` conflates three concerns

From first principles, these are independent controls:

1. "Allow explicit-invalid candidate?" (boolean)
2. "Show structural error messages to voter?" (visibility enum)
3. "Block submission on structural errors?" (strictness enum)

Packing them into a single enum (ALLOWED / WARN / WARN_IMPLICIT_AND_EXPLICIT /
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

---

## 5. Toward a Declarative Implementation

### 5.1 Goals

1. **Single artifact** that both booth and tally consult (today: checker.rs
   is shared, but UI gating/filtering logic is separate TypeScript).
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
fn tally_effect(config: &ContestConfig, vote: &VoteState) -> TallyEffect {
    if vote.is_explicit_invalid { return TallyEffect::ExplicitInvalid }
    if vote.has_errors(config)  { return TallyEffect::ImplicitInvalid }
    TallyEffect::CountedNormally
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

1. **Enumerate the current mapping** — write a test that exercises every
   reachable cell of the input space through the existing code and records
   the observed effect. This becomes the ground-truth table.
2. **Express the table declaratively** — either as a Rust match expression
   or as a data structure the workbench can load and visualize.
3. **Verify equivalence** — the declarative version must produce identical
   outputs to the recorded table for every cell.
4. **Optionally refactor** — once the declarative version is proven
   equivalent, the scattered imperative code (filter functions, dialog
   utilities, disable-prop wiring) can be replaced by a single interpreter
   that reads the specification. This is optional and can be done
   incrementally per-checker.

---

## 6. Relationship to Other Documents

- [VOTE_VALIDATION.md](./VOTE_VALIDATION.md) — describes the current
  implementation architecture (call chains, data flow, component roles).
  This document describes the *desired specification* that the implementation
  should converge toward.
- [FIXTURE_VARIANCE.md](./FIXTURE_VARIANCE.md) — identifies which fixture
  dimensions exercise which code paths. The exhaustive mapping in §2 above
  subsumes and formalises the variance dimensions relevant to validation.
