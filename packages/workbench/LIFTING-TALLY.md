<!--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Tally visualization: velvet → `@sequentech/ui-essentials`

## Status: no longer a lift

This document used to describe a **copy lift** — admin-portal's tally
views re-hosted into `ui-essentials` as
`TallyResultsView` / `TallyResultsCharts` /
`TallyResultsCandidatesPlurality` / `TallyResultsCandidatesIRV`, with a
hardcoded English `strings.ts` shim standing in for i18n.

That lift is **gone**. Upstream built its own production tally
visualization in the same place (`ui-essentials/src/components/
TallyResults/`), shared with `results-portal`, and the workbench adopted
it wholesale when `main` was merged. The eight copy-lifted files were
deleted; nothing is copied or re-hosted any more.

What remains is a single adapter, and this document is now just its
mapping table plus the conventions that are easy to get wrong.

| | before (copy lift) | now |
|---|---|---|
| Component source | copied from admin-portal, adapted | upstream `ui-essentials`, unmodified |
| i18n | `strings.ts` shim | `labels?: Partial<ResultsAndParticipationLabels>` with English defaults |
| Drift risk | manual re-application on every refresh | none — same component production renders |
| Workbench-owned code | 8 files | 1 adapter |

Because the workbench now renders the **same component production
renders**, there is no refresh procedure and no canary list. If upstream
changes the props, the adapter fails to type-check — that is the whole
early-warning system.

---

## The adapter (`app/src/lib/velvetTallyAdapter.ts`)

`adaptVelvetContestResult(result, contestName)` maps velvet's serialised
`ContestResult` (see `workbench/velvet-core/src/result.rs`) onto a
`VelvetTallyView`, whose first four fields are passed straight to
`ResultsAndParticipation`.

### Participation summary

| velvet field | `ResultsParticipationSummary` | note |
|---|---|---|
| `census` | `eligibleCensus` | rename |
| `total_valid_votes` | `totalValidVotes` | rename |
| `total_invalid_votes` | `totalInvalidVotes` | rename |
| `total_blank_votes` | `blankVotes` | rename |
| — | `totalVotes` | derived: valid + invalid |
| — | `*Percent` fields | derived: value ÷ census, as a **fraction** |

The component leaves any field it is not given as `-`, so the richer
metrics upstream supports (auditable votes, explicit/implicit invalid and
blank splits) simply render as dashes — velvet does not emit them.

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
