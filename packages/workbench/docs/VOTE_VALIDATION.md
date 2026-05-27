<!--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Vote Validation Architecture

How the system validates voter selections — from the moment a checkbox is
clicked in the booth UI through to tally-time decoding.

---

## Overview

Vote validation uses a **single source of truth**: the Rust checker functions
in [`sequent-core/src/ballot_codec/checker.rs`](../../sequent-core/src/ballot_codec/checker.rs).
These run both in the voting booth (via WASM, for real-time feedback) and
during tally (via native Rust, when decoding decrypted ballots). This
guarantees that the booth and tally always agree on what constitutes a valid
vote.

The booth does **not** re-implement validation in TypeScript. Instead, it
performs an encode→decode round-trip through the same WASM codec that will
later process the ballot during tally. The decode step invokes the checkers
and returns structured error/alert lists that the UI renders.

---

## Architecture: Encode→Decode Round-Trip

When the voter clicks a candidate, the booth immediately:

1. **Encodes** the current selection into a bigint (the same compact
   representation that would be encrypted and cast).
2. **Decodes** that bigint back into a `DecodedVoteContest` struct. During
   decode, all checker functions fire and populate `invalid_errors` and
   `invalid_alerts` arrays on the result.

This serves three purposes:
- Validates the selection is **encodable** (doesn't exceed capacity).
- Produces the **exact same errors** that tally would produce.
- Verifies **round-trip fidelity** (input == output after encode→decode).

---

## Call Chain

```
┌─────────────────────────────────────────────────────────────────┐
│  Voting Portal (React)                                          │
│                                                                 │
│  VotingScreen.tsx                                               │
│    └─ useMemo(errorSelectionState) — recalculates on every      │
│       selection change (real-time, per click)                    │
│    └─ disableNextButton() — checked on page/review transition   │
│    └─ showNextDialog() — checked on page/review transition      │
│                                                                 │
│  Question.tsx → InvalidErrorsList.tsx                            │
│    └─ renders invalid_errors as warnings/errors per contest     │
└────────────────────────┬────────────────────────────────────────┘
                         │ calls
┌────────────────────────▼────────────────────────────────────────┐
│  ui-core/src/services/wasm.ts  (TypeScript WASM wrappers)       │
│                                                                 │
│  interpretContestSelection(selection, ballotStyle)               │
│  interpretMultiContestSelection(selection, ballotStyle)          │
│  check_voting_not_allowed_next_bool(contests, decodedContests)  │
│  check_voting_error_dialog_bool(contests, decodedContests)      │
└────────────────────────┬────────────────────────────────────────┘
                         │ wasm_bindgen
┌────────────────────────▼────────────────────────────────────────┐
│  sequent-core/src/wasm/wasm.rs  (WASM entry points)             │
│                                                                 │
│  test_contest_reencoding_js(decoded_contest, ballot_style)      │
│    → contest.encode_plaintext_contest_bigint(&selection)        │
│    → contest.decode_plaintext_contest_bigint(&bigint)  ← HERE   │
│                                                                 │
│  check_voting_not_allowed_next(contests, decoded_contests)      │
│    → voting_screen.rs::check_voting_not_allowed_next_util       │
│                                                                 │
│  check_voting_error_dialog(contests, decoded_contests)          │
│    → voting_screen.rs::check_voting_error_dialog_util           │
└────────────────────────┬────────────────────────────────────────┘
                         │ calls during decode
┌────────────────────────▼────────────────────────────────────────┐
│  sequent-core/src/ballot_codec/raw_ballot.rs                    │
│  (or multi_ballot.rs for multi-contest plurality ballots)       │
│                                                                 │
│  decode_plaintext_contest_bigint() calls sequentially:          │
│    1. check_invalid_vote_policy()                               │
│    2. check_max_min_votes_policy()                              │
│    3. check_over_vote_policy()                                  │
│    4. check_min_vote_policy()                                   │
│    5. check_under_vote_policy()                                 │
│    6. check_blank_vote_policy()                                 │
│    7. check_preference_gaps_policy()    (preferential only)     │
│    8. check_duplicated_rank_policy()    (preferential only)     │
│                                                                 │
│  Results populate: DecodedVoteContest.invalid_errors[]           │
│                    DecodedVoteContest.invalid_alerts[]           │
└─────────────────────────────────────────────────────────────────┘
```

---

## When Validation Runs

### Real-time (per selection change)

`errorSelectionState` is a `useMemo` in `ContestPagination` that
recalculates every time `ballotSelectionState` changes. This means:
- Every click on a candidate triggers an encode→decode round-trip.
- Error/alert messages appear inline beneath the contest immediately.
- The validation runs against **all contests in the ballot selection state**
  (not just the visible page), ensuring cross-contest constraints are caught.

### On page transition (next page or → review)

Two gate functions are evaluated when the voter clicks "Next" or "Review":

| Function | Effect |
|----------|--------|
| `check_voting_not_allowed_next_bool` | **Hard block** — disables the button; voter must fix issues |
| `check_voting_error_dialog_bool` | **Soft warn** — shows a confirmation dialog before proceeding |

These read the already-computed `decodedContests` (populated by each
`<Question>` component) and consult the policies to determine severity.

---

## Dual Decode Paths

The system has two ballot encoding formats:

| Path | File | Used when | Checkers invoked |
|------|------|-----------|-----------------|
| Single-contest | `raw_ballot.rs` | IRV/Borda/preferential contests, or single-contest ballots | All 8 |
| Multi-contest | `multi_ballot.rs` | Multiple **plurality** contests on one ballot (30-byte payload) | First 6 only |

`multi_ballot::decode` rejects non-Plurality contests up-front — IRV/Borda
ballots always travel the `raw_ballot::decode` path. This is why
`check_duplicated_rank_policy` and `check_preference_gaps_policy` only exist
in the raw_ballot path.

---

## Numeric Parameters

Three contest-level integers drive the validation thresholds:

### min_votes

- **Type**: `Contest.min_votes: i64`
- **Semantics**: Minimum number of selections the voter must make for the
  ballot to be valid. If `num_selected < min_votes`, the checker produces an
  error (hard block, always — regardless of policy).
- **Checker**: [`check_min_vote_policy`](../../sequent-core/src/ballot_codec/checker.rs#L80)
- **Interaction**: Interacts with `BlankVotePolicy` (blank = 0 selections)
  and `UnderVotePolicy` (under = between min_votes and max_votes).

### max_votes

- **Type**: `Contest.max_votes: i64`
- **Semantics**: Maximum number of selections allowed. Behavior when
  exceeded depends on `OverVotePolicy`.
- **Checker**: [`check_over_vote_policy`](../../sequent-core/src/ballot_codec/checker.rs#L137)
- **Encoding impact**: For preferential contests, the encoding base =
  `max_votes + 1` ([`bases.rs`](../../sequent-core/src/ballot_codec/bases.rs#L23)).
  For plurality, base = 2 (selected/not-selected per candidate).
- **UI impact**: When `OverVotePolicy = NOT_ALLOWED_WITH_MSG_AND_DISABLE`,
  checkboxes are disabled once `max_votes` selections are made
  ([`Question.tsx`](../../voting-portal/src/components/Question/Question.tsx)).

### winning_candidates_num

- **Type**: `Contest.winning_candidates_num: i64`
- **Semantics**: Number of candidates who win (take seats). Used by the
  **tally** algorithm (e.g. top-N plurality, IRV elimination threshold) but
  **not** by the booth-side validation checkers. Does not affect whether a
  voter's selection is valid.

### Sanity check

[`check_max_min_votes_policy`](../../sequent-core/src/ballot_codec/checker.rs#L37)
validates that `max_votes` and `min_votes` are convertible to `usize`
(non-negative, within bounds). This is a configuration-level check, not a
voter-action check — it fires first and short-circuits if the contest config
itself is malformed.

---

## Vote Validation Policies (6 policies)

All six policies live in `ContestPresentation` (the presentation sub-object
of a `Contest`). Each has a fixed set of enum variants that control:
- Whether the condition produces an **error** (hard block) or **alert** (soft warn).
- Whether the condition is surfaced **during voting**, **only in review**, or **not at all**.

### Policy severity model

The submission-gate in [`voting_screen.rs`](../../sequent-core/src/util/voting_screen.rs)
classifies policy variants into two tiers:

- **Hard blockers** (`NOT_ALLOWED*` variants): `check_voting_not_allowed_next_util`
  returns true → button disabled, voter cannot proceed.
- **Soft warnings** (`WARN*`, `ALLOWED*` variants): `check_voting_error_dialog_util`
  returns true → confirmation dialog shown, voter may proceed anyway.

---

### 1. InvalidVotePolicy

**What it checks**: Whether the voter selected an "explicitly invalid"
candidate (a special candidate marked as invalid in the election config).

**Enum**: `InvalidVotePolicy` — 4 variants

| Variant | Behavior |
|---------|----------|
| `ALLOWED` (default) | No error, no alert. |
| `WARN` | Alert if explicit invalid selected. |
| `WARN_INVALID_IMPLICIT_AND_EXPLICIT` | Alert on both implicit and explicit invalidity. |
| `NOT_ALLOWED` | Error — blocks submission. |

**Checker**: [`check_invalid_vote_policy`](../../sequent-core/src/ballot_codec/checker.rs#L281)

**UI surface**: [`InvalidErrorsList.tsx`](../../voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx)
reads the policy and displays/hides the corresponding messages.

---

### 2. OverVotePolicy

**What it checks**: Whether the voter selected **more** candidates than
`max_votes` allows.

**Enum**: `EOverVotePolicy` — 5 variants

| Variant | Behavior |
|---------|----------|
| `ALLOWED` | Silently allowed (extra votes encoded but ignored at tally). |
| `ALLOWED_WITH_MSG` | Allow, show message. |
| `ALLOWED_WITH_MSG_AND_ALERT` (default) | Allow, show message + alert popup. |
| `NOT_ALLOWED_WITH_MSG_AND_ALERT` | Error — blocks submission + alert. |
| `NOT_ALLOWED_WITH_MSG_AND_DISABLE` | Error — blocks submission + **disables checkboxes** when max reached. |

**Checker**: [`check_over_vote_policy`](../../sequent-core/src/ballot_codec/checker.rs#L137)

**Special UI behavior**: The `NOT_ALLOWED_WITH_MSG_AND_DISABLE` variant has
a unique UX — [`Question.tsx`](../../voting-portal/src/components/Question/Question.tsx)
disables unchecked checkboxes once `num_selected == max_votes`, preventing
the overvote entirely at the UI level (the checker still validates as
defense-in-depth).

---

### 3. UnderVotePolicy

**What it checks**: Whether the voter selected **fewer** candidates than
`max_votes` (but still ≥ `min_votes`). This is distinct from BlankVotePolicy
(which handles 0 selections).

**Enum**: `EUnderVotePolicy` — 4 variants

| Variant | Behavior |
|---------|----------|
| `ALLOWED` (default) | No warning. |
| `WARN` | Alert shown during voting. |
| `WARN_ONLY_IN_REVIEW` | Alert shown only on the review screen, not during voting. |
| `WARN_AND_ALERT` | Alert + popup dialog. |

**Checker**: [`check_under_vote_policy`](../../sequent-core/src/ballot_codec/checker.rs#L197)

**Note**: Under-vote is defined as `min_votes ≤ num_selected < max_votes`.
If `num_selected < min_votes`, that's a **min_vote violation** (always an
error, regardless of UnderVotePolicy).

---

### 4. BlankVotePolicy

**What it checks**: Whether the voter selected **zero** candidates (blank
ballot).

**Enum**: `EBlankVotePolicy` — 4 variants

| Variant | Behavior |
|---------|----------|
| `ALLOWED` (default) | Blank ballots accepted without comment. |
| `WARN` | Alert shown. |
| `WARN_ONLY_IN_REVIEW` | Alert shown only on review screen. |
| `NOT_ALLOWED` | Error — blocks submission (equivalent to enforcing min_votes ≥ 1). |

**Checker**: [`check_blank_vote_policy`](../../sequent-core/src/ballot_codec/checker.rs#L103)

---

### 5. DuplicatedRankPolicy (preferential only)

**What it checks**: Whether the voter assigned the **same rank** to multiple
candidates in a preferential (IRV / Borda) contest.

**Enum**: `EDuplicatedRankPolicy` — 2 variants

| Variant | Behavior |
|---------|----------|
| `ALLOWED_WARN_AND_DIALOG` (default) | Allow duplicates, warn with dialog. |
| `NOT_ALLOWED_WARN_AND_DIALOG` | Reject duplicates, warn with dialog (hard block). |

**Checker**: [`check_duplicated_rank_policy`](../../sequent-core/src/ballot_codec/checker.rs#L235)

**Only applies to**: Contests with `counting_algorithm` = `InstantRunoff`,
`Borda`, `BordaNauru`, `BordaMasMadrid`, `Desborda*`, or `PairwiseBeta`.
Never fires for `PluralityAtLarge` or `Cumulative`.

---

### 6. PreferenceGapsPolicy (preferential only)

**What it checks**: Whether the voter left **gaps** in their ranking (e.g.
rank 1, rank 3, skipping rank 2).

**Enum**: `EPreferenceGapsPolicy` — 2 variants

| Variant | Behavior |
|---------|----------|
| `ALLOWED_WARN_AND_DIALOG` (default) | Allow gaps, warn with dialog. |
| `NOT_ALLOWED_WARN_AND_DIALOG` | Reject gaps, warn with dialog (hard block). |

**Checker**: [`check_preference_gaps_policy`](../../sequent-core/src/ballot_codec/checker.rs#L258)

**Only applies to**: Same preferential contest types as DuplicatedRankPolicy.

---

## Error Display Pipeline

The flow from checker output to rendered UI:

1. **Checker** populates `DecodedVoteContest.invalid_errors[]` and
   `DecodedVoteContest.invalid_alerts[]` during decode.

2. **wasm.ts** returns the full `BallotSelection` (array of
   `DecodedVoteContest`) as `errorSelectionState` to the React layer.

3. **`<Question>`** receives `errorSelectionState` and passes the relevant
   contest's errors to **`<InvalidErrorsList>`**.

4. **`<InvalidErrorsList>`** applies UI-level filtering:
   - Hides under-vote warnings if not in review mode (for `WARN_ONLY_IN_REVIEW`).
   - Suppresses messages when policy is `ALLOWED`.
   - Renders remaining `invalid_errors` as `<WarnBox variant="warning">`.
   - Renders remaining `invalid_alerts` as `<WarnBox variant="info">`.

---

## Submission Gating Detail

[`voting_screen.rs`](../../sequent-core/src/util/voting_screen.rs) exposes
two utility functions consumed via WASM:

### check_voting_not_allowed_next_util

Iterates all contests. For each, checks if any `invalid_errors` entry
corresponds to a `NOT_ALLOWED*` policy variant. If any contest has such an
error → returns `true` → **button disabled**.

### check_voting_error_dialog_util

Iterates all contests. For each, checks if any `invalid_errors` or
`invalid_alerts` exist (regardless of severity). If any contest has
warnings → returns `true` → **dialog shown** (voter can dismiss and
proceed).

### Interaction with pagination

The voting screen paginates contests (`contests: IContest[][]` — array of
pages). The gate functions check **all contests across all pages** (via the
`decodedContests` record), not just the currently visible page. This means a
hard-block error on page 1 will prevent navigation to review even if the
voter is currently on page 3.

---

## Key Source Files

| File | Role |
|------|------|
| [`sequent-core/src/ballot_codec/checker.rs`](../../sequent-core/src/ballot_codec/checker.rs) | All 8 checker functions (single source of truth) |
| [`sequent-core/src/ballot_codec/raw_ballot.rs`](../../sequent-core/src/ballot_codec/raw_ballot.rs) | Single-contest decode; invokes all 8 checkers |
| [`sequent-core/src/ballot_codec/multi_ballot.rs`](../../sequent-core/src/ballot_codec/multi_ballot.rs) | Multi-contest decode; invokes first 6 checkers (plurality only) |
| [`sequent-core/src/util/voting_screen.rs`](../../sequent-core/src/util/voting_screen.rs) | Submission gating logic (hard block vs soft warn) |
| [`sequent-core/src/wasm/wasm.rs`](../../sequent-core/src/wasm/wasm.rs) | WASM entry points (`test_contest_reencoding_js`, gate functions) |
| [`ui-core/src/services/wasm.ts`](../../ui-core/src/services/wasm.ts) | TypeScript WASM wrappers |
| [`voting-portal/src/routes/VotingScreen.tsx`](../../voting-portal/src/routes/VotingScreen.tsx) | Orchestrates validation calls; manages gate state |
| [`voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx`](../../voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx) | Renders errors/alerts; applies UI-level policy filtering |
| [`voting-portal/src/components/Question/Question.tsx`](../../voting-portal/src/components/Question/Question.tsx) | Per-contest rendering; checkbox disable for `NOT_ALLOWED_WITH_MSG_AND_DISABLE` |

---

## Relationship Between Policies and Parameters

```
                    ┌──────────────────────────────────────────────┐
                    │           num_selected (voter's choices)      │
                    └──────────────┬───────────────────────────────┘
                                   │
          ┌────────────────────────┼────────────────────────────┐
          │                        │                            │
   num_selected == 0        0 < num_selected < min     min ≤ num_selected < max
          │                        │                            │
   BlankVotePolicy          min_vote error             UnderVotePolicy
   (configurable)           (always hard block)        (configurable)
                                                                │
                                                    num_selected == max
                                                         │
                                                    (valid — no policy fires)
                                                                │
                                                    num_selected > max
                                                         │
                                                    OverVotePolicy
                                                    (configurable)
```

For preferential contests, two additional checks apply regardless of count:
- **Duplicate ranks** → DuplicatedRankPolicy
- **Gaps in ranking** → PreferenceGapsPolicy

And across all contest types:
- **Invalid candidate selected** → InvalidVotePolicy
