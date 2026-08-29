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

Related, same function: `isVotedState` is threaded into
`filterErrorList` as a parameter but consulted only by a debug log —
a dead parameter since `7b0a1c71e8` dropped its uses from the filter
conditions while keeping it in the signature (verified 2026-08-15
during the workbench spec transcription).

**How found:** same investigation.

---

# Suspects — for consultation (adjudication pending)

All six are recorded, reproducible behaviours (pointers below); the open
question in each case is *intent*, not *fact*. Where the rationalized
reference (`validation-spec/`) takes a position on a suspect, that
position is a **fix decision made autonomously** — recorded in the
entry and in the fix ledger — and the upstream pull-request review is
where it is finally adjudicated: the workbench treats each such
decision as correct until overturned there, and acts on it again only
reactively. The intent questions below remain as evidence for that
review, not as gates on the work.

They are **not all the same kind of concern** — they sit on three distinct
axes, and conflating them muddles the consultation:

| Axis | What it is about | Suspects |
|---|---|---|
| **Silent discounting** | the voter is given no signal that their vote will not count | **S1** |
| **Marker semantics** | what an explicit-blank / explicit-invalid marker *means* and does | **S3** (a blank is subject to the count rules), **S5** (invalid preserves choices) |
| **Checker/gate disagreement** | the two mechanisms behind one policy do not decide from the same thing | **S4** (they differ in the *predicate*, at one boundary), **S6** (they differ in the *operand* — the selection count — on every ranked ballot) |

