<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Vote validation

Everything the platform decides about a voter's selections — what they are
told, whether they may proceed, and how the ballot counts — is decided by
one module, [`src/validation.rs`](../src/validation.rs). This document says
what those decisions are, where the rest of the system asks for them, and
how to reason about them.

## 1. What validation produces

Validation yields exactly four things. Everything else is plumbing that
carries one of them somewhere.

| effect | what it is | values |
|---|---|---|
| **messages** | the errors and alerts recorded on a ballot when it is decoded | a set of message keys, each with the parameters its translation needs |
| **dialog** | what the voter meets on clicking Next | nothing, a confirmation they may dismiss, or a block they must fix |
| **inline** | the warning boxes under a contest | a set of message keys, differing between the voting screen and the review screen |
| **reachability** | whether the booth will let a selection form at all | a control that refuses, or a marker that clears what it contradicts |
| **tally** | how the cast ballot counts | one of six classes: valid, blank or invalid (each explicit or implicit), or declined |

That is five rows for four effects because **messages are not an effect the
voter perceives** — they are the intermediate the other three read. Nothing
renders a message key directly: the inline rules choose which are shown, the
gates ask only whether any error exists and of what kind, and the tally asks
only whether the list is empty.

The list is closed by construction rather than by luck. Every consumer of
validation state in the platform reads one of these, and the module has no
other public answer to give. A consumer that needs something else is a
signal that this list is wrong, not that the module needs a back door.

## 2. Where the system asks

Each effect has one query, and the call sites are few enough to name in
full.

| effect | the query | asked by |
|---|---|---|
| messages | `ContestValidator::messages` | ballot decoding, in [`ballot_codec/raw_ballot.rs`](../src/ballot_codec/raw_ballot.rs) and [`ballot_codec/multi_ballot.rs`](../src/ballot_codec/multi_ballot.rs) — one contest at a time, and every contest packed into one ciphertext |
| dialog | `BallotValidator::for_ballot(…)`, then `hard_gate()` / `soft_gate()` | [`util/voting_screen.rs`](../src/util/voting_screen.rs), reached from the booth's Next button through the `check_voting_not_allowed_next` and `check_voting_error_dialog` wasm exports |
| inline | `ContestValidator::filter_visible_messages` | the booth's `InvalidErrorsList.tsx`, through `filter_visible_messages_js` |
| reachability | `ContestValidator::selection_capped` | the booth's `Question.tsx`, deciding whether to disable the remaining controls, through `selection_capped_js` |
| reachability | `ContestValidator::apply` | the booth's selection reducer `ballotSelectionsSlice.ts`, applying what a marker clears, through `apply_selection_js` |
| tally | `ContestValidator::classify` | velvet's `classify_ballot`, in [`counting_algorithm/utils.rs`](../../velvet/src/pipes/do_tally/counting_algorithm/utils.rs), which the plurality and instant-runoff algorithms call for every cast ballot |

Two shapes recur, and the difference between them matters:

- **`ContestValidator`** answers about one contest. Building one never
  fails, because a contest whose `min_votes` / `max_votes` cannot be read as
  counts can still say what the voter sees on screen. Only the questions
  that compare a selection count against those bounds — the messages and the
  gates — can fail, and they say so in their return type.
- **`BallotValidator`** answers about a whole ballot, which is a different
  question rather than a larger one: the gates fire if *any* contest blocks,
  acclaimed contests are skipped because they offer nothing to select, and a
  contest whose decoded state is missing blocks rather than being presumed
  fine.

Within a contest the vote-state facts are derived once, and every answer is
a projection of that single derivation. That is what stops two questions
about the same ballot from disagreeing — the failure mode behind most of
the corrections in section 4.

## 3. How this replaced the previous arrangement

> **Transitional.** This section exists to help review the change that
> introduced the module. Delete it once that change has merged upstream: it
> describes an arrangement that will no longer exist.

The same five decisions were previously made in the places that needed
them, each deriving its own facts from the ballot:

| effect | before | now |
|---|---|---|
| messages | seven checker calls sequenced inside decoding, written once per encoding | one call, shared by both encodings |
| dialog | some sixty lines re-deriving counts and policies, written twice — once per gate | two delegating lines |
| inline | a filter in TypeScript carrying its own copy of the policy rules | one call through wasm |
| reachability | a selection count in one React effect, a disable flag in a second, and marker clearing spread across three reducers | two calls |
| tally | a classifier in another crate, reading the fields the rules had written | one call |

