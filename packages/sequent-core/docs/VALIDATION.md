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
| messages | `ContestValidator::messages` | ballot decoding, in [`ballot_codec/raw_ballot.rs`](../src/ballot_codec/raw_ballot.rs) |
| dialog | `BallotValidator::for_ballot(…)`, then `hard_gate()` / `soft_gate()` | [`util/voting_screen.rs`](../src/util/voting_screen.rs), reached from the booth's Next button through the `check_voting_not_allowed_next` and `check_voting_error_dialog` wasm exports |
| inline | `ContestValidator::filter_visible_messages` | the booth's `InvalidErrorsList.tsx`, through `filter_visible_messages_js` |
| reachability | `ContestValidator::selection_capped` | the booth's `Question.tsx`, deciding whether to disable the remaining controls, through `selection_capped_js` |
| reachability | `ContestValidator::apply` | the booth's selection reducer `ballotSelectionsSlice.ts`, applying what a marker clears, through `apply_selection_js` |
| tally | `ContestValidator::classify` | the counting algorithms in `velvet-core` |

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
| messages | seven checker calls sequenced inside decoding | one call |
| dialog | some sixty lines re-deriving counts and policies, written twice — once per gate | two delegating lines |
| inline | a filter in TypeScript carrying its own copy of the policy rules | one call through wasm |
| reachability | a selection count in one React effect, a disable flag in a second, and marker clearing spread across three reducers | two calls |
| tally | a classifier in another crate, reading the fields the rules had written | one call |

The duplication was not merely untidy. Where one decision was expressed
twice, the two expressions drifted, and section 4 is largely that drift.

## 4. Corrections to behaviour

Five behaviours change. Four follow from writing each rule once; the fifth
is a deliberate judgment about what a rule should mean.

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
effect read?" yields a dependency map: for each effect, the inputs that
change it and — more usefully — the inputs that provably cannot. Such a map
earns its keep because policies do not always do what their names suggest.
The over-vote policy, for instance, governs alerts and one gate clause, but
never whether the over-vote error is recorded, which is unconditional.

A generated map of this kind exists in the characterization work, at
`packages/workbench/characterization/effect-map.md` — but read it for what
it is. It was produced by evaluating the *frozen record of the previous
behaviour*, not these rules, and several of the corrections in section 4
changed precisely the dependencies it charts. It is a description of what
the behaviour was, useful for comparison and not as documentation of this
module.

No such map has been produced for the rules as they now stand. Doing so is
an application of the technique above rather than a separate tool: vary one
input at a time across the enumeration, and record for each effect which
inputs ever change it.