**S2 is not a fourth axis — it is the intersection of S1 and S3,**
realized by the classifier's invalid-outranks-marker precedence (see
S2's body). A deliberate explicit-blank, run through the `min_votes`
rule at all (S3's domain facet) and failing it, is discarded *silently*
(S1). It is
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

**Observed** (`characterization/no-silent-discount.md` — observation-based:
248 recorded cells scanned → 7 candidates (`tally = ImplicitInvalid` ∧ no
gate) → **5 browser-confirmed** at the review screen, two families): with
`invalid_vote_policy =
allowed`, a ballot that violates an
error-producing rule is cast with **no inline message, no dialog, and no
block**, then classified `ImplicitInvalid` at tally and excluded from the
valid total. The checker flags the error internally in every case; the
booth's message filter mutes it — under `invalid = allowed`,
`filterErrorList` (`InvalidErrorsList.tsx`) hides every `invalid_error`
except its two-entry **keep-list**: `selectedMax` survives iff
`over_vote_policy ≠ allowed`, `blankVote` iff `blank = not-allowed` —
and neither gate fires, while the tally still consumes the error via
`is_invalid()`.

| family | configuration | silently-discarded states |
|---|---|---|
| over-vote | `over_vote_policy=allowed` + `invalid=allowed` | over the max |
| min-vote | `min_votes ≥ 1` + `invalid=allowed` | below the min |

The keep-list explains the table's shape. Over-vote silence needs
`over_vote_policy = allowed` because `allowed` is that rule's only
*signal-free* variant: every other variant emits an inline alert from
the checker, keeps `selectedMax` visible via the keep-list carve-out,
and (for the `*_AND_ALERT` variants) raises a dialog. The
`allowed-with-msg × allowed × over_max` row shows this exactly: no
gate fires, yet the inline signal — observed live at the review screen
(`characterization/dom-validate.md`, over-vote table) — makes it a
non-violation.
Min-vote needs only `invalid = allowed`: `selectedMin` is on no
keep-list and min-vote emits no alert, so nothing can rescue it.

**Structural characterization** (from the no-silent-discount query over
all seven rules): a rule is silent-discount-prone iff all three hold —
(i) its checker emits an `invalid_error` (only errors reach the tally's
`is_invalid()`; alerts have no tally consequence, so without an error
there is nothing to discount), (ii) its own policy has a fully
*signal-free* configuration — one that emits no inline alert, retains
nothing via the filter's keep-list, and fires no rule-specific dialog
("does not gate" is too weak: over-vote `allowed-with-msg` does not
gate yet signals inline, and is recorded as a non-violation), and
(iii) `invalid_vote_policy = allowed` removes both generic signals —
the generic dialog gate and inline-error visibility (the filter's
mute). Over-vote (error emitted unconditionally; `allowed` is its only
signal-free variant) and min-vote (error unconditional; no policy of
its own, hence never any signal of its own) meet all three. The preferential rules fail (ii) —
only `*_WARN_AND_DIALOG` variants — and are provably immune; under-vote
fails (i) — its checker emits only alerts, so an under-voted ballot
stays `Valid` and there is nothing to discount; blank fails (i)∧(ii)
jointly — it emits an error only under `not-allowed`, which hard-gates;
the invalid rule fails (i) under the only configurations (iii) permits —
under `invalid = allowed` its marker sets a flag, not an error, and its
discard class (`ExplicitInvalid`) is voter-intended, excluded from the
property by definition.
A candidate fix therefore falls out: give every error-producing rule a
mandatory dialog variant. See VALIDATION_LOGIC_DISTILLATION.md §4.5.

**Evidence strength:** each of the five violating cells (over-vote plus
the four min-vote cells) is reproduced through one continuous run of the
real workbench pipeline — booth encrypt → cast → decrypt → decode → tally
— with the voter shown nothing and the ballot ending 0 valid / 1
implicit-invalid
(`characterization/{overvote,minvote}-e2e-pipeline.recorded.json`;
re-runnable in one command via `characterization/reproduce-verify.mjs`).
The booth-signal half (nothing inline at review, no dialog, reachable) is
additionally observed across every recorded (configuration ×
vote-state) cell of all seven rules in
`characterization/dom-validate.md` (233/233 matching the spec).

**Provenance of the silence** — the two families have different
histories, both pinned against the pre-image of `7b0a1c71e8` ("🐞
Inconsistencies in Voting Portal (#2018)", 2025-09-29, meta#8235, read
in full 2026-08-14: it asks only for *more* voter warnings, and never
mentions `allowed` or any suppression). **Min-vote**: the silence is
new in that commit, unrequested by its ticket, untested and
undocumented — accidental-collateral in shape, and it fires under
**factory defaults** (`allowed` is the platform default; only
`min_votes ≥ 1` is needed). **Over-vote**: the silence predates the
ticket and reads as intended semantics — but the same commit made error
emission unconditional, flipping that cell's tally outcome from *fully
counted* (every selection, even past max) to *silently discarded*. The
commit diffs behind both verdicts, per family:
[INVALID_VOTE_POLICY_INTENT.md §5](INVALID_VOTE_POLICY_INTENT.md).
**Sharpened consultation questions** (to
the commit's authors): for min-vote — was extending the `allowed`
silence to `selectedMin`, a rule with no policy of its own to restore
signal with, considered? For over-vote — is silence-by-double-`allowed`
acceptable design, and was the tally flip considered for elections
already configured that way?

**Why suspect:** the voter is given zero indication their vote will not
count. **Confidence:** per family, on the provenance above — min-vote
(four of the five cells, S2 included): strong intuition this is a
defect. Over-vote (one cell): the silence appears deliberate, so the
suspicion shifts from "accident" to "questionable design" — plus the
unexamined tally flip. Either way, at minimum a combination that must
be surfaced to election designers (e.g. an admin-portal configuration
warning). **Consultation question:** is
`invalid_vote_policy = allowed` intended to mean "invalid ballots may be
*cast* silently" even though they are discarded, and if so, should the
combination be flagged at configuration time? **Reproduce:**
[REPRODUCE.md](REPRODUCE.md) Part 1, Recipe 1 (over-vote) and Recipe 2
(below-min).

**Upstream evolution (2026-08-21, widens the finding).** `main` now
carries a fifth `invalid_vote_policy` value,
`allowed-with-exclusive-explicit` (#2941), added for a different purpose
(it makes the invalid marker mutually exclusive — the S5 resolution
below). It behaves **identically to `allowed` at the checker, both gates
and the message filter** — it emits nothing, gates like `allowed`, and
mutes with the same two-entry keep-list — verified exhaustively on the
catch-up branch (production ≡ spec on 345,600 headless cells, 0
disagreements). So it **inherits S1 exactly**: the no-silent-discount
property, evaluated over the certified domain, now reports the
silent-discount configurations DOUBLING from 80 to 160, the new 80 all
requiring `invalid = allowed-with-exclusive-explicit`, in the same two
families (`selectedMin`, `selectedMax`); 6,336 cells in total. Wherever
this finding reads `invalid = allowed`, read `invalid ∈ {allowed,
allowed-with-exclusive-explicit}` — both are signal-free at the filter,
so both discount silently. The consultation question stands for both.

**Rationalized reference (phase 3, 2026-08-28): fixed by judgment.**
The workbench's rationalized implementation
(`validation-spec/src/queries.rs`) removes the mute — every emitted
error renders inline; gates, dialogs and tally are unchanged, so the
allowed family still never interrupts ("informed but uninterrupted",
the documented pre-2025-09 posture). Consequence, asserted per cell in
`characterization/fix-diff.md`: silent discounting is unrepresentable
in `f_fixed` — 0 cells of 345,600, against the oracle's 6,336. The
grounds are recorded in the fix ledger (`quirks()` in
`validation-spec/src/lib.rs`). Like every fix-ledger decision this was
made autonomously and is finally adjudicated at the upstream
pull-request review; it stands until overturned there, and the
workbench acts on it again only reactively. The intent question above
(meta#8235's authors) remains as evidence for that review, not as a
gate.

## S2. (S1 ∩ S3) A deliberate explicit-blank vote silently discarded when `min_votes ≥ 2`

*This is the intersection of S1 (silent discounting) and S3 (a deliberate
blank subject to `min_votes`), not an independent finding — see the
axes table above. Recorded separately because it is the sharpest cell.*

**Observed** (`min_votes=2 × marker_only`; tally class in
`characterization/minvote-rule.md`, the silence — nothing inline at
review, no dialog — observed live in `characterization/dom-validate.md`
and confirmed by `no-silent-discount.md`): a voter who selects the
explicit-blank marker — an unambiguous, deliberate expression of "blank
vote" — has the ballot silently classified `ImplicitInvalid`. The silence is the S1 facet. The
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

For a blank ballot the discount and the misreport are **one act, not
two harms**: a blank elects nobody either way, so "discarding" it can
only mean booking it as `ImplicitInvalid` instead of `ExplicitBlank`.
What distinguishes S2 from the generic S1 cells is where that act
lands. In the generic cells the landing category is *truthful* — an
over-vote really does violate the rule it is booked under, whatever the
voter's intent — so the published record stays internally accurate and
the harm is the voter's lost chance to fix the ballot. Here the same
single reclassification also rewrites the published record: it shifts
`total_valid_votes` (blanks count as valid — "not invalid and not
declined", `velvet-core/src/result.rs`), `blank_votes.explicit`, and
`invalid_votes.implicit`, plus their percentages — and those categories
exist to separate declared intent from derived condition (the docs
gloss explicit invalid as "null votes, spoiled ballots, protest
actions"; implicit as "invalid due to configuration"), so a declared
abstention is published as derived invalidity. Whether
"misreport" is the right word presupposes the S3(i) domain answer:
under rules-as-implemented the label is internally consistent.
Qualitatively sharper than a plain S1 cell:
this is not voter inattention; a clearly expressed intent is dropped
without notice. Confirmed end-to-end through the full pipeline
(the voter clicks "Blank vote (explicit blank)", is shown nothing, ballot tallies
implicit-invalid) — `minvote-e2e-pipeline.recorded.json`, `min=2/marker_only`. **Consultation question:** should an explicit
blank ever be subject to `min_votes` at all (see S3); if it is, should
its rejection ever be silent; and even if both stand, should a
failed-minimum blank still be *classified and reported* as a blank
rather than as implicit-invalid (a third lever, in the classifier)?
**Reproduce:**
[REPRODUCE.md](REPRODUCE.md) Part 1, Recipe 2, variant d.

**Fix decision (2026-08-28).** Decided autonomously, like every entry
in the fix ledger — the upstream pull-request review is where these
judgments are finally adjudicated, and the decision stands until
overturned there: **explicit blank votes are not subject to the
min-vote rule.** The rationalized reference implements it
(`validation-spec/src/queries.rs`, `is_deliberate_blank`): the
marker-only ballot emits no `selectedMin`, so it is reported as what
the voter declared — `ExplicitBlank` at every `min_votes` — and no
gate fires on it (4,800 cells; `characterization/fix-diff.md`, the
S2S3 bucket). All three of this entry's levers are answered at once by
the one move: the blank is outside the rule's domain, so nothing is
rejected, nothing is silent, and nothing is re-booked. The workbench's
position on S2 is settled — no further action here unless the review
rejects the rule.

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

**Fix decision (2026-08-28).** Decided autonomously (adjudication
happens at the upstream pull-request review; the decision stands until
overturned there), settling facet (i) for the min-vote rule: **explicit
blank votes are not subject to it.** Scope, precisely: the exemption
covers a ballot whose content is the blank marker alone
(`validation-spec/src/queries.rs`, `is_deliberate_blank`) and the
min-vote rule only — the marker still counts as a selection in the
blank rule (which is what keeps `blankVote` from ever firing on a
deliberate blank) and in the over/under zones, and the invalid flag's
counting is untouched (a null ballot is not an explicit blank vote).
The `min = 1` rescue is thereby subsumed: the deliberate blank passes
at every minimum because the rule no longer applies to it, not because
its count clears the bar. No further workbench action unless the
review rejects the rule.

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
found) reads it. Both directions of the asymmetry are also observed live
in `characterization/dom-validate.md`: the invalid `marker_plus` cell
*forms* ({regular + null marker}, reachable), while the blank
`regular_then_marker` cell *collapses* (`no (cleared)` — the marker
deselects the regular).

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

**Upstream evolution (landed on `main`, and now characterized):** the
fifth policy value `allowed-with-exclusive-explicit` (#2941/#2949) does
exactly what the question above proposes — the invalid marker clears
other selections, and vice versa — as an **opt-in**, documenting why the
default does not: combining explicit-invalid with candidates "remains an
intentionally supported ballot shape for clients that rely on it". So
preservation is intended (client reliance) and exclusivity is now
available. As of the 2026-08-21 catch-up this value is on `main` and the
workbench characterizes it: the exclusivity clear is booth-confirmed —
`dom-validate.md`'s `(allowed-with-exclusive-explicit, marker_plus)` cell
records `reachable=false, constraintKind=marker_cleared` (the flag is
cleared when the regular is selected), the mirror of the blank marker.
**Open residue, unchanged:** the privacy-adjacent facet — under the
DEFAULT (`allowed`) the latent preference is still encrypted into the
cast ballot, confirmed end-to-end again on merged code
(`invalid-latent-choices-e2e`), and the new value does not address it
because it is opt-in. See
[INVALID_VOTE_POLICY_INTENT.md §8](INVALID_VOTE_POLICY_INTENT.md).

## S6. On a ranked ballot the submission gates count only first preferences

**Observed** (`characterization/gate-count-agreement.md`; both sites read
directly): one decoded ballot is counted two different ways.

| site | counts | line |
|---|---|---|
| checker | `choice.selected > -1` — every ranked selection | [`raw_ballot.rs:386`](../../sequent-core/src/ballot_codec/raw_ballot.rs) |
| both gates | `choice.selected == 0` | [`voting_screen.rs:59`](../../sequent-core/src/util/voting_screen.rs), `:188` |

`selected` is overloaded by contest type: on a plurality ballot `0` means
*chosen*, on a preferential ballot it is the **rank**. So the two
predicates coincide on plurality — which is why this is invisible there —
and diverge on a ranked ballot, where the gates are counting *first
preferences*. A well-formed ranking has exactly one, so **the gates see 1
selection however many candidates the voter ranked**, and every gate
clause that consults the count (blank `n == 0`, over-vote `n > max`,
under-vote `min ≤ n < max`) decides from a number unrelated to the ballot.

**Not a counting defect.** The tally class is decided from the checker's
emissions, which use the correct count; the gates feed nothing into the
tally. No vote is miscounted. This corrupts *what the voter is told at
casting time*, in both directions.

**Scope** (exhaustive over the 345,600 cells `headless-sweep.md`
certifies): the two counts disagree on 115,200 cells. On 106,624 another
clause fires either way, so nothing reaches the voter. On **8,576** the
dialog the voter meets differs from the one the ballot warrants:

| consequence | cells |
|---|---|
| dialog kind changed: dismissible → blocking | 2,192 |
| dialog kind changed: blocking → dismissible | 1,776 |
| dialog with **nothing** rendered inline (should be no dialog) | 1,952 |
| **missing** dialog the policy promises | 2,000 |
| spurious dialog (should be none) | 656 |

Malformed rankings (a duplicate or a gap) only ever see the dialog's
*kind* change, because their error's policy raises a dialog either way.
The **well-formed** rankings — the ordinary ranked ballot — are where a
dialog appears on a ballot the checker is content with, or the promised
dialog never fires.

**The sharpest pair**, both confirmed in a real booth, and both under
`under_vote_policy = WARN_AND_ALERT`, which promises the voter an inline
warning *and* a dialog:

- a voter ranks **all three** candidates (`min_votes ≤ 1`, `max 3`). The
  checker is satisfied — no error, no alert, ballot `Valid` — yet the
  gates place their count of 1 inside the under-vote zone and raise a
  confirmation dialog. Nothing is rendered inline, so the dialog has no
  accompanying text: an interruption with no explanation.
- a voter ranks **two of three** with `min_votes = 2`. The checker counts
  2, finds it inside the zone and emits `underVote`; the gates count 1,
  decide `1 >= 2` is false, and raise nothing. The warning appears; the
  confirmation step the policy specifies never happens.

So under that policy a well-formed ranked ballot essentially never gets
the behaviour the policy describes — it gets one half or the other,
depending on how many candidates were ranked.

**Reachability confirmed in the booth** (2026-08-18): a ranking with no
first preference at all (`[-1, 1, 2]`) forms exactly as requested, so the
count can genuinely reach 0 — which is what makes `blank_vote_policy =
not-allowed` hard-block a ballot with two candidates ranked on it as
"blank" (counterfactual-proven: with the gap policy held fixed, flipping
only the blank policy flips the gate).

**Provenance** (git archaeology, 2026-08-19 — history, not
interpretation): the `selected == 0` count entered the gates on
**2024-08-08** (#590, blank-vote policy) and **2024-08-25** (#628,
over-vote policy), when every contest was plurality and the predicate was
exactly "is selected". Instant-runoff support arrived **2025-11-30**
(#2068), fifteen months later. The IRV invalid-vote policies arrived
**2026-03-14** (#2414), and that commit *did* touch this file — 72
insertions, 0 deletions — adding the `duplicated_rank_policy` and
`preference_gaps_policy` clauses. Those two are the only clauses in the
file that do not read the count; they match on error messages. So the
ranked-ballot work opened these functions, added handling for the new
errors, and did not revisit what the pre-existing count means when
`selected` holds a rank.

**Why suspect:** the divergence looks like an unrevisited assumption
rather than a decision — no commit states an intent for the gates to
count first preferences, and the behaviour it produces is not coherent
under any reading of the policies (it both invents and suppresses
dialogs, on the same policy, depending on ballot length).
**Confidence:** high that the behaviour is unintended; the open question
is which count the gates *should* use, which is a design answer we cannot
give. **Consultation question:** should the submission gates count
selections the way the checker does — every ranked candidate — so that
`blank`, `over_vote` and `under_vote` policies mean the same thing on a
ranked ballot as on a plurality one? And if some clause is meant to key on
first preferences specifically, which, and why?

**A caution for whoever fixes it.** Min-vote is the one count-based rule
with **no gate clause at all** (`check_min_vote_policy` arrived
2025-09-29 in `7b0a1c71e8` / #2018 — the same commit behind S1 — and
checker-side only; the gates are organised by policy and min-vote has no
policy). That absence is why min-vote signalling is *correct* on ranked
ballots today. Giving the gates a min-vote clause while the count is
wrong would import this defect into the one rule currently free of it.

**Reproduce:** `node characterization/gate-count-agreement.mjs` derives
the whole table from the certified spec, headlessly, in about a minute.