The duplication was not merely untidy. Where one decision was expressed
twice, the two expressions drifted, and section 4 is largely that drift.

## 4. Corrections to behaviour

Six behaviours change. Five follow from writing each rule once; the
remaining one is a deliberate judgment about what a rule should mean.

**One selection count.** The submission gates counted only candidates at
first preference while every other rule counted all selections. On a ranked
ballot the two disagreed: a voter ranking three candidates was three
selections to the rules and one to the gates, so gates fired on ballots the
rules were content with, and stayed silent on ballots the rules had
flagged. There is now one count, markers included, shared by every rule.

**One under-vote boundary.** The under-vote zone was defined twice, and the
definitions disagreed at the empty ballot: the checker alerted on it, the
gate did not. The empty ballot belongs to the blank rule, and the single
shared predicate now says so.

**Every error reaches the voter.** Under the permissive invalid-vote
policies the booth hid every error but two carve-outs, so a ballot could be
excluded at the count with nothing having told the voter that anything was
wrong. Errors now always render. The policy still governs interruption —
the permissive settings never block and never raise a dialog — but no
longer governs whether the voter is informed.

**A deliberate blank is not subject to the minimum.** Selecting the
explicit-blank marker is a statement, not a failure to choose, so the
min-vote rule no longer applies to it. Such a ballot previously fell below a
`min_votes` of 2 or more, was recorded as an error, and counted as
implicitly invalid; it now counts as the explicit blank the voter declared,
at every minimum. This one is a judgment about what the rule ought to mean
rather than the repair of an inconsistency.

**An alert is dropped when its message already shows as an error.** The
de-duplication compared an alert against itself, so it always fired — which
happened to give the right answer only because the error is always present
when the alert is.

**A decline is not held to the selection rules.** Declining says the voter
is not choosing, rather than choosing badly, so the over-, min-, under-
and blank-vote rules have nothing to judge. The rule existed, but in one
place only: the decoder that packs every contest into one ciphertext
skipped those rules for a decline, while everything else applied them. So
a ballot that recorded no error at all at the count still told the voter
it fell short of the minimum, and under a policy that reacts to that
message, stopped them at Next or asked them to confirm. One predicate,
`selection_rules_apply`, now answers for both, and for the gates as well
as the messages: five clauses across the two gates read the selection
rules directly rather than through the messages, so skipping only the
messages would have left the same split one level down.

One behaviour was examined and deliberately **kept**: marking a ballot
invalid does not clear the voter's selections, which are recorded but not
counted — except under the invalid-vote policy that makes the marker
exclusive. This is intended, and the module's tests pin it in both
directions so that a future tidying does not quietly remove it.

## 5. Analysing the rules

Because the rules are pure functions over small types, questions about
*every* ballot can be settled by evaluation rather than by argument. A
contest's configuration is six policies and two bounds; a vote state is a
count and four flags. The combinations that matter are finite, and small
enough to enumerate exhaustively in well under a second.

The recipe does not vary:

1. enumerate the configurations and vote states of interest;
2. build a `ContestValidator` for each and ask it for the effects;
3. assert the property over every result.

`src/validation.rs` carries one such analysis as a test,
`no_ballot_is_discarded_without_telling_the_voter`. It asserts the property
that the third correction in section 4 establishes: there is no reachable
combination in which the voter is shown nothing at either casting point and
the ballot is nonetheless excluded from the count. It is written to be read
as a worked example — copy it, change the predicate, and you have asked a
different question of the entire rule set.

Other properties worth asking this way: whether any policy combination can
produce a dialog with nothing rendered to explain it; whether two effects
that ought to agree ever disagree; and which inputs an effect actually
reads, which is the subject of the next section.

## 6. What each policy actually controls

Applying section 5's technique to the question "which inputs does this
effect read?" yields a dependency map. `src/validation.rs` carries it as
the test `which_inputs_move_which_effects`: it evaluates every
configuration in a domain — the six policies in every combination,
minimums `0..=2` against maximums `1..=3` (skipping the impossible
`min > max`), and eight ballots spanning both
contest kinds, each with and without the ballot marked invalid — then
varies one input at a time and records which effects move.

