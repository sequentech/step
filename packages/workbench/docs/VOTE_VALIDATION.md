<!--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Vote Validation Architecture

How the system validates voter selections — from the moment a checkbox is
clicked in the booth UI through to tally-time decoding.

---

## Overview

The checker functions in
[`sequent-core/src/ballot_codec/checker.rs`](../../sequent-core/src/ballot_codec/checker.rs)
are a **shared component**: they run both in the voting booth (via WASM, for
real-time feedback) and during tally (via native Rust, when decoding decrypted
ballots). Booth and tally are therefore guaranteed to agree on **checker
output** — the `invalid_errors` / `invalid_alerts` a given selection produces.

They are *not* the whole truth about what a ballot ultimately is. Three other
sites carry validation semantics of their own:

- **Tally classification** is `ContestValidator::classify`, which velvet's
  `classify_ballot` calls for every cast ballot. It reads the messages only
  to ask whether any error was recorded, and decides the rest from the
  ballot: the decline-to-vote precedence, the explicit/implicit blank split,
  and the marker-mix rule (see "Tally-Time Classification" below).
- **The marker exclusivity rule** is `ContestValidator::apply`, which the
  booth's `ballotSelections` reducer asks on every edit. It applies in full
  to the blank marker: selecting a regular candidate clears a selected
  explicit-blank marker and vice versa, so that mixed state is prevented
  rather than validated. The **invalid (null) marker is exempt under every
  policy but one** — marking a ballot invalid sets the flag and leaves the
  selections standing, so {regular + null marker} forms and is cast (finding
  S5 in [UPSTREAM_FINDINGS.md](UPSTREAM_FINDINGS.md); both directions
  observed in `characterization/dom-validate.md`). The exception is
  `ALLOWED_WITH_EXCLUSIVE_EXPLICIT`, under which the invalid marker behaves
  like the blank one in both directions.
- **Error visibility** is decided by `ContestValidator::filter_visible_messages`,
  which `InvalidErrorsList.tsx` asks through the `filter_visible_messages_js`
  wasm export. Which of a ballot's messages the voter sees is a validation
  rule like the rest, not a UI-layer decision on top of them.

The booth re-implements no validation in TypeScript. It performs an
encode→decode round-trip through the same WASM codec that will later
process the ballot during tally, the decode step returns the structured
error and alert lists, and the three questions the booth used to answer
itself — which messages to show, whether to accept another selection, and
what a marker clears — it now asks that same module.

---

## Functional Model: Who Does What, and Why

Before any call-stack detail: validation is **one producer, two independent
consumers, plus two preventers and a final classifier**. Every behaviour in
this document is one of these six roles doing its job.

```
 BOOTH — prevention (acts on gestures, BEFORE a state exists)
 ─────────────────────────────────────────────────────────────
    voter gesture ──► INPUT CONSTRAINT (Question.tsx: checkboxes may be
                      disabled — the gesture is impossible)
                  ──► MARKER EXCLUSIVITY (ballotSelections reducer: selecting
                      a candidate clears a BLANK marker, and vice versa —
                      the null marker is exempt; see S5)
                                        │
                                        ▼  selection state
 BOOTH — enforcement (reacts to states that DID form)
 ─────────────────────────────────────────────────────────────
                         ┌──────────────────────────────┐
    every click ───────► │ CHECKERS (checker.rs, WASM)  │  produce the record:
                         │ encode→decode round-trip     │  invalid_errors[] /
                         └──────────────┬───────────────┘  invalid_alerts[]
                                        │ DecodedVoteContest
                    ┌───────────────────┴───────────────────┐
                    ▼                                       ▼
     ┌────────────────────────────┐          ┌────────────────────────────────┐
     │ FILTER (InvalidErrorsList, │          │ GATES (voting_screen.rs, WASM) │
     │ TypeScript)                │          │ on Next / review transition    │
     │ decides what the voter     │          │ decide whether the voter may   │
     │ SEES inline                │          │ PROCEED (hard block vs         │
     └────────────────────────────┘          │ dismissible dialog vs nothing) │
                    │                        └────────────────────────────────┘
                    │ setDecodedContests()                  ▲
                    └───────── the only junction ───────────┘

 TALLY — classification (after casting, in a different process)
 ─────────────────────────────────────────────────────────────
    decrypted ballot ──► decode: the SAME CHECKERS run again ──► CLASSIFIER
                         (identical code, identical output)      (classify_ballot:
                                                                  six BallotClass
                                                                  values)
```

The prevention band never appears in the enforcement dataflow because its
job is to keep states out of it; the tally band runs later, elsewhere, and
re-uses the checkers — the CHECKERS box exists twice in time, once per
band, which is the precise content of the shared-component guarantee.

