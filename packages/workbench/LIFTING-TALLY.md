<!--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Tally visualization: velvet → `@sequentech/ui-essentials`

## Status

The tally visualization is the upstream `ui-essentials` component
(`src/components/TallyResults/`, shared with `results-portal`),
imported unmodified; nothing is copied or re-hosted. The workbench's
only code is the thin adapter below, mapping velvet's output onto the
component's props — if upstream changes them, the adapter fails to
type-check, which is the whole early-warning system.

---

## The adapter (`app/src/lib/velvetTallyAdapter.ts`)

`adaptVelvetContestResult(result, contestName)` maps velvet's serialised
`ContestResult` (see `workbench/velvet-core/src/result.rs`) onto a
`VelvetTallyView`. `TallyPage` passes its `chartName`, `summary` and
`candidates` straight to `ResultsAndParticipation` (with a literal
`preferential={false}` — see adaptation 2 below) and mounts
`PreferentialCandidateResults` on `processResults`.

### Participation summary

| velvet field | `ResultsParticipationSummary` | note |
|---|---|---|
| `census` | `eligibleCensus` | rename |
| `auditable_votes` | `totalAuditableVotes` | rename |
| `total_votes` | `totalVotes` | falls back to valid + invalid when absent |
| `total_valid_votes` | `totalValidVotes` | rename |
| `total_invalid_votes` | `totalInvalidVotes` | rename |
| `invalid_votes.explicit` | `explicitInvalidVotes` | rename |
| `invalid_votes.implicit` | `implicitInvalidVotes` | rename |
| `total_blank_votes` | `blankVotes` | rename |
| `blank_votes.explicit` | `explicitBlankVotes` | rename |
| `blank_votes.implicit` | `implicitBlankVotes` | rename |
| `percentage_*` | the matching `*Percent` field | **÷ 100** — see below |

**Percentages are forwarded, not recomputed.** velvet emits every
percentage itself, and the bases differ per row: turnout rows
(`percentage_total_votes`, `percentage_auditable_votes`) are over census,
while valid / invalid / blank are over votes cast. An earlier version of
this adapter derived them all as `value ÷ census`, which silently
disagreed with production on any contest where those bases differ.
Forwarding velvet's own numbers is what makes the workbench a fidelity
check rather than a second opinion.

Fields velvet does not emit are left `undefined` so the component renders
`-` rather than a misleading `0.00%`.

### Candidates

| velvet field | `CandidateResultRow` | note |
|---|---|---|
| `candidate_result[].candidate.id` | `id` | direct |
| `candidate_result[].candidate.name` | `name` | falls back to `id` |
| `candidate_result[].total_count` | `castVotes` | direct |
| `candidate_result[].percentage_votes` | `castVotesPercent` | **÷ 100** — see below |
| — | `winningPosition` | assigned by the adapter, 1..n by `castVotes` desc |

`winningPosition` has no velvet counterpart. `sortCandidateResults`
*consumes* it but never computes it, so the adapter must.

### Preferential (IRV)

`process_results` → `PreferentialProcessResults` is close to a
pass-through: velvet already emits `candidates_status`,
`name_references`, `round_count`, `max_rounds` and a `rounds[]` whose
entries carry `winner`, `candidates_wins`, `eliminated_candidates`,
`active_candidates_count`, `active_ballots_count` and
`exhausted_ballots_count`. The adapter only normalises nulls and fills
candidate names. Velvet's newer `pending_tie_resolution`,
`tie_breaking_policy` and `tie_resolutions` are currently ignored.

### The two percentage conventions

This is the one thing that silently produces nonsense, so it is worth
stating twice:

- **Plurality** — velvet's `percentage_votes` is in `[0, 100]`.
  `percentOrDash` runs it through `formatPercentOne`, which multiplies by
  100. So the adapter **divides by 100**. Skip that and you get
  "10,000.00%".
- **IRV rounds** — velvet's `rounds[].candidates_wins[].percentage` is
  already a `[0, 1]` fraction
  (`velvet-core/src/counting/instant_runoff.rs`, `.clamp(0.0, 1.0)`), and
  `PreferentialCandidateResults` renders
  `(outcome.percentage * 100).toFixed(2)`. So it **passes through
  unchanged**. Dividing here would be wrong.

The asymmetry is a quirk of velvet's result shape, not of the components.

---

## Two workbench-side adaptations in `TallyPage.tsx`

Both exist because `ResultsAndParticipation` is built for production
pages, not for a dark inspector sandbox. Neither modifies upstream.

1. **Counting algorithm / winners line.** The component has no slot for
   `counting_algorithm` or `winning_candidates_num`, which the workbench
   wants visible. `TallyPage` renders them itself above the
   visualization; the adapter carries them on `VelvetTallyView` as
   `countingAlgorithm` and `winnersCount`.

2. **A light theme around the IRV table.** `preferential` is left
   `false` so the candidate table always renders, and
   `PreferentialCandidateResults` is mounted separately underneath when
   velvet emitted rounds — the sandbox shows both views rather than
   either/or. That component hardcodes light cell backgrounds
   (`#FBFBFB`, `#fff`, `#F9F9FF`) because results-portal is a light page;
   under the workbench's dark MUI theme its text would be painted white
   onto those cells and the candidate column would vanish. It therefore
   gets its own `muiLightTheme` — a light island in dark chrome, but
   legible and pixel-identical to production. Fixing this properly means
   making the upstream component theme-aware, which is a production
   change and out of scope here.

---

## Verifying

Load `bundled:mixed-3contests` and tally each contest:

- Mayor — plurality, 3 candidates, 1 winner (`…00c1`)
- City council — plurality-at-large, 4 candidates, 2 winners (`…00c2`)
- Park funding — instant-runoff, 3 candidates, 1 winner (`…00c3`)

Expect a participation pie plus summary table, a candidates pie with an
"Others" rollup past 5 candidates, a `Options / Number of Votes / Percent
of Votes / Winning position` table, and — for the IRV contest — a
round-by-round table with a green `Winner` chip and red `Eliminated`
chips.

`bundled:instant-runoff-3cand` is the quickest IRV check. With five
ranked ballots split 2/2/1 it should eliminate the trailing candidate in
round 1 and transfer, giving `3 (60.00%)` / `2 (40.00%)` in round 2.
