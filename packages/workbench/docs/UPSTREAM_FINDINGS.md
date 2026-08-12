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

## 3. `InvalidErrorsList.tsx`: tautological dedup predicate

**Where:**
`packages/voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx`
(~L149-151):

```ts
// if overvote is an error, remove the info message
("errors.implicit.selectedMax" === error.message &&
    containsError(ret, "errors.implicit.selectedMax"))
```

The filter runs over `ret.invalid_alerts`, and `containsError`
searches `ret` — which still contains the very alert being examined —
so the predicate is true for every `selectedMax` alert and the alert
is dropped unconditionally. Currently benign (the checker always
pushes a paired `selectedMax` into `invalid_errors`, so the intended
"drop the alert copy when an error copy exists" would also always
fire), but the predicate is self-referential and will misfire the day
that pairing changes.

**How found:** `invalid_vote_policy` intent investigation
(`INVALID_VOTE_POLICY_INTENT.md`, 2026-08-12).

## 4. `InvalidErrorsList.tsx`: stale `useMemo` — policy arguments missing from deps

**Where:** same file, ~L173-193: `filterErrorList` takes
`invalid_vote_policy` and `over_vote_policy` as arguments, but the
`useMemo` dependency array lists neither — a policy change without a
selection change renders with the stale filter. Latent in production
(policies are static per session); an exhaustive-deps lint would flag
it.

**How found:** same investigation.

---

# Suspects — for consultation (adjudication pending)

All five are recorded, reproducible behaviours (pointers below); the open
question in each case is *intent*, not *fact*.

They are **not all the same kind of concern** — they sit on three distinct
axes, and conflating them muddles the consultation:

| Axis | What it is about | Suspects |
|---|---|---|
| **Silent discounting** | the voter is given no signal that their vote will not count | **S1** |
| **Marker semantics** | what an explicit-blank / explicit-invalid marker *means* and does | **S3** (a blank is subject to the count rules), **S5** (invalid preserves choices) |
| **Threshold consistency** | two mechanisms for one policy disagree on a boundary | **S4** |