| Role | Where | Trigger | Question it answers |
|------|-------|---------|---------------------|
| **Checkers** | `checker.rs` via the codec round-trip | every selection change | *does a violation record exist?* |
| **Filter** | `InvalidErrorsList.tsx::filterErrorList` | every render | *does the voter see it?* |
| **Gates** | `voting_screen.rs::check_voting_*_util` | Next / review click | *may the voter proceed?* |
| **Input constraint** | `Question.tsx` (checkbox disable) | every render | *can the state be reached at all?* |
| **Marker exclusivity** | `ballotSelectionsSlice.ts` reducer | every selection change | *can markers and candidates co-exist?* (prevention — **blank marker only**: no, it clears; the null marker co-exists, see S5) |
| **Tally classifier** | `velvet-core::classify_ballot` | tally | *what is this ballot, finally?* (six classes) |

Three properties of this shape explain most of the system's surprises:

1. **The consumers do not consume each other's conclusions.** The gates do
   not look for the checker's `blankVote` entry — they re-derive blankness
   from the choices count. The filter does not ask the gates anything — it
   re-reads three policies itself. One condition therefore has up to three
   independently-computed answers (record exists / voter sees it / voter is
   blocked), and the answers can disagree by design or by bug. This is why
   effects are recorded per category — `(inline, dialog, reachability)` —
   rather than as one value per state.
2. **The two consumer paths meet at exactly one point**: each rendered
   `<InvalidErrorsList>` writes its decoded contest into a record via
   `setDecodedContests`, and the gates read that record. The gates never
   decode anything themselves — so a contest that never rendered has no
   entry for the gates to check.
3. **Prevention removes states; enforcement blocks transitions.** The
   constraint and (blank-marker) exclusivity roles stop selection states
   from *forming in the booth*; the gates let states exist but block
   progression. Prevention is UI-only — a hand-built or decoded record
   can still hold the "prevented" state, which is why the checkers
   validate them anyway; and prevention has holes by design (the null
   marker does not clear — S5) and by bug (`fdc7f92db5`, upstream's
   2026-07 "Decline to Vote with Overvote (Disable) still allows
   selecting an additional candidate" fix, #2892, silently re-opened a
   state the config assumed impossible).

The remainder of this document is the detail: the round-trip mechanism, the
call stack (the *how*), each checker's exact behaviour, the filter rules,
the gate condition tables, and the tally classification.

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

This is the **stack view** — who calls whom, across the React → TypeScript
→ WASM → Rust boundaries — of the functional model above. Read it as
plumbing, not as sequence: the arrows show **call direction, not data
direction**. In particular the filter (top box) runs *after* the checkers
(bottom box) in data order — it operates on what the decode returns — and
the two consumer paths from the functional model appear here overlaid in
one stack: the per-click path descends through `interpretContestSelection`
to the checkers, while the per-transition path descends through
`check_voting_*_bool` to the gates.

