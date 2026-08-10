<!--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Upstream findings — to be reported

Defects in production code discovered incidentally during workbench work
(characterization, consumer census). Collected here so they can be filed
upstream without cluttering the characterization artifacts. Remove entries
once a meta issue exists and note the issue number.

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
