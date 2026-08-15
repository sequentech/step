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
claims (the accessor rename, the filter/checker/tally pre-images of
`7b0a1c71e8`, the release-branch location of #2949) were verified
directly against `git show`; meta#8235 and PR #2018 were read in full
(2026-08-14); doc and i18n quotes carry their file paths for
re-checking.

## 1. One dial, two policies

The enum answers two independent questions with a single value:

- what to do about **explicit** invalidity — the voter deliberately
  marks the ballot null (a protest / "voto nulo" act);
- what to do about **implicit** invalidity — the selections are
  invalid by rule (over-vote, below-`min_votes`), deliberately or not.

Responses to either kind sit on one severity ladder:

> silence → inline message → dismissible dialog → hard block

("Not permitted" is not a separate concept from signaling — a hard
block is the top of the ladder.) The four values on `origin/main` are
points in that 2-axis space (a fifth, release/10.0-only value exists —
§8):

| value | response to explicit invalid | response to implicit invalid |
|---|---|---|
| `allowed` | silence | **silence** (min-vote: since 2025-09, visible before; over-vote: silent all along — but counted rather than discarded before; §5) |
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

The full silence is recent — but its two families have different
histories, and the difference is the finding. Everything below is
verified against the pre-image of `7b0a1c71e8` ("🐞 Inconsistencies
in Voting Portal (#2018)", 2025-09-29, parent issue
sequentech/meta#8235; co-authored by Félix Robles and Eduardo Robles
Elvira) — `git show 7b0a1c71e8^:<path>` for the filter, the checker,
and the tally. (An earlier revision of this section checked only the
filter pre-image and wrongly claimed that an *over-voted* ballot
under `allowed` showed its inline error before 2025-09; only the
below-min ballot did. Corrected 2026-08-14.)

**Filter pre-image**
(`voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx`):
the booth's message filter **never consulted `invalid_vote_policy`**
— `invalid_errors` were filtered only by a "voter hasn't touched the
contest yet" heuristic, and the review screen showed whatever had
been emitted.

**Checker pre-image** (`sequent-core/src/ballot_codec/raw_ballot.rs`)
— this is where the families split:

- **min-vote**: the `selectedMin` error was pushed
  **unconditionally**, so under `allowed` a below-min ballot really
  did display its inline error ("Number of selected choices … is less
  than the minimum …") by review at the latest. The original min-vote
  posture was "no dialog interruptions, but the voter is informed".
- **over-vote**: the `selectedMax` **error emission itself was
  guarded** by `invalid_vote_policy != Some(ALLOWED)`, under the
  comment "for errors, we use only invalid_vote_policy. Overvote
  policy is going to be used only for alerts". Under explicitly-set
  `allowed` — the common case: the `ContestPresentation` constructor
  defaults the field to `Some(ALLOWED)` and the admin-portal form
  falls back to it (§6) — no error existed, so nothing showed *and
  nothing reached the tally*. Whether the voter saw anything was
  carried entirely by the *alert*, which every over-vote variant
  except `allowed` pushes.

**Tally pre-image**
(`velvet/src/pipes/do_tally/counting_algorithm/plurality_at_large.rs`
with `plaintext.rs::is_invalid` = flag ∨ errors): a ballot with no
error and no flag took the valid branch, which counts **every**
`selected >= 0` choice with no cap. Before this commit, an over-max
ballot under explicit `allowed` was therefore **counted fully valid —
all selections counted, including those past `max_votes`** (two
selections at `max_votes: 1` gave one vote to *each* candidate).

**What `7b0a1c71e8` actually did is a migration.** It removed the
over-vote emission guard — errors are now recorded unconditionally,
plausibly a deliberate tally-integrity fix, given that over-max
ballots had been fully counted — and reinstated the same "under
`allowed`, no implicit-error signal" rule at the display layer,
**generalized to all errors**, with two carve-outs (over-vote when
`over_vote_policy ≠ allowed`, blank when `blank_vote_policy =
not-allowed`) that reconstruct the old alert-driven visibility.
Min-vote — always emitted, always visible before, and with no policy
of its own to key a carve-out on — was swept into a rule written with
over-vote's semantics in mind. Post-commit, min-vote is behaviourally
identical to an over-vote-style rule whose policy is frozen at
`allowed`: the `allowed × *` over-max rows of
`../characterization/overvote-rule.md` are column-identical to the
below-min rows of `../characterization/minvote-rule.md`. Per family:

- **over-vote** (`over = allowed × invalid = allowed`): the *silence*
  is old and deliberate (pre-ticket guard + comment); the *discount*
  is new (recorded error → `is_invalid()` → discarded, where before
  every selection counted).
- **min-vote** (all four cells, S2's `marker_only` included): the
  *discount* is old (`selectedMin` always reached the tally); the
  *silence* is new, unrequested, and inverted previously-visible
  behaviour.

Both families end at the same cell — silent and discarded — each
having gained the opposite half from this one commit.

The suppression block itself:

- arrived inside a bug-fix commit about display inconsistencies, not
  a feature commit;
- has no unit test (none exist for `InvalidErrorsList` at all);
- has no documentation (no JSDoc on the enum, no README mention in
  voting-portal / ui-core / ui-essentials);
- carries two incidental defects recorded as Defects 3–4 in
  UPSTREAM_FINDINGS.md (a tautological dedup predicate; a stale
  `useMemo` dependency list).

**meta#8235, read in full 2026-08-14** (the ticket, its comments, PR
#2018's body and review threads — the complete written record): five
reported defects, **every one a warning that is missing or
inconsistently shown** — the ticket asks for *more* voter signal,
never less. Both of its configurations set `invalid_vote_policy:
warn`; the value `allowed` appears nowhere; "Expected Behavior" is
empty; there are no issue comments, and the PR body is only the
parent-issue link. The ticket therefore specifies neither the display
suppression nor the emission change; both are implementation
decisions made inside the fix. What no artifact anywhere states is
the **composition**: unconditional emission + display suppression +
`is_invalid()` at tally = silent discount. No ticket line, test,
comment, or doc connects the display decision to the tally
consequence, and the over-vote tally flip (fully counted → silently
discarded, for elections already configured `allowed`) is likewise
unmentioned. The archival trail is exhausted; the remaining intent
questions go to the commit's authors (§9).

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
of having the policy. (Had that deployment wanted a null option *as
well as* its over-vote blocking, it could have had both — the
over-vote rule's own policy blocks independently of
`invalid_vote_policy`. What it could not have added is a hard-blocked
below-minimum zone — that is the off-diagonal gap of §1, and it is
specific to the min-vote rule.)

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
`allowed-with-exclusive-explicit`: per its commit message, the null
marker and other selections become mutually exclusive, mirroring the
blank marker's clearing behaviour (the exact reducer mechanics are not
verifiable from this tree). Its doc line also states why the default
does *not* clear:

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
characterization gains a fifth policy value, and marker exclusivity
(`ballotSelectionsSlice.ts`) becomes policy-dependent for the invalid
marker — which today has **no** exclusivity (the mixed state forms; only
the blank marker clears). Both current directions are observed in
`characterization/dom-validate.md` (invalid `marker_plus` reachable;
blank `regular_then_marker` → `no (cleared)`), giving the fifth value a
full observed baseline to diff against.

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
- *Intent:* now split per family (§5; ticket read 2026-08-14). The
  **min-vote** silence — four of the five confirmed cells, S2
  included — is ten months old, introduced by a bug-fix commit whose
  ticket asked for *more* warnings, untested, undocumented, and it
  inverted previously-visible behaviour: the strongest defect-shaped
  evidence in this document. The **over-vote** silence has pre-ticket
  lineage (the emission guard and its comment) and reads as intended
  semantics; for that cell the consultation question is not "was this
  an accident" but "is this design acceptable" — noting that the same
  commit silently flipped its tally outcome from fully-counted to
  discarded. Where upstream *does* intend a surprising behaviour, it
  says so in writing — #2949's "intentionally supported ballot shape"
  line is the house style for documented intent — and no such
  language exists for implicit-invalid silence, for the min-vote
  generalization, or for the tally flip. Honest counterweights: the
  docs gloss "Submit without warning", the deliberate #2126 default
  choice, and the unmerged-branch preset prose; all three are
  consistent with intent but none distinguishes the message layer
  from the dialog layer or mentions the discount consequence.

With meta#8235 read (§5) the archival trail is exhausted; the
remaining intent questions are for the commit's authors (Félix
Robles, Eduardo Robles Elvira): (i) was extending the `allowed`
silence to `selectedMin` — a rule with no policy of its own and no
alert to fall back on — considered? (ii) was the over-vote tally
change (over-max ballots under explicit `allowed`: fully counted
before, silently discarded after) considered for elections already
configured that way?

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
| `7b0a1c71e8` (#2018, meta#8235) | 2025-09-29 | made error emission unconditional (over-vote was guarded before) and moved the `allowed` suppression to the display layer → min-vote newly silent; over-vote tally flipped from fully-counted to discarded |
| `ff6e47600d` (#2126) | 2025-10 | settled the default toward `allowed` |
| `23932601d2` (#2949) | 2026-08-05 | `allowed-with-exclusive-explicit`, release/10.0 only |
| `7c89ba69b3` (unmerged) | 2026-08 | only prose rationale for `allowed` |

Investigation date: 2026-08-12. Behavioural claims cross-checked
against the workbench's recorded characterization
(`characterization/invalid-rule.md`, `no-silent-discount.md`; the
booth-surface claims — inline visibility under `allowed`, marker
reachability — are since observed across every recorded
(configuration × vote-state) cell of all seven rules in
`characterization/dom-validate.md`, 229/229).

Updated 2026-08-14: meta#8235 and PR #2018's full written record read
via authenticated `gh`; the checker pre-image (`raw_ballot.rs`) and
tally pre-image (`plurality_at_large.rs`, `plaintext.rs`) of
`7b0a1c71e8` read via `git show`. §1's table row and §5 corrected
accordingly — the earlier claim that an over-voted ballot under
`allowed` showed an inline error before 2025-09 was wrong (only
min-vote did; over-vote was silent at the message layer all along and
its ballots were fully counted rather than discarded).