```
┌─────────────────────────────────────────────────────────────────┐
│  Voting Portal (React)                                          │
│                                                                 │
│  VotingScreen.tsx → ContestPagination (inner component)         │
│    └─ useMemo(errorSelectionState) — recalculates on every      │
│       selection change (real-time, per click)                    │
│    └─ disableNextButton() — checked on page/review transition   │
│    └─ showNextDialog() — checked on page/review transition      │
│                                                                 │
│  Question.tsx → InvalidErrorsList.tsx                            │
│    └─ filters + renders invalid_errors / invalid_alerts         │
│    └─ calls setDecodedContests() to feed gate functions         │
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

Two gate functions are evaluated when the voter clicks "Next" or navigates
to the review screen:

| Function | Effect |
|----------|--------|
| `check_voting_not_allowed_next_bool` | **Hard block** — opens a dialog with only "OK" (no continue); also **physically disables** the Next button when multi-page |
| `check_voting_error_dialog_bool` | **Soft warn** — opens a dialog with "Continue" + "Cancel" |

The `encryptAndReview()` handler opens the dialog whenever either function
returns `true`. The dialog variant (dismissible vs non-dismissible) depends
on whether `disableNextButton()` is `true`.

These read the already-computed `decodedContests` record (populated by each
`<InvalidErrorsList>` component via `setDecodedContests`) and evaluate
specific policy+condition combinations (detailed in "Submission Gating
Detail" below).

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

**Decline-to-vote is a multi-ballot-only wire feature.** When the election's
`decline_to_vote_policy == ENABLED`, the multi-ballot mixed-radix layout
gains a **ballot-level decline bit** (omitted when disabled, for backwards
compatibility with existing elections — see
`multi_ballot.rs::decline_to_vote_enabled`). The single-contest path never
carries it: `raw_ballot::decode` hardcodes `is_decline_to_vote: false`. Two
consequences: the wire format of a multi-contest ballot depends on an
election-level policy (a booth/tally version-skew hazard if the two sides
disagree about the policy), and declined ballots can only originate from
multi-contest encodings.

---

## Numeric Parameters

Three contest-level integers drive the validation thresholds:

### min_votes

- **Type**: `Contest.min_votes: i64`
- **Semantics**: Minimum number of selections the voter must make for the
  ballot to be valid. If `num_selected < min_votes`, the checker **always
  pushes the `selectedMin` error** — min-vote has no policy of its own —
  but what the voter experiences depends entirely on
  `invalid_vote_policy`: `not-allowed` hard-blocks, the `warn*` variants
  raise a dismissible dialog, and **`allowed` shows nothing at all**
  (`selectedMin` is on no keep-list and min-vote emits no alert) while the
  tally still discards the ballot `ImplicitInvalid` — four of the five
  confirmed silent discounts (S1/S2 in
  [UPSTREAM_FINDINGS.md](UPSTREAM_FINDINGS.md)) are exactly this cell
  family.
- **Checker**: [`check_min_vote_policy`](../../sequent-core/src/ballot_codec/checker.rs#L130)
- **Interaction**: Interacts with `BlankVotePolicy` (blank = 0 selections)
  and `UnderVotePolicy` (under = between min_votes and max_votes).

### max_votes

- **Type**: `Contest.max_votes: i64`
- **Semantics**: Maximum number of selections allowed. Behavior when
  exceeded depends on `OverVotePolicy`.
- **Checker**: [`check_over_vote_policy`](../../sequent-core/src/ballot_codec/checker.rs#L187)
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

### Sanity checks (configuration-level)

[`check_max_min_votes_policy`](../../sequent-core/src/ballot_codec/checker.rs#L87)
validates that `max_votes` and `min_votes` are convertible to `usize`
(non-negative, within bounds). This is a configuration-level check, not a
voter-action check — it fires first and short-circuits if the contest config
itself is malformed.

There is now a second configuration-level checker:
[`check_contest_configuration`](../../sequent-core/src/ballot_codec/checker.rs#L37)
(wrapped as `validate_contest_configuration` in
[`contest_context.rs`](../../sequent-core/src/ballot_codec/contest_context.rs)),
which rejects a contest that defines **more than one explicit-blank marker
candidate** (and mirrors the same rule for explicit-invalid markers). It runs
when the codec context is constructed, so a malformed contest fails before
any ballot is encoded or decoded. "Config rejected" is thus a distinct
outcome class that exists before any per-ballot validation.

### Selection counting and marker candidates

The decode path computes **two different selection counts**, and which rules
use which is load-bearing
([`raw_ballot.rs#L382-L405`](../../sequent-core/src/ballot_codec/raw_ballot.rs#L382)):

- `num_selected_candidates` — selections **excluding** explicit-blank marker
  candidates.
- `num_selected_with_markers` — that count **plus one** for a selected
  explicit-invalid flag **plus one** for a selected explicit-blank marker.

The min-vote, over-vote, under-vote and blank-vote rules all use the
**marker-inclusive** count. Consequences worth internalising:

- An explicit-blank marker selected alone **satisfies `min_votes: 1`** and
  prevents the blank checker from firing — at the booth, an explicit blank
  is *not* a blank ballot. (At tally it classifies as `ExplicitBlank`.)
- A marker plus `max_votes` regular candidates trips the over-vote checker.
- The encoding itself carries the markers as leading flag slots:
  `choices[0]` is the explicit-invalid flag, and — **only when the contest
  defines an explicit-blank candidate** — `choices[1]` is the explicit-blank
  flag. The wire layout of a contest therefore depends on its candidate
  configuration.

The submission gates compute their own marker-inclusive count
(`selections_with_markers` in `voting_screen.rs`) with a guard against
double-counting a marker that is already present in the decoded choices —
see "Submission Gating Detail" below.

**The gates and the checker do not count the same thing** (finding **S6**,
[UPSTREAM_FINDINGS.md](UPSTREAM_FINDINGS.md)). The checker counts
`choice.selected > -1` — every selection. Both gates count
`choice.selected == 0`. On a plurality contest those are the same
predicate, because every selection sits at rank 0. On a **preferential**
contest `selected` holds the *rank*, so the gates are counting **first
preferences**: a well-formed ranking has exactly one, and the gates
therefore see 1 selection however many candidates the voter ranked.

Every gate clause that consults the count — blank (`n == 0`), over-vote
(`n > max`), under-vote (`min ≤ n < max`) — is affected on ranked ballots.
The tally is **not**: it reads the checker's emissions, so no vote is
miscounted; what diverges is the dialog the voter meets. Min-vote is
unaffected for a structural reason worth knowing — it has no gate clause
at all, reaching the gates only through the generic "any error" clauses,
which read the error list rather than any count.

---