| input | messages | dialog | inline | reachability | tally |
|---|:-:|:-:|:-:|:-:|:-:|
| `invalid_vote_policy` | • | • | • | • |  |
| `blank_vote_policy` | • | • | • |  | • |
| `over_vote_policy` | • | • | • | • |  |
| `under_vote_policy` | • | • | • |  |  |
| `duplicated_rank_policy` |  | • |  |  |  |
| `preference_gaps_policy` |  | • |  |  |  |
| `min_votes` | • | • | • |  | • |
| `max_votes` | • | • | • | • | • |
| selections | • | • | • | • | • |
| blank marker | • | • | • | • | • |
| explicit invalid | • | • | • | • | • |

The last three rows are the ballot rather than the contest's
configuration, and together they are the count the bounds read: the
regular selections, plus one if the explicit-blank marker is set, plus one
if the ballot is marked invalid. `selections` is the regular candidates
alone — how many are picked on the plurality contest, and which of the
three rankings on the preferential one.

The two markers are named differently because they are different things.
The explicit blank is a candidate: the voter selects it as they would any
other, and it travels in the record as a selected choice. Explicit invalid
is a field on the record; a contest may render a candidate for it, but
clicking that candidate sets the field rather than selecting the
candidate, and encoding drops the candidate from the choices. So "blank
marker" names a selection and "explicit invalid" names a flag — which is
also why they reach the tally by different arguments to `classify`: the
blank marker through the selection class, the invalid flag through a
parameter of its own.

The blanks are the point, and they are stronger claims than the dots: an
input that moves an effect somewhere is a fact about one case, while an
input that moves it nowhere in an exhaustive domain is a fact about the
rules. Four are worth stating outright, because the names do not suggest
them.

**The two rank policies reach nothing but the dialog.** Both values of
each emit the same error; the policy decides only which gate reacts. A
duplicated rank is therefore recorded, displayed and counted identically
whichever way the contest is configured — the setting is purely about
whether the voter is blocked or merely asked to confirm.

**`under_vote_policy` cannot make a ballot invalid.** It raises an alert
and never an error, and the tally reads errors. Changing the setting never
changes whether a ballot counts.

**`invalid_vote_policy` cannot reach the tally either**, for a different
reason: it speaks only when the ballot is already explicitly invalid, and
that flag settles how the ballot counts on its own. It does reach
reachability, which `selection_capped` gives no sign of — that query never
reads the policy. The path runs through `apply`: under
`ALLOWED_WITH_EXCLUSIVE_EXPLICIT`, marking a ballot invalid clears the
selections, and the selection count is what the cap compares. A dependency
can arrive by changing the ballot rather than by reading the input.

**`over_vote_policy` cannot reach the tally, but `max_votes` can.** The
over-vote error is unconditional; the policy governs only the alert and
the "maximum reached" hint. Relaxing the policy never makes an
over-voted ballot count — raising the maximum does.

That asymmetry is the shape of the whole tally column. What a ballot
counts as turns on whether *any* error was emitted, so among the
configuration inputs the ones that decide an emission reach the tally and
the ones that only decorate an emission do not. `min_votes` is in the
first group, which is what makes the deliberate-blank correction in
section 4 a change to how ballots count rather than only to what the voter
is told. The ballot's own three inputs reach the tally by a second route
besides: with no error anywhere, what is selected still decides between
valid, blank and declined.

The test that produces this map also asserts it, but is marked `#[ignore]`:
enumerating the domain takes about three seconds, several times the rest of
the crate's tests together, and that is too surprising a toll to put on
everyone who runs them. So the table can drift from the rules, and the date
below is the guard against reading a stale one:

**Last verified: 2026-08-31.**

    cargo test -p sequent-core --features default_features --lib \
        validation::tests::which_inputs_move_which_effects \
        -- --ignored --nocapture

That prints the rows above and asserts the same map. A pass means this
section is current, and the date should be moved to today. A failure prints
the map as it now stands, which is what this section should then say — and
means some rule started or stopped reading an input, which is worth
understanding before the table is updated to match.

Run it whenever the rules change. Nothing else will notice if you do not.
