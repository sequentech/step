<!--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# `invalid_vote_policy` — intent evidence

Why does `invalid_vote_policy = allowed` exist, is it expected in real
elections, and what does the answer do to the silent-discount findings
(S1/S2) and the null-vote finding (S5) in
[UPSTREAM_FINDINGS.md](UPSTREAM_FINDINGS.md)?

Everything below is grounded in production sources the workbench lifts
unmodified (voting-portal, sequent-core, admin-portal), the shipped
docusaurus documentation, and git history. Quotes are verbatim and
commits are named so the chain can be re-audited. This is intent
*evidence* gathered for consultation, not adjudication. The pivotal
claims (the accessor rename, the filter pre-image, the release-branch
location of #2949) were verified directly against `git show`; doc and
i18n quotes carry their file paths for re-checking.

## 1. One dial, two policies

The enum answers two independent questions with a single value:

- what to do about **explicit** invalidity — the voter deliberately
  marks the ballot null (a protest / "voto nulo" act);
- what to do about **implicit** invalidity — the selections are
  invalid by rule (over-vote, below-`min_votes`), deliberately or not.

Responses to either kind sit on one severity ladder:

> silence → inline message → dismissible dialog → hard block

("Not permitted" is not a separate concept from signaling — a hard
block is the top of the ladder.) The four values are points in that
2-axis space:

| value | response to explicit invalid | response to implicit invalid |
|---|---|---|
| `allowed` | silence | **silence** (since 2025-09; before: message, no dialog) |
| `warn` | silence | message + dismissible dialog |
| `warn-invalid-implicit-and-explicit` | alert + dismissible dialog | message + dismissible dialog |
| `not-allowed` | hard block | hard block |

Two structural facts follow:

1. **`allowed` and `warn` are identical on the explicit axis.** The
   entire distinctive content of `allowed` is on the implicit axis:
   silence about rule-invalid selections. Yet the value's name, its
   docs gloss, and the enum's origin (§2) all advertise the explicit
   axis. An election designer who needs null votes reads "Allowed"
   correctly on the advertised axis and gets the other axis silently.

2. **Off-diagonal combinations are unreachable.** The exposed values
   keep the explicit response ≤ the implicit response. The point
   *(explicit → silence, implicit → hard block)* — "null votes are a
   frictionless right, accidents must be stopped" — is plausibly what
   a mandatory-voting jurisdiction with over-vote protection wants,
   and it cannot be configured: blocking implicit invalidity requires
   `not-allowed`, which also kills the null option. The per-rule
   policies rescue parts of it (`over_vote_policy =
   not-allowed-with-msg-and-disable` blocks over-votes
   independently; `blank_vote_policy = not-allowed` blocks the empty
   ballot), but the zone `0 < n < min_votes` has **no rule-specific
   policy at all**, so a partially-filled below-min ballot can only be
   hard-blocked by `invalid_vote_policy = not-allowed`. The min-vote
   family is simultaneously the silent-discount-prone one
   (VALIDATION_LOGIC_DISTILLATION.md §4.5) and the un-blockable one —
   both for the same reason: it has no policy of its own.

## 2. The original job: "may the voter null-vote?"

Until October 2024 the enum's only consumer-facing accessor was named
for the explicit axis (`packages/sequent-core/src/ballot.rs`, removed
by `68a558ef34`):

```rust
pub fn allow_explicit_invalid(&self) -> bool {
    ...
    [InvalidVotePolicy::ALLOWED, InvalidVotePolicy::WARN]
        .contains(&invalid_vote_policy)
}
```

The policy's job, in the codebase's own naming: *may this ballot be
explicitly marked invalid?* — with `allowed` and `warn` both meaning
yes. The commits that shaped it carry the same understanding:
`b837d992b0` / `68a558ef34` — "Invalid vote policy doesn't work as
expected **when explicit invalid vote is configured**" (#849) — treat
the policy and the explicit-invalid marker candidate as one coupled
feature.

## 3. Null votes are real-election machinery

The explicit-invalid category is not a diagnostic; it is built as a
statutory, reportable quantity:

- **A reserved wire slot in every ballot.** The codec reserves
  position 0 of every encoded contest for the explicit-invalid flag,
  unconditionally (`docs/docusaurus/docs/05-reference/07-ballot_encoding.md`
  §4.2: "The first position is always reserved for the explicit
  invalid flag").
- **A standing line in every results report.** The velvet PDF/HTML
  report prints "Explicit invalid votes" in the Participation table
  beside Census and blank votes
  (`packages/velvet/src/resources/report_content.hbs:167`).
- **A mandatory field for hand-counted paper ballots.** The
  tally-sheet import requires one `explicit_invalid` row per ballot
  box and reconciles `total_invalid = implicit_invalid +
  explicit_invalid`
  (`docs/.../08-03-election_management_election-event_tally-sheet-imports.md`).
  A category that must be supplied for paper counts is a category a
  jurisdiction reports.
- **The platform's own vocabulary is protest voting.** Candidate
  reference: the marker is "e.g., 'Null Vote,' 'Spoil Ballot'". Tally
  reference: "Explicitly Invalid Votes: Null votes, spoiled ballots,
  protest actions."

So the hypothesis "election authorities require voters to be able to
cast protest/null votes" is confirmed as a requirement the platform is
engineered to serve — at the encoding, reporting, and even
paper-reconciliation layers.

**Terminology guard — two senses of "spoil".** The platform also has
an election-level `spoil_ballot_option` column
(`windmill/src/postgres/election.rs`), which is **not** this feature:
it belongs to the Benaloh cast-or-audit challenge — a ballot
deliberately discarded to prove its encryption was honest, after
which the voter votes again; it never reaches the tally and has no
result category (voter docs: `03-voters/01-tutorials/
03-voter_audit_ballot.md`). The protest sense — the one this document
is about — is contest-level (`is_explicit_invalid` marker +
`invalid_vote_policy`) and is tallied and reported. The collision is
easy to fall into because the candidate docs suggest "Spoil Ballot"
as a marker name; the two mechanisms share nothing.

## 4. What `allowed` does *not* do: enable null voting

The follow-on hypothesis — that casting a null vote is only possible
with `invalid_vote_policy = allowed` — is false:

- **The booth never consults the policy to render or select the
  marker.** The null option is enabled by the *candidate* flag
  (`presentation.is_explicit_invalid`) plus `invalid_vote_position`;
  partitioning, rendering, and the selection reducer are all
  policy-blind (`categoryService.ts`, `Question.tsx`, `Answer.tsx`,
  `ballotSelectionsSlice.ts` in voting-portal / ui-core).
- **`warn` is behaviourally identical to `allowed` for an explicit
  null.** `check_invalid_vote_policy`
  (`sequent-core/src/ballot_codec/checker.rs:331-361`) pushes an
  error only under `not-allowed`, an alert only under
  `warn-invalid-implicit-and-explicit`, and nothing otherwise. The
  workbench's recorded matrix
  ([../characterization/invalid-rule.md](../characterization/invalid-rule.md))
  confirms: identical columns for `allowed` and `warn` on every
  explicit-invalid state.

Null voting is available under three of the four values. The only
capability `allowed` adds over `warn` is silence about **implicit**
invalidity — which is exactly the silent-discount surface (S1).

## 5. What `allowed` uniquely does — and since when

The full silence is recent. Until commit `7b0a1c71e8` ("🐞
Inconsistencies in Voting Portal (#2018)", 2025-09-29, parent issue
sequentech/meta#8235), the booth's message filter
(`voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx`)
**never consulted `invalid_vote_policy`**. Verified against the
commit's pre-image: `invalid_errors` were filtered only by a
"voter hasn't touched the contest yet" heuristic, and the review
screen always showed them. Under `allowed`, an over-voted or
below-min ballot displayed its inline error ("Number of selected
choices … is less than the minimum …") by review at the latest.

The original `allowed` posture was therefore **"no dialog
interruptions, but the voter is informed"** — no silent discount at
the message layer. `7b0a1c71e8` added the suppression block that
hides *all* `invalid_errors` under `allowed` (with two carve-outs:
over-vote when `over_vote_policy ≠ allowed`, blank when
`blank_vote_policy = not-allowed`). That block:

- arrived inside a bug-fix commit about display inconsistencies, not
  a feature commit;
- has no unit test (none exist for `InvalidErrorsList` at all);
- has no documentation (no JSDoc on the enum, no README mention in
  voting-portal / ui-core / ui-essentials);
- carries two incidental defects recorded as Defects 3–4 in
  UPSTREAM_FINDINGS.md (a tautological dedup predicate; a stale
  `useMemo` dependency list).

The behaviour the workbench characterized as S1 — checker flags
internally, voter sees nothing, tally discards — is the composition
of this 2025-09 message suppression with the older dialog-gate
condition (`!invalid_errors.is_empty() && policy != ALLOWED`, present
since the gates were written). Before 2025-09 the composition left
the inline message visible; after, nothing.

We could not read meta#8235 from this environment; it is the single
most informative artifact left. If it describes marker-display fixes
and never mentions suppressing implicit-invalid messages, the silence
is overreach in a bug fix. If it specifies the suppression, the
silence is intended and the consultation shifts to whether the intent
is safe.

## 6. Defaults, guidance, and steering

- `ALLOWED` is the platform default in three independent places:
  `#[default]` on the Rust enum (`ballot.rs:876`), the
  `ContestPresentation` constructor (`ballot.rs:1598`), and the
  admin-portal form fallback (`EditContestDataForm.tsx:483-484`).
- When the frontend default (`warn-invalid-implicit-and-explicit`)
  and backend default (`allowed`) were found to disagree, the fix
  (`ff6e47600d`, meta#2126, release 9.0.2) resolved **toward
  `allowed`** — the permissive value was deliberately confirmed as
  the platform-wide default over the warning value.
- The admin portal offers **no helper text, no tooltip, and no
  cross-field validation** for the policy: nothing links it to the
  explicit-invalid candidate (configured on a different screen),
  warns that `allowed` also silences min-vote/over-vote messaging, or
  warns that `not-allowed` makes a configured null option unusable.
- The admin docs gloss
  (`docs/.../04-contest/01-election_management_contest_data.md`):
  "When voter selection is invalid (e.g., null vote or too many
  selections): **Allowed**: Submit without warning." One line covers
  both axes; "without warning" reads naturally as "no dialog" against
  the pre-2025 behaviour and as "no signal at all" against the
  current one. The docs do not distinguish message from dialog.

Net effect: a designer whose requirement is "voters must be able to
cast null votes" is steered to `allowed` (correct on the advertised
axis, and the default anyway) and thereby also selects total silence
about accidental invalidity — undocumented, unflagged, and unneeded
for their requirement (`warn` would have satisfied it).

## 7. In-repo usage evidence

| artifact | value | explicit-invalid candidate? |
|---|---|---|
| voting-portal demo fixture `ELECTION_WITH_INVALID` (`src/fixtures/election.ts:402-517`) | `allowed` | yes — "Invalid vote", `invalid_vote_position: "top"`, min=max=1: the classic voto-nulo ballot layout |
| all other voting-portal fixture contests | `allowed` | no |
| windmill janitor precinct template (`external-bin/janitor/templates/contest.hbs:34`) | `not-allowed` | no (over-vote: `not-allowed-with-msg-and-disable`) |
| `step-cli/data/mock.json` | `not-allowed` ×24 | no |
| election-architect base template (unmerged branch `7c89ba69b3`) | `warn-invalid-implicit-and-explicit` | n/a |

Reading: `allowed` is the default posture and the shipped template
for a null-vote election; the configs in-repo that most resemble real
public deployments (the janitor precinct import) deliberately move
off it to `not-allowed`. Both real postures exist, which is the point
of having the policy — but note the janitor deployment could not have
offered null votes and over-vote blocking simultaneously if it had
wanted both (§1, off-diagonal gap).

An unmerged 2026 branch (`feat/meta-12769…`, `7c89ba69b3`) carries
the only prose rationale for `allowed` found anywhere: a `permissive`
preset described as "Accept whatever the voter does, and say nothing.
/ For an election where a spoiled or partial ballot is a legitimate
choice rather than a mistake to catch." Treat with caution: it
post-dates the 2025-09 change and reads as a recent engineer's
account of current behaviour, not as original design intent.

## 8. Upstream evolution: the exclusivity variant (#2949)

`23932601d2` ("✨ Explicit Invalid/Decline to vote should support
mutual exclusivity (#2949)", 2026-08-05) — merged to the
`release/10.0` branch family, **not yet on `origin/main`** and absent
from this workbench's tree — adds a fifth value,
`allowed-with-exclusive-explicit`: selecting the null marker clears
other selections and vice versa, mirroring the blank marker. Its doc
line also states why the default does *not* clear:

> "Combining an explicit-invalid selection with other candidates
> (e.g. via an older client) is still tallied like Allowed, since
> that combination remains an intentionally supported ballot shape
> for clients that rely on it."

This bears directly on S5: the reducer asymmetry the workbench
recorded is known upstream; clearing is now an opt-in policy variant;
and preservation-by-default is declared intentional on grounds of
client reliance. What #2949 does not address is S5's privacy facet —
under every `allowed`-family default the latent candidate preference
is still encrypted into the cast ballot.

**Workbench follow-up:** when #2949 reaches main, the invalid-rule
characterization gains a fifth policy value, and the marker-exclusivity
preventer (`ballotSelectionsSlice.ts`) becomes policy-dependent.

## 9. Bearing on the findings

**S1/S2 (silent discounting).** Confidence that this is a real
concern — *can occur in a real election* AND *is not intended* —
**increases on both conjuncts**:

- *Occurrence:* the min-vote family fires under **factory defaults**.
  `allowed` is the default; the only non-default ingredient is
  `min_votes ≥ 1`, an ordinary contest requirement. The over-vote
  family needs one non-default choice (`over_vote_policy = allowed`).
  Nothing in the admin portal or docs warns about either combination,
  and the null-vote requirement actively steers designers into the
  prone value (§6).
- *Intent:* the full silence is ten months old, introduced by a
  bug-fix commit, untested, undocumented, and it inverted the
  original "informed but uninterrupted" posture (§5). Where upstream
  *does* intend a surprising behaviour, it says so in writing —
  #2949's "intentionally supported ballot shape" line is the house
  style for documented intent — and no such language exists for
  implicit-invalid silence. Honest counterweights: the docs gloss
  "Submit without warning", the deliberate #2126 default choice, and
  the unmerged-branch preset prose; all three are consistent with
  intent but none distinguishes the message layer from the dialog
  layer or mentions the discount consequence.

The sharpest remaining question is now concrete and internally
answerable: **what does meta#8235 say?**

**S5 (null vote preserves choices).** Substantially answered by
#2949 (§8): preservation is intended (client reliance), exclusivity
is opt-in. Open residue: the privacy-adjacent facet.

**Remedies strengthened.** The §4.5 candidate fix (a mandatory-dialog
variant for every error-producing rule) plus a config-time lint stand;
§1 adds a third: the min-vote zone needs a policy of its own (or the
axes need unbundling) so that "frictionless nulls + blocked accidents"
becomes expressible at all.

## Sources

| commit | date | role |
|---|---|---|
| `b837d992b0` / `68a558ef34` | 2024 | policy ↔ explicit-marker coupling; removed `allow_explicit_invalid()` |
| `7b0a1c71e8` (#2018, meta#8235) | 2025-09-29 | added the `allowed` message suppression → full silence |
| `ff6e47600d` (#2126) | 2025-10 | settled the default toward `allowed` |
| `23932601d2` (#2949) | 2026-08-05 | `allowed-with-exclusive-explicit`, release/10.0 only |
| `7c89ba69b3` (unmerged) | 2026-08 | only prose rationale for `allowed` |

Investigation date: 2026-08-12. Behavioural claims cross-checked
against the workbench's recorded characterization
(`characterization/invalid-rule.md`, `no-silent-discount.md`).