## Vote Validation Policies (6 policies)

All six policies live in `ContestPresentation` (the presentation sub-object
of a `Contest`). Each has a fixed set of enum variants that control:
- Whether the condition produces an **error** (hard block) or **alert** (soft warn).
- Whether the condition is surfaced **during voting**, **only in review**, or **not at all**.

### Policy severity model

The submission-gate in [`voting_screen.rs`](../../sequent-core/src/util/voting_screen.rs)
does NOT simply classify policy variants into tiers. Instead, it evaluates
**specific condition+policy combinations** (see "Submission Gating Detail"
below). However, the general pattern is:

- **Hard blockers**: `NOT_ALLOWED*` variants of a policy cause the gate to
  prevent navigation when the corresponding condition is detected.
- **Soft warnings**: `WARN*`, `ALLOWED_WARN*`, `ALLOWED_WITH_MSG_AND_ALERT`
  variants cause a dismissible dialog.
- **Silent**: `ALLOWED` variants (except `ALLOWED_WITH_MSG*`) do not trigger
  any gate at all.

The mapping is not uniform across policies — each policy has its own
specific gating logic. See "Submission Gating Detail" for the precise
conditions.

---

### 1. InvalidVotePolicy

This policy has a **triple role** that makes it the most complex policy:

1. **Checker role** (`check_invalid_vote_policy`): Controls what happens when
   the voter selects a candidate whose `CandidatePresentation.is_explicit_invalid`
   is `true` — a special "vote invalid" option in the ballot UI.
2. **UI filter role** (`InvalidErrorsList.tsx`): Acts as a **master switch**
   that controls whether `invalid_errors` produced by *other* checkers
   (overvote, min_vote, etc.) are displayed to the voter.
3. **Submission gating role** (`voting_screen.rs`): When set to `NOT_ALLOWED`,
   the presence of *any* `invalid_errors` (from any checker) triggers a hard
   block.

#### Explicit vs Implicit Invalid Votes

