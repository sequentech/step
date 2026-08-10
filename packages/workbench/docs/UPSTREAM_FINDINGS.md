<!--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Upstream findings — to be reported or consulted

Collected during workbench work (characterization, consumer census) so
they can be raised upstream without cluttering the characterization
artifacts. Two kinds of entry, kept separate:

- **Defects** — behaviour that is wrong on its face (no design judgment
  required). File as bugs.
- **Suspects** — behaviour we can *characterize* precisely but cannot
  adjudicate: whether it is intended requires consultation with the
  people who hold design authority. Neither the workbench work nor its
  operator claims that authority; verdicts are outputs of consultation,
  not inputs. Confidence intuitions are noted because they guide
  attention, not because they decide anything.

Remove entries once a meta issue exists and note the issue number.

## 1. `mcballot_images.rs`: decline flag populated from the invalid flag; error lists stubbed empty

**Where:** `packages/velvet/src/pipes/ballot_images/mcballot_images.rs`
(~L770–776, as of `origin/main@0db8f855ec`):

```rust
let marked_contest = DecodedVoteContest {
    contest_id: contest.contest_id.clone(),
    is_explicit_invalid: contest.is_explicit_invalid,
    is_decline_to_vote: dbc.is_explicit_invalid,   // <-- ballot-level flag
    // FIXME
    invalid_alerts: vec![],
    // FIXME
    invalid_errors: vec![],
    ...
```

**Two distinct problems:**

1. `is_decline_to_vote` is filled from `dbc.is_explicit_invalid` — a
   *ballot-level* field with a misleading name. In the multi-ballot decode
   (`multi_ballot.rs` ~L209), when `include_decline_to_vote` is enabled the
   ballot-level bit `choices[0]` — which **is the decline bit** — is bound
   to a local *named* `is_explicit_invalid`. So the assignment may be
   behaviourally intended (decline bit → decline field) while reading as a
   copy-paste bug, or it may genuinely be wrong when the field carries an
   invalid flag. Either way the naming makes the code unreviewable: a
   decline bit travelling in a variable/field named `is_explicit_invalid`
   is a defect in itself. Suggested fix: rename the decoded ballot-level
   field to what it carries, and make this assignment self-evident.
2. The two `FIXME`s: `invalid_errors` / `invalid_alerts` are stubbed empty,
   so ballot images render every ballot as checker-clean regardless of its
   actual validity. If images are used for audit purposes this silently
   hides invalid markings.

**How found:** consumer census of `invalid_errors` / `invalid_alerts` read
sites (workbench characterization work, 2026-08-10). Ballot-images
functionality is not present in the workbench, so this consumer is out of
the characterization's scope — hence recorded here instead.

## 2. `voting_screen.rs`: debug log interpolates `min` for both fields

**Where:** `packages/sequent-core/src/util/voting_screen.rs`,
`check_voting_error_dialog_util`:

```rust
console_log!("max={min:?}, min={min:?}, blank_policy={blank_policy:?}, ...");
```

`max` is printed from `min`. Cosmetic (debug logging only), but it renders
the log line useless for diagnosing over-vote gating and it prints on
every gate evaluation in the browser console.

**How found:** the log surfaced in Node while running the headless
characterization harness (`packages/workbench/characterization/`).

---

# Suspects — for consultation (adjudication pending)

All four are recorded, reproducible behaviours (pointers below); the open
question in each case is *intent*, not *fact*.

## S1. Silent vote discounting under `invalid_vote_policy = allowed`

**Observed** (`characterization/no-silent-discount.md`, 196 cells, two
families): with `invalid_vote_policy = allowed`, a ballot that violates an
error-producing rule is cast with **no inline message, no dialog, and no
block**, then classified `ImplicitInvalid` at tally and excluded from the
valid total. The checker flags the error internally in every case; the
filter suppresses it and neither gate fires, while the tally still
consumes it via `is_invalid()`.

| family | configuration | silently-discarded states |
|---|---|---|
| over-vote | `over_vote_policy=allowed` + `invalid=allowed` | over the max |
| min-vote | `min_votes ≥ 1` + `invalid=allowed` | below the min |

**Why suspect:** the voter is given zero indication their vote will not
count. **Confidence:** strong intuition this is a defect (or at minimum a
combination that must be surfaced to election designers — e.g. an
admin-portal configuration warning). **Consultation question:** is
`invalid_vote_policy = allowed` intended to mean "invalid ballots may be
*cast* silently" even though they are discarded, and if so, should the
combination be flagged at configuration time?

## S2. A deliberate explicit-blank vote silently discarded when `min_votes ≥ 2`

**Observed** (`characterization/minvote-rule.md`, `min_votes=2 ×
marker_only`): a voter who selects the explicit-blank marker — an
unambiguous, deliberate expression of "blank vote" — has the ballot
silently classified `ImplicitInvalid`, because the marker counts as one
selection and 1 < 2. Sub-case of S1's min-vote family but qualitatively
sharper: this is not voter inattention; a clearly expressed intent is
dropped without notice. **Consultation question:** should an explicit
blank ever be subject to `min_votes` at all (see S3), and if it is,
should its rejection ever be silent?

## S3. Explicit-blank markers count toward `min_votes`

**Observed** (`raw_ballot.rs` decode: `num_selected_with_markers`;
recorded in `characterization/blank-rule.md` and `minvote-rule.md`): a
selected explicit-blank marker counts as a selection for the min/max/
under/blank rules — so it *satisfies* `min_votes: 1`, and *fails*
`min_votes: 2` (producing S2). **Why suspect:** defensible design either
way (a blank is "a choice" vs. "the absence of choices"), but the
interaction with `min_votes ≥ 2` produces S2, which suggests the
combination was not considered. **Confidence:** genuinely uncertain.
**Consultation question:** is the marker-inclusive count intended
semantics or an artifact of implementation convenience?

## S4. Under-vote alert/gate threshold asymmetries at `n = 0`

**Observed** (`characterization/undervote-rule.md`): (a) with
`min_votes = 0`, the under-vote zone `min ≤ n < max` includes `n = 0`, so
the under-vote alert fires on a completely empty ballot, overlapping the
blank condition (the UI dedups only when a blankVote message is also
present); (b) the checker alerts at `n = 0` but the soft gate requires
`n > 0`, so the WARN_AND_ALERT dialog never fires for the empty ballot
the checker just alerted on. **Why suspect:** the thresholds of two
mechanisms that appear to implement one policy differ; possibly
deliberate (blank policy owns the empty case), possibly an off-by-design
inconsistency. **Confidence:** low stakes either way; worth a question,
not an alarm. **Consultation question:** are the alert and gate meant to
share a threshold?