**S2 is not a fourth axis — it is the intersection of S1 and S3.** A
deliberate explicit-blank, run through the `min_votes` rule at all
(S3's domain facet) and failing it, is discarded *silently* (S1). It is
kept as its own entry
only because it is the sharpest single cell where the two axes collide;
its two consultation questions belong one to each axis (should a blank be
subject to `min_votes`? → S3; should any rejection be silent? → S1). Read
S2 as a worked example, not a separate root cause.

**Reproducing the serious ones.** S1, S2, and S5 — the disenfranchisement
and privacy findings — have click-by-click workbench recipes in
[REPRODUCE.md](REPRODUCE.md), kept separate from the explanations here so a
reviewer can reach and check the behaviour firsthand.

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

**Structural characterization** (from the no-silent-discount query over
all six rules): a rule is silent-discount-prone iff its own policy can be
configured not to gate AND `invalid_vote_policy = allowed` removes the
generic gate. Over-vote (`allowed` variant) and min-vote (no policy) meet
this; the preferential rules do not (their enums have only
`*_WARN_AND_DIALOG` variants) and are provably immune. A candidate fix
therefore falls out: give every error-producing rule a mandatory dialog
variant. See VALIDATION_LOGIC_DISTILLATION.md §4.5.

**Evidence strength:** all violating cells (over-vote and both min-vote
sub-cases) are reproduced through ONE continuous run of the real workbench
pipeline — booth encrypt → cast → decrypt → decode → tally — with the
voter shown nothing and the ballot ending 0 valid / 1 implicit-invalid
(`characterization/{overvote,minvote}-e2e-pipeline.recorded.json`).

**Provenance of the silence** (full evidence chain:
[INVALID_VOTE_POLICY_INTENT.md](INVALID_VOTE_POLICY_INTENT.md)): the
message-layer suppression is recent. Until `7b0a1c71e8` ("🐞
Inconsistencies in Voting Portal (#2018)", 2025-09-29, meta#8235) the
booth filter never consulted `invalid_vote_policy` — an over-vote or
below-min ballot under `allowed` showed its inline error by the review
screen at the latest, so the original posture was "no dialog, but
informed". That commit added the suppression (untested, undocumented)
that composes with the older dialog-gate condition into full silence.
Note also that the min-vote family fires under **factory defaults**
(`allowed` is the platform default; only `min_votes ≥ 1` is needed).
**Sharpened consultation question:** did meta#8235 intend to suppress
implicit-invalid messages under `allowed`, or is the suppression
overreach in a marker-display fix?

**Why suspect:** the voter is given zero indication their vote will not
count. **Confidence:** strong intuition this is a defect (or at minimum a
combination that must be surfaced to election designers — e.g. an
admin-portal configuration warning). **Consultation question:** is
`invalid_vote_policy = allowed` intended to mean "invalid ballots may be
*cast* silently" even though they are discarded, and if so, should the
combination be flagged at configuration time? **Reproduce:**
[REPRODUCE.md](REPRODUCE.md) Part 1, Recipe 1 (over-vote) and Recipe 2
(below-min).

## S2. (S1 ∩ S3) A deliberate explicit-blank vote silently discarded when `min_votes ≥ 2`

*This is the intersection of S1 (silent discounting) and S3 (a deliberate
blank subject to `min_votes`), not an independent finding — see the
axes table above. Recorded separately because it is the sharpest cell.*

**Observed** (`characterization/minvote-rule.md`, `min_votes=2 ×
marker_only`): a voter who selects the explicit-blank marker — an
unambiguous, deliberate expression of "blank vote" — has the ballot
silently classified `ImplicitInvalid`. The silence is the S1 facet. The
S3 facet is the rule's *applicability*, not the count value: the
`min_votes` check runs against a deliberately-blank ballot at all, and
the resulting `selectedMin` error outranks the blank marker at
classification (`classifier-table.md`: errors × marker →
ImplicitInvalid; precedence decline → invalid → mix → marker → empty →
valid). That the marker counts as one selection is immaterial here — a
non-counting design would fail `0 < 2` identically. Counting decides
only *where* the collision first appears: at `min_votes: 1` the
marker-inclusive count **rescues** the blank (1 ≥ 1, no error, tallied
`ExplicitBlank` — `minvote-rule.md`, `min=1 × marker_only`), which is
why S2 exists only at `min ≥ 2`.

A second harm, distinct from the silence: the ballot is not merely
discounted but **misreported**. A deliberate blank lands in the
implicit-invalid (accident) bucket instead of the explicit-blank
bucket — a first-class reported category (the results report carries an
"Explicit blank votes" line, and blanks count inside "total valid votes
(including blanks)"). Even the aggregate statistics misstate the
voter's intent. Qualitatively sharper than a plain S1 cell:
this is not voter inattention; a clearly expressed intent is dropped
without notice. Confirmed end-to-end through the full pipeline
(the voter clicks "Blank vote (explicit blank)", is shown nothing, ballot tallies
implicit-invalid) — `minvote-e2e-pipeline.recorded.json`, `min=2/marker_only`. **Consultation question:** should an explicit
blank ever be subject to `min_votes` at all (see S3), and if it is,
should its rejection ever be silent? **Reproduce:**
[REPRODUCE.md](REPRODUCE.md) Part 1, Recipe 2, variant d.

## S3. A deliberate blank is subject to the selection-count rules (the marker counting as one selection)

**Observed** (`raw_ballot.rs` decode: `num_selected_with_markers`;
recorded in `characterization/blank-rule.md` and `minvote-rule.md`): two
nested facts. (i) **Domain** — an explicit-blank ballot is run through
the min/max/under/blank count rules at all. (ii) **Count value** — the
selected marker counts as one selection in them. The count value masks
the domain fact in the common case: the marker *satisfies*
`min_votes: 1`, so a deliberate blank sails through. At `min_votes ≥ 2`
the mask fails and the **domain** fact produces S2 — the count value is
immaterial there (0 and 1 both fall below the min). **Why suspect:**
defensible design either way (a blank is "a choice" vs. "the absence of
choices"), but the `min ≥ 2` interaction produces S2, which suggests the
combination was not considered. **Confidence:** genuinely uncertain.
**Consultation questions:** (i) should a deliberately-blank ballot be
inside the count rules' domain at all? (ii) if it is, is the
marker-inclusive count intended semantics or an artifact of
implementation convenience?

## S4. Under-vote alert/gate threshold discrepancy at `n = 0`

**Observed** (`characterization/undervote-rule.md`): (a) with
`min_votes = 0`, the under-vote zone `min ≤ n < max` includes `n = 0`, so
the under-vote alert fires on a completely empty ballot, overlapping the
blank condition (the UI dedups only when a blankVote message is also
present); (b) the checker alerts at `n = 0` but the soft gate requires
`n > 0`, so the WARN_AND_ALERT dialog never fires for the empty ballot
the checker just alerted on.

**Root cause** (`checker.rs:check_under_vote_policy` vs
`voting_screen.rs:check_voting_error_dialog_util`): the boundary is defined
*twice*. For the ranked-choice rules the gate raises its dialog by reading
the checker's verdict — it searches `invalid_errors` for
`duplicatedPosition` / `preferenceOrderWithGaps` — so those cannot disagree
with the checker. The under-vote branch does not: it re-derives the zone
from `selections_with_markers` / `min` / `max`, and its re-derivation adds
an `n > 0` guard the checker lacks (the checker uses `n ≥ min` with `min`
defaulting to `0`). Two independent expressions for one boundary, drifted
at `n = 0`.

**Defect or intended?** Our read: the `n > 0` guard itself is probably
*deliberate* — it hands the empty ballot to the blank branch (which fires
at `selections_with_markers == 0`) to avoid double-dialoging. What is
defect-shaped is that this carve-out was made by *recomputing* in the gate
rather than in the shared predicate, so the checker never got the same
carve-out and still alerts "under-vote" on the very ballot the gate treats
as "blank". The gate already demonstrates the clean pattern (consume the
checker's verdict) for the ranked-choice rules; under-vote/over-vote/blank
were simply never migrated to it — this reads as incremental accretion, not
a designed split. The fix that removes the whole class: define the boundary
once (in the checker), have the gate consume its `underVote` alert, and
decide the `n = 0` hand-off to the blank rule in that one place.

**Confidence:** low stakes behaviourally (a missing dialog on an empty
ballot, which the blank policy usually covers), but the duplicated predicate
is a real latent-defect smell. **Consultation question:** is the `n = 0`
hand-off to the blank rule intended — and if so, should the checker and gate
share one predicate so they cannot drift again?

## S5. A null vote preserves the voter's candidate selections in the ciphertext

**Observed** (`characterization/invalid-latent-choices-e2e.recorded.json`;
mechanism traced in `ballotSelections` reducers and `raw_ballot.rs`
encode): the two markers are handled asymmetrically. Choosing the
explicit-**blank** marker goes through `setBallotSelectionBlankVote`, which
rewrites `choices` to deselect every other candidate — a blank clears the
ballot. Choosing the explicit-**invalid** (null) marker goes through
`setBallotSelectionInvalidVote`, which sets only the `is_explicit_invalid`
field and **leaves `choices` untouched**. The UI does not disable the
regular candidates either (`isSelectable = !isReview`), so a voter can
select a candidate *and then* mark the ballot null. The encoder writes each
regular candidate's bit unconditionally, so both the invalid flag and the
candidate selection are encrypted into the cast ballot.

**Confirmed end-to-end** (booth → encrypt → cast → decrypt → decode →
tally): with a regular candidate selected and the null marker set, the
recovered plaintext bigint is `3` (invalid bit + candidate bit), the
decoded ballot carries `is_explicit_invalid = true` **and** one regular
selection, and the tally is `ExplicitInvalid` (0 valid, the candidate
counted for no one). So the voter's would-be vote lives in the cast
ciphertext even though nothing tallies or (as far as the consumer census
found) reads it.

**Why suspect:** **not** a silent discount — the voter opts into the null
vote deliberately, and its exclusion from the count is intended. But the
choice-preservation has no apparent functional purpose (nothing consumes
those choices for a null ballot), it is asymmetric with the blank marker
which clears them, and it has a **privacy-adjacent** edge: a protest
voter's latent candidate preference is carried, encrypted, in their cast
ballot, recoverable by anyone who can decrypt it (a tally, an audit).
**Confidence:** uncertain whether intended; the privacy angle is the part
worth a careful answer. **Consultation question:** should the invalid
(null-vote) reducer clear `choices` the way the blank reducer does, so a
null ballot carries no latent candidate preference — or is preserving
them intended (and if so, why)? **Reproduce:** [REPRODUCE.md](REPRODUCE.md)
Part 2 (decrypt the cast ciphertext in the ballot pipeline; the choice
falls out of the plaintext).

**Upstream evolution (2026-08-05, substantially answers the mechanism
question):** `23932601d2` ("✨ Explicit Invalid/Decline to vote should
support mutual exclusivity (#2949)", on `release/10.0`, not yet on
`origin/main`) adds a fifth policy value
`allowed-with-exclusive-explicit` that does exactly what the question
above proposes — the null marker clears other selections — as an
**opt-in**, and documents why the default does not: combining
explicit-invalid with candidates "remains an intentionally supported
ballot shape for clients that rely on it". So preservation is
intended (client reliance) and exclusivity is now available. **Open
residue:** the privacy-adjacent facet — under the default the latent
preference is still encrypted into the cast ballot, and #2949 does
not address it. See
[INVALID_VOTE_POLICY_INTENT.md §8](INVALID_VOTE_POLICY_INTENT.md).