- **Explicit invalid**: The voter selected a candidate with
  `is_explicit_invalid: true` in the election config. This is a deliberate
  opt-in — a "null vote" button. The flag is encoded as the first element
  of the choices array (`choices[0] = 1`). Since the explicit-blank feature
  (#2842), a contest that defines an explicit-blank marker candidate gets a
  second flag slot (`choices[1]`) for the blank marker — see "Selection
  counting and marker candidates" above.
- **Implicit invalid**: The ballot violates structural rules (overvote,
  undervote, blank, rank issues) without the voter explicitly opting into
  invalidity. All other checker errors are tagged
  `InvalidPlaintextErrorType::Implicit`.

#### Checker behavior

The checker *only* fires when `is_explicit_invalid == true`:

| Variant | Checker output |
|---------|----------------|
| `ALLOWED` (default) | Nothing — explicit invalid is silently accepted. |
| `WARN` | Nothing — **identical to ALLOWED at the checker level.** |
| `WARN_INVALID_IMPLICIT_AND_EXPLICIT` | Pushes alert (`errors.explicit.alert`) to `invalid_alerts`. |
| `NOT_ALLOWED` | Pushes error (`errors.explicit.notAllowed`) to `invalid_errors`. |

(A fifth value, `allowed-with-exclusive-explicit`, exists on the
`release/10.0` branch family only — #2949; see
[UPSTREAM_FINDINGS.md](UPSTREAM_FINDINGS.md) S5 and
[INVALID_VOTE_POLICY_INTENT.md](INVALID_VOTE_POLICY_INTENT.md) §8.)

Note: `WARN` and `ALLOWED` produce the same checker output (none). Their
difference is entirely in the UI filter and submission gating.

#### UI filter behavior (`InvalidErrorsList.tsx`)

When `InvalidVotePolicy == ALLOWED`, the `filterErrorList` function
**suppresses `invalid_errors`** from all other checkers, with two exceptions:

- `errors.implicit.selectedMax` survives whenever `OverVotePolicy` is
  anything **other than `ALLOWED`** — i.e. under four of the five variants,
  *including the default* `ALLOWED_WITH_MSG_AND_ALERT` (the code condition
  is `over_vote_policy !== ALLOWED`).
- `errors.implicit.blankVote` survives if `BlankVotePolicy` is `NOT_ALLOWED`.

When the policy is anything else (`WARN`, `WARN_INVALID_IMPLICIT_AND_EXPLICIT`,
or `NOT_ALLOWED`), the UI filter does **not** suppress — all `invalid_errors`
are shown.

This means `ALLOWED` says approximately: "don't show the voter structural
error messages" — but under a **default-configured** contest the overvote
error still shows (because the default overvote policy is not `ALLOWED`),
so the suppression is total only when `over_vote_policy == ALLOWED` too.
`WARN` says: "show all structural error messages" (even though neither
produces its own checker output for explicit invalid).

#### Submission gating behavior

In `check_voting_not_allowed_next_util` (hard block):
- If `invalid_vote_policy == NOT_ALLOWED` AND `invalid_errors` is non-empty
  → hard block. This catches *any* validation error (overvote, min_vote, etc.)
  — not just explicit-invalid.

In `check_voting_error_dialog_util` (soft warn):
- If `invalid_vote_policy != ALLOWED` AND `invalid_errors` is non-empty →
  show dialog. Again catches errors from any checker.
- Additionally: if `WARN_INVALID_IMPLICIT_AND_EXPLICIT` AND
  `is_explicit_invalid` → show dialog (even without other errors).

**Checker**: [`check_invalid_vote_policy`](../../sequent-core/src/ballot_codec/checker.rs#L331)

**UI layer**: [`InvalidErrorsList.tsx`](../../voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx)
reads the policy to filter errors from all checkers before rendering.

---

### 2. OverVotePolicy

**What it checks**: Whether the voter selected **more** candidates than
`max_votes` allows.

**Enum**: `EOverVotePolicy` — 5 variants

| Variant | Behavior |
|---------|----------|
| `ALLOWED` | Error generated (structural); no alert message. |
| `ALLOWED_WITH_MSG` | Error generated; alert message shown. |
| `ALLOWED_WITH_MSG_AND_ALERT` (default) | Error generated; alert message shown + dialog on submission. |
| `NOT_ALLOWED_WITH_MSG_AND_ALERT` | Error generated; alert message shown; **hard block** on submission. |
| `NOT_ALLOWED_WITH_MSG_AND_DISABLE` | Error generated; alert message shown; **UI disables checkboxes** at max. |

**Checker**: [`check_over_vote_policy`](../../sequent-core/src/ballot_codec/checker.rs#L187)

**Critical implementation detail**: When `num_selected > max_votes`, the
checker **always** pushes an entry to `invalid_errors` — regardless of the
policy variant. The policy only controls whether an *additional* entry is
pushed to `invalid_alerts`. The code comment says: *"for errors, we use only
invalid_vote_policy. Overvote policy is going to be used only for alerts."*

This means `invalid_errors` always contains the overvote error for structural
purposes (submission gating and round-trip verification), but whether the
voter *sees* it depends on `InvalidVotePolicy`'s UI filter (see §1 above).

**Special case — `num_selected == max_votes`**: When the policy is
`NOT_ALLOWED_WITH_MSG_AND_DISABLE` and the voter has selected exactly
`max_votes` candidates (not over), the checker pushes an *alert*
(`errors.implicit.overVoteDisabled`) to inform the voter that further
selections are disabled.

**Special UI behavior**: The `NOT_ALLOWED_WITH_MSG_AND_DISABLE` variant —
[`Question.tsx`](../../voting-portal/src/components/Question/Question.tsx)
disables unchecked checkboxes once `num_selected == max_votes`, preventing
the overvote at the UI level. The checker still validates as defense-in-depth.

**Submission gating**: Only `NOT_ALLOWED_WITH_MSG_AND_ALERT` triggers a hard
block in `check_voting_not_allowed_next_util`. The `NOT_ALLOWED_WITH_MSG_AND_DISABLE`
variant is *not* checked in the gate because the UI prevents the condition
from occurring.

---

### 3. UnderVotePolicy

**What it checks**: Whether the voter selected **fewer** candidates than
`max_votes` (but still ≥ `min_votes`). This is distinct from BlankVotePolicy
(which handles 0 selections).

**Enum**: `EUnderVotePolicy` — 4 variants

| Variant | Behavior |
|---------|----------|
| `ALLOWED` (default) | No alert generated. |
| `WARN` | Alert pushed to `invalid_alerts`; shown during voting. |
| `WARN_ONLY_IN_REVIEW` | Alert pushed to `invalid_alerts`; UI filters it out unless in review mode. |
| `WARN_AND_ALERT` | Alert pushed to `invalid_alerts`; additionally triggers submission dialog. |

**Checker**: [`check_under_vote_policy`](../../sequent-core/src/ballot_codec/checker.rs#L247)

**Critical detail**: This checker only pushes to `invalid_alerts`, **never**
to `invalid_errors`. Under-vote is always a soft warning — it can never
hard-block submission on its own.

**Note**: Under-vote is defined as `min_votes ≤ num_selected < max_votes`.
If `num_selected < min_votes`, that's a **min_vote violation** — an error
always pushed to `invalid_errors` regardless of UnderVotePolicy (handled by
`check_min_vote_policy`); whether it blocks, warns, or shows nothing at all
is decided by `invalid_vote_policy` (see *MinVotes* above — silent under
`allowed`).

---

### 4. BlankVotePolicy

**What it checks**: Whether the marker-inclusive selection count
(`num_selected_with_markers` — see "Selection counting and marker
candidates") is **zero** AND the voter did NOT select the explicit-invalid
candidate. Because markers count as selections, choosing an explicit-blank
marker means the ballot is **not** blank for this checker — it satisfies
`min_votes` and skips the blank policy entirely; the tally later classifies
it as `ExplicitBlank`.

**Enum**: `EBlankVotePolicy` — 4 variants

| Variant | Behavior |
|---------|----------|
| `ALLOWED` (default) | No output — checker skips entirely. |
| `WARN` | Alert pushed to `invalid_alerts`; dialog triggered on submission. |
| `WARN_ONLY_IN_REVIEW` | Alert pushed to `invalid_alerts`; UI filters it out unless in review mode. |
| `NOT_ALLOWED` | Error pushed to `invalid_errors`; hard-blocks submission. |

**Checker**: [`check_blank_vote_policy`](../../sequent-core/src/ballot_codec/checker.rs#L153)

**Important**: The checker is gated on `!is_explicit_invalid`. If the voter
selected the explicit-invalid candidate, the ballot is not considered "blank"
— it's "explicitly invalid" (handled by InvalidVotePolicy instead).

---

### 5. DuplicatedRankPolicy (preferential only)

**What it checks**: Whether the voter assigned the **same rank** to multiple
candidates in a preferential (IRV / Borda) contest.

**Enum**: `EDuplicatedRankPolicy` — 2 variants

| Variant | Behavior |
|---------|----------|
| `ALLOWED_WARN_AND_DIALOG` (default) | Error pushed to `invalid_errors`; triggers soft-warn dialog. |
| `NOT_ALLOWED_WARN_AND_DIALOG` | Error pushed to `invalid_errors`; triggers hard-block dialog. |

**Checker**: [`check_duplicated_rank_policy`](../../sequent-core/src/ballot_codec/checker.rs#L285)

**Critical detail**: Both variants produce **identical checker output** — an
`invalid_errors` entry is always pushed. The difference is solely in which
submission gate function reacts:
- `ALLOWED_WARN_AND_DIALOG` → `check_voting_error_dialog_util` shows a
  dismissible dialog.
- `NOT_ALLOWED_WARN_AND_DIALOG` → `check_voting_not_allowed_next_util`
  shows a non-dismissible dialog (hard block).

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
| `ALLOWED_WARN_AND_DIALOG` (default) | Error pushed to `invalid_errors`; triggers soft-warn dialog. |
| `NOT_ALLOWED_WARN_AND_DIALOG` | Error pushed to `invalid_errors`; triggers hard-block dialog. |

**Checker**: [`check_preference_gaps_policy`](../../sequent-core/src/ballot_codec/checker.rs#L308)

**Critical detail**: Same pattern as DuplicatedRankPolicy — both variants
produce identical checker output (`invalid_errors` entry). The variant only
affects which gate function reacts.

**Only applies to**: Same preferential contest types as DuplicatedRankPolicy.

---

## Error Display Pipeline

The flow from checker output to rendered UI involves three layers: Rust
checkers → WASM boundary → React UI filter.

### Layer 1: Checker output (Rust)

Each checker pushes entries to `invalid_errors[]` and/or `invalid_alerts[]`
on the `DecodedVoteContest` struct. The distinction:
- `invalid_errors` = structurally significant violations (used by submission
  gating AND potentially shown to voter).
- `invalid_alerts` = informational messages (used for UI display and dialog
  triggers only).

Note: Some checkers always push to `invalid_errors` regardless of policy
(overvote, duplicated rank, preference gaps, min_vote). The policy determines
*how the system reacts*, not whether the error exists.

### Layer 2: WASM → React

`wasm.ts` returns the full array of `DecodedVoteContest` as
`errorSelectionState`. The `<InvalidErrorsList>` component extracts the
contest matching its `question.id`.

### Layer 3: UI filter (`filterErrorList`)

Before rendering, `InvalidErrorsList.tsx` applies a multi-step filter:

1. **`invalid_alerts` filter** — removes:
   - `errors.implicit.underVote` when not in review mode and
     `under_vote_policy == WARN_ONLY_IN_REVIEW`.
   - `errors.implicit.blankVote` when not in review mode and
     `blank_vote_policy == WARN_ONLY_IN_REVIEW`.
   - `errors.implicit.overVoteDisabled` when in review mode (since the
     disable state is already visually obvious on the review screen).

2. **Touch gate** — if the contest is not "touched" (voter hasn't interacted
   with it) and it's not the review screen, ALL errors and alerts are cleared.
   This prevents error messages from showing before the voter has made any
   selection.

3. **Deduplication** — removes:
   - `errors.implicit.underVote` if `errors.implicit.blankVote` also exists
     (blank subsumes under-vote).
   - Duplicate `errors.implicit.selectedMax` entries.

4. **InvalidVotePolicy master filter** — when
   `invalid_vote_policy == ALLOWED`, removes ALL `invalid_errors` **except**:
   - `errors.implicit.selectedMax` when `over_vote_policy != ALLOWED`
     (four of five variants, including the default
     `ALLOWED_WITH_MSG_AND_ALERT`).
   - `errors.implicit.blankVote` when `blank_vote_policy == NOT_ALLOWED`.

5. **Render** — remaining `invalid_errors` render as
   `<WarnBox variant="warning">`, remaining `invalid_alerts` render as
   `<WarnBox variant="info">`.

---

## Submission Gating Detail

[`voting_screen.rs`](../../sequent-core/src/util/voting_screen.rs) exposes
two utility functions consumed via WASM. Both iterate all contests and check
the decoded contest state against specific policy+condition combinations.

### check_voting_not_allowed_next_util (Hard Block)

Returns `true` → dialog opens with **no "Continue" button**; voter must fix
the issue. Triggers when ANY contest matches ANY of these conditions:

| # | Condition | Rationale |
|---|-----------|-----------|
| 1 | Any `invalid_errors` entry has `error_type == Explicit \|\| EncodingError` | Encoding corruption or explicit-invalid error (from NOT_ALLOWED policy) |
| 2 | `invalid_errors` is non-empty AND `invalid_vote_policy == NOT_ALLOWED` | Master hard-block — any structural error when policy forbids invalid votes |
| 3 | `choices_selected == 0` AND `blank_vote_policy == NOT_ALLOWED` | Blank ballot prohibited |
| 4 | `choices_selected > max_votes` AND `over_vote_policy == NOT_ALLOWED_WITH_MSG_AND_ALERT` | Overvote hard-block |
| 5 | `duplicated_rank_policy == NOT_ALLOWED_WARN_AND_DIALOG` AND matching error exists | Duplicate ranks prohibited |
| 6 | `preference_gaps_policy == NOT_ALLOWED_WARN_AND_DIALOG` AND matching error exists | Rank gaps prohibited |

Note: `NOT_ALLOWED_WITH_MSG_AND_DISABLE` (overvote) is NOT checked here
because the UI prevents the condition by disabling checkboxes.

### check_voting_error_dialog_util (Soft Warn)

Returns `true` → dialog opens with **"Continue" and "Cancel" buttons**;
voter may dismiss and proceed. Triggers when ANY contest matches ANY of:

| # | Condition | Rationale |
|---|-----------|-----------|
| 1 | `invalid_errors` is non-empty AND `invalid_vote_policy != ALLOWED` | Any structural error when policy isn't fully permissive |
| 2 | `invalid_vote_policy == WARN_INVALID_IMPLICIT_AND_EXPLICIT` AND `is_explicit_invalid` | Explicit invalid selected with warn-both policy |
| 3 | `blank_vote_policy == WARN` AND `choices_selected == 0` | Blank ballot with warn policy |
| 4 | `choices_selected > max_votes` AND `over_vote_policy == ALLOWED_WITH_MSG_AND_ALERT` | Overvote with alert policy |
| 5 | `choices_selected > 0` AND `choices_selected >= min_votes` AND `choices_selected < max_votes` AND `under_vote_policy == WARN_AND_ALERT` | Under-vote with alert policy |
| 6 | `duplicated_rank_policy == ALLOWED_WARN_AND_DIALOG` AND matching error exists | Duplicate ranks allowed but warned |
| 7 | `preference_gaps_policy == ALLOWED_WARN_AND_DIALOG` AND matching error exists | Rank gaps allowed but warned |

### How the UI combines both gates

In [`VotingScreen.tsx`](../../voting-portal/src/routes/VotingScreen.tsx), the
`encryptAndReview()` handler:

```
if (showNextDialog() || disableNextButton()) → open dialog
```

The dialog rendered depends on `disableNextButton()`:
- **true** → `variant="softwarning"`, only an "OK" button (no escape route).
- **false** → `variant="action"`, "Continue" + "Cancel" buttons.

### `choices_selected` calculation

Both gate functions start from a raw count of selected choices:
```rust
choices.iter().filter(|choice| choice.selected == 0).count()
```

and then compute the count the conditions actually use,
`selections_with_markers`: the raw count **plus one** when
`decoded_contest.is_explicit_invalid` is set *and* the explicit-invalid
marker candidate is not already present among the decoded choices. The
guard prevents double-counting: in single-contest encodings the invalid
flag travels separately from the choices, while marker candidates that
appear *as choices* are already in the raw count. The blank/overvote
gating rows in the tables above all compare `selections_with_markers`,
mirroring the decode-side `num_selected_with_markers` semantics.

For **plurality**, `selected = 0` means "chosen" and `selected = -1` means
"not chosen" — so the raw count correctly counts selections.

For **preferential**, `selected` encodes the rank (0 = rank 1, 1 = rank 2,
etc.). The `== 0` filter only counts candidates at rank 1. In practice this
is acceptable because:
- Overvote/blank gating conditions are designed for plurality contests.
- Preferential contests rely on DuplicatedRankPolicy and PreferenceGapsPolicy
  for their gating logic.

### Interaction with pagination

The voting screen paginates contests (`contests: IContest[][]` — array of
pages). The gate functions check **all contests across all pages** (via the
`decodedContests` record), not just the currently visible page. This means a
hard-block error on page 1 will prevent navigation to review even if the
voter is currently on page 3.

---

## Tally-Time Classification

Everything above describes booth-time behaviour. At tally, each decoded
ballot is classified into exactly one of **six** classes by
`classify_ballot`
([`velvet-core/src/counting/extended_metrics.rs`](../velvet-core/src/counting/extended_metrics.rs);
upstream this logic lives inline in velvet's `do_tally`). The precedence
order is part of the specification:

```
1. is_decline_to_vote:  blank ballot        → Declined
                        anything selected   → ImplicitInvalid
2. is_invalid()  (= is_explicit_invalid || any invalid_errors):
                        explicit flag set   → ExplicitInvalid
                        otherwise           → ImplicitInvalid
3. explicit-blank marker selected:
                        + regular selection → ImplicitInvalid  (mix rule —
                          unreachable via the booth: the blank marker
                          clears co-selected regulars; non-booth state only)
                        alone               → ExplicitBlank
4. nothing selected                         → ImplicitBlank
5. otherwise                                → Valid
```

Aggregation facts that follow from the class, and are easy to get wrong:

- **Blank ballots (both kinds) count toward `total_valid_votes`** — they are
  valid ballots that name no candidate. Declined ballots are **excluded**
  from the valid total (accumulated in
  `extended_metrics.total_declined_to_vote`).
- The explicit/implicit splits surface as `blank_votes.{explicit,implicit}`
  and `invalid_votes.{explicit,implicit}` on `ContestResult`.
- Because `is_invalid()` includes *any* checker error, an overvote that the
  booth allowed through (soft-warn dismissed) is `ImplicitInvalid` here —
  the same class as the marker-mix rule, reached by a different route.

Note the classifier runs on **decoded** ballots: the checkers populate
`invalid_errors` during tally-side decode exactly as they do in the booth,
and the classifier consumes that plus the marker/decline structure. This is
the precise sense in which booth and tally share logic — the checkers — and
the precise sense in which they do not: steps 1, 3 and 4 above exist only
here.

---

## Key Source Files

| File | Role |
|------|------|
| [`sequent-core/src/ballot_codec/checker.rs`](../../sequent-core/src/ballot_codec/checker.rs) | All 9 checker functions (8 decode-time + `check_contest_configuration`); shared by booth and tally |
| [`sequent-core/src/ballot_codec/contest_context.rs`](../../sequent-core/src/ballot_codec/contest_context.rs) | Codec context: config validation, marker-candidate discovery, base layout (`single_contest_bases`) |
| [`velvet-core/src/counting/extended_metrics.rs`](../velvet-core/src/counting/extended_metrics.rs) | `classify_ballot` — tally-time six-class classification (see above) |
| [`voting-portal/src/store/ballotSelections/ballotSelectionsSlice.ts`](../../voting-portal/src/store/ballotSelections/ballotSelectionsSlice.ts) | Marker exclusivity reducer — prevents blank-marker + regular-candidate co-selection in the booth; deliberately does **not** clear under the null marker (S5) |
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
   (configurable)           (error always pushed;      (configurable)
                            gating/visibility set by
                            invalid_vote_policy —
                            silent under `allowed`)
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

**`num_selected` in this chart is the marker-inclusive count**
(`num_selected_with_markers`): a selected explicit-blank or
explicit-invalid marker counts as a selection, so "0" means *nothing at
all* was chosen — not merely no regular candidate.

For preferential contests, two additional checks apply regardless of count:
- **Duplicate ranks** → DuplicatedRankPolicy
- **Gaps in ranking** → PreferenceGapsPolicy

And across all contest types:
- **Invalid candidate selected** → InvalidVotePolicy
- **Blank marker selected** → not a policy check at the booth (it counts as
  a selection); classified `ExplicitBlank` at tally (`ImplicitInvalid` when
  mixed with a regular — a state the booth itself cannot produce, since the
  blank marker clears co-selected regulars)
- **Declined to vote** (election-level, multi-ballot only) → no checker;
  classified `Declined` (or `ImplicitInvalid` if non-empty) at tally
