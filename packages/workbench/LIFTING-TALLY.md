<!--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Lifting recipe: admin-portal tally visualization into `@sequentech/ui-essentials`

## Why this document exists

The workbench inspector needs to render tally results for the contests it
runs through `velvet-core`. Production already does this — `admin-portal`
shows pie charts, results tables, and IRV round-by-round breakdowns —
but its source lives behind a craco/CRA build and is coupled to a
GraphQL schema (`Sequent_Backend_*` types) that the workbench does not
speak.

We **lifted the rendering layer into `@sequentech/ui-essentials`** as a
permanent feature, so:

- the workbench (Vite, React 19) can import the same components,
- the admin-portal can continue to use its own copy without touching this
  one (until/unless we migrate it to consume `ui-essentials` too),
- a thin `velvetTallyAdapter.ts` in the workbench maps velvet's snake_case
  `ContestResult` to the plain TS shape the components expect.

This is the **admin-portal counterpart** to [LIFTING.md](LIFTING.md), which
covers the voting-portal lift. The scope here is much narrower:

| | voting-portal lift | admin-portal tally lift |
|---|---|---|
| Source location | `packages/voting-portal/src/` (untouched, lifted *in place*) | `packages/admin-portal/src/resources/Tally/` (re-hosted into `ui-essentials`) |
| Build harness substitution | Vite replaces craco | Webpack (ui-essentials) replaces craco |
| Re-host strategy | Vite alias to portal sources | Copy-with-adaptations into `ui-essentials/src/components/TallyResults/` |
| Surface area | ~hundreds of files | 5 source files |
| Drift risk | High (lots of upstream churn) | Low (admin-portal tally views are stable) |

Like `LIFTING.md`, fidelity here is *manual*. Admin-portal still owns the
canonical version. When it evolves, we have to re-apply the adaptations
documented below.

---

## Inventory of adaptations

Each adaptation has:

- **What** — the change.
- **Where** — the file(s) in `packages/ui-essentials/src/components/TallyResults/`.
- **Why** — what admin-portal behaviour it substitutes for.
- **Canary** — the symptom that would prove admin-portal evolved past it.

### L. i18n removed

| # | Adaptation | Why | Canary |
|---|------------|-----|--------|
| L1 | `strings.ts` — a hardcoded `t(key)` shim returning English strings, in place of `useTranslation()` from `react-i18next`. | Per explicit user direction: *"we do not need to replicate any i18n concerns, we're ok with having fixed language strings"*. Adding `react-i18next` would force every workbench/`ui-essentials` consumer to ship an `<I18nextProvider>` and translation bundles. | If admin-portal adds a new translated string the lifted component references, it will appear as the raw key (e.g. `"tally.chart.someNewLabel"`). Fix: add the key to `strings.ts` with the English value from `admin-portal/src/locales/en.ts`. |

The keys currently shimmed (verbatim values copied from
`admin-portal/src/locales/en.ts`):

```
tally.chart.votesForCandidates  = "Votes For Candidates"
tally.chart.blankVotes          = "Blank Votes"
tally.chart.invalidVotes        = "Invalid Votes"
tally.chart.nonVoters           = "Non-Voters"
tally.table.candidates          = "Candidates"
tally.table.options             = "Options"
tally.table.cast_votes          = "Number of Votes"
tally.table.cast_votes_percent  = "Percent of Votes"
tally.table.winning_position    = "Winning position"
tally.table.preferential.candidate    = "Candidate"
tally.table.preferential.round        = "Round"
tally.table.preferential.winner       = "Winner"
tally.table.preferential.eliminated   = "Eliminated"
```

### T. Types decoupled from GraphQL

| # | Adaptation | Why | Canary |
|---|------------|-----|--------|
| T1 | Dropped `Sequent_Backend_Candidate_Extended` (admin-portal's generated GraphQL type) and inlined a plain `TallyCandidate` interface in `types.ts`. | The workbench has no GraphQL backend. Generated types would force ui-essentials to depend on admin-portal's codegen output. | If a new field appears on `Sequent_Backend_Candidate_Extended` that the lifted components consume, add it to `TallyCandidate` and to `velvetTallyAdapter.ts`. |
| T2 | Dropped `ExtendedMetricsContest` / `ParsedAnnotations`; replaced with `TallyParticipationSummary` (a 4-field plain object). | Admin-portal threads metrics annotations through MobX/Apollo state. We don't need any of that. | If `ParticipationSummaryChart` starts referencing an additional metric, plumb it through `TallyParticipationSummary` and update the adapter. |

### U. Utilities slimmed

| # | Adaptation | Why | Canary |
|---|------------|-----|--------|
| U1 | Kept only `winningPositionComparator` from admin-portal's `Tally/utils.ts`. Dropped `convertGraphQLCandidate` / `convertGraphQLContest`. | Those converters target the GraphQL types we removed in T1/T2. | n/a — our adapter does the equivalent job for velvet. |
| U2 | Dropped `parseProcessResults`. | Admin-portal parses `processResults` from a JSON string field. Velvet emits `process_results` as already-parsed `serde_json::Value`. | If admin-portal changes the structure inside `process_results`, mirror the change in our `RunoffStatus` type and in `velvetTallyAdapter.adaptRunoff`. |

### C. `TallyResultsCharts.tsx` adaptations

| # | Adaptation | Why | Canary |
|---|------------|-----|--------|
| C1 | `useTranslation()` → `t` shim from `./strings`. | See L1. | See L1. |
| C2 | `ParticipationSummaryChart` props: takes a plain `TallyParticipationSummary` instead of an `ExtendedMetricsContest`. | See T2. | If admin-portal renames the participation fields, update both `TallyParticipationSummary` and the adapter. |
| C3 | `CandidatesResultsCharts` prop rename: `results` → `candidates`, and type changed to `TallyCandidate[]`. | The original prop name `results` was ambiguous; we already have a different `result` symbol in adapter call-sites. The rename keeps the intent obvious at use sites. | If you re-sync from admin-portal and copy the prop signature verbatim, restore the `results` name *only inside the component* and adjust call sites — but a rename is preferred. |
| C4 | `import type {Props} from "react-apexcharts"` → `import type {ApexOptions} from "apexcharts"`. | `react-apexcharts` ≥ 1.7 dropped the named `Props` export. The workbench resolves `react-apexcharts` at 1.9.0 (`^1.7.0`). | If a `yarn install` downgrades `react-apexcharts` below 1.7, the old import would work again — but we should stay on the modern import either way. |
| C5 | Replaced `<CardChart title={...}>...</CardChart>` (an admin-portal-local presentational wrapper) with a plain `<Box><Typography variant="subtitle1">{chartName}</Typography><Chart .../></Box>`. | `CardChart` is in `admin-portal/src/components/`, not in `ui-essentials`. Lifting it too would balloon scope. The replacement is visually equivalent at workbench fidelity. | If admin-portal's `CardChart` grows new behaviour (e.g. download / fullscreen buttons), we'd need to either lift it or recreate the behaviour inline. |

### P. `TallyResultsCandidatesPlurality.tsx` adaptations

| # | Adaptation | Why | Canary |
|---|------------|-----|--------|
| P1 | `useTranslation()` → `t` shim. | See L1. | See L1. |
| P2 | `<DataGrid>` rows typed `TallyCandidate[]` instead of `Sequent_Backend_Candidate_Extended[]`. Columns wired to the plain field names (`name`, `cast_votes`, `cast_votes_percent`, `winning_position`). | See T1. | If admin-portal adds a column, mirror it here and in the adapter. |
| P3 | Empty-state `<NoItem>` (admin-portal component) → `<Typography variant="body2">No data available.</Typography>`. | `NoItem` lives in admin-portal. Same scope-control reasoning as C5. | If we want richer empty-state styling, consider lifting `NoItem` too. |

`@mui/x-data-grid` is **kept** (rather than substituting a hand-rolled MUI
`<Table>`) per explicit user direction: *"we prefer option 1, easier
maintenance is more important than the hundreds of KB"*. The workbench pays
the DataGrid bundle cost in exchange for visual fidelity with admin-portal
and zero rewrite work on every refresh.

### I. `TallyResultsCandidatesIRV.tsx` adaptations

| # | Adaptation | Why | Canary |
|---|------------|-----|--------|
| I1 | `useTranslation()` → `t` shim. | See L1. | See L1. |
| I2 | Removed `useRef` and an unused `ECandidateStatus` import; removed an unused destructure of `candidates_status` from `process_results`. | These were dead in the lifted slice. | If admin-portal starts using either, restore. |

The round-window navigation (3/2/1 visible columns at xl/lg/below
breakpoints, arrow `IconButton`s, keyboard handlers for ←/→/Home/End,
sticky candidate column, `Winner`/`Eliminated` `Chip`s) is preserved
**verbatim**.

### V. `TallyResultsView.tsx` is workbench-flavoured

| # | Adaptation | Why | Canary |
|---|------------|-----|--------|
| V1 | New file: high-level wrapper composing `ParticipationSummaryChart` + a summary `<Typography>` (Census / valid / blank / invalid + counting algorithm / winners) + `TallyResultsCandidatesPlurality` + conditional `TallyResultsCandidatesIRV`. Not present in admin-portal. | Admin-portal composes these inside a much larger React tree (Resource pages, MobX stores). We need a single drop-in for the workbench inspector. | On a refresh, this file generally won't need changes — its job is to be the workbench's composition root for the lifted components. Update it if you want to surface new state, e.g. tie counts. |

---

## Dependency tracking

`packages/ui-essentials/package.json` gained the following because of this
lift:

### `dependencies`

- `apexcharts: ^4.4.0` — admin-portal currently pins `^3.41`. The workbench
  lift uses 4.x because that's the modern major and the `ApexOptions`
  type shape we use is stable across both. If admin-portal upgrades to 4.x,
  no action required. If it stays on 3.x and a 3.x-only API appears in
  the lifted components, pin ui-essentials to 3.x as well.
- `react-apexcharts: ^1.7.0` (resolves to 1.9.0). Admin-portal pins
  `^1.4.x`. The `Props` → `ApexOptions` change (C4) is mandatory at 1.7+.

### `peerDependencies`

- `@mui/icons-material: 7.3.2` — used by `TallyResultsCandidatesIRV`
  (ChevronLeft/Right).
- `@mui/x-data-grid: 8.17.0` — used by `TallyResultsCandidatesPlurality`.
  Matches the admin-portal pin to keep API drift to zero.

The workbench app (`packages/workbench/app/package.json`) lost
`apexcharts` and `react-apexcharts` (the spike used them directly; now
they are an ui-essentials concern) and gained `@mui/icons-material` and
`@mui/x-data-grid` to satisfy the new peer requirements.

---

## Adapter inventory (`packages/workbench/app/src/lib/velvetTallyAdapter.ts`)

This is the workbench-side glue. It owns the field-name and unit
conventions between velvet's `ContestResult` and the ui-essentials
component props.

| velvet field | ui-essentials field | Adaptation |
|---|---|---|
| `census: u64` | `TallyParticipationSummary.census` | direct |
| `total_valid_votes: u64` | `.totalValidVotes` | rename |
| `total_invalid_votes: u64` | `.totalInvalidVotes` | rename |
| `total_blank_votes: u64` | `.totalBlankVotes` | rename |
| `candidate_result[].candidate.id` | `TallyCandidate.id` | direct |
| `candidate_result[].candidate.name` | `TallyCandidate.name` | falls back to `id` if absent |
| `candidate_result[].total_count: u64` | `TallyCandidate.cast_votes` | direct |
| `candidate_result[].percentage_votes: f64` (`[0, 100]`) | `TallyCandidate.cast_votes_percent` (`[0, 1]`) | **divide by 100** — `formatPercentOne` multiplies by 100, so passing velvet's percentage directly produces "10,000.00%". This is verified empirically. |
| n/a | `TallyCandidate.status` | adapter sets `"active"` for all candidates (required string field on the type). |
| n/a | `TallyCandidate.winning_position` | adapter sorts by `cast_votes` desc and assigns 1..n. |
| `process_results: Option<Value>` | `RunoffStatus` | `adaptRunoff` extracts `candidates_wins` / `last_round` if present. |

**IRV round percentage convention.** Velvet emits
`process_results.candidates_wins[].rounds[].percentage` as a
**[0, 1] fraction** (see `velvet-core/src/counting/instant_runoff.rs:560`
— `outcome.percentage = ((wins as f64) / act_ballots_f64).clamp(0.0, 1.0)`).
The lifted IRV component multiplies by 100 at display time, so the
adapter passes the value through unchanged. **Do not divide by 100** for
this path — that would only be needed for the plurality
`percentage_votes` path above. (The asymmetry is a quirk of velvet's
result shape, not the components.)

---

## Refresh procedure

When admin-portal's tally views evolve, refresh as follows.

1. **Diff the upstream sources.**

   ```
   git diff <prev-known-good> -- packages/admin-portal/src/resources/Tally/
   ```

   Focus on:
   - `Tally/types.ts` (interfaces consumed by chart/table components)
   - `Tally/utils.ts` (comparators)
   - `TallyResultsCharts/` (chart components + `CardChart` wrapper)
   - `TallyResultsCandidatesPlurality.tsx`
   - `TallyResultsCandidatesIRV.tsx`
   - `admin-portal/src/locales/en.ts` keys under `tally.*`

2. **Re-apply this recipe.** Each section above is a checklist:
   - L1: copy any new `tally.*` keys into `strings.ts`.
   - T1/T2: extend `TallyCandidate` / `TallyParticipationSummary` for any new fields.
   - U1/U2: keep only the comparators; never copy back GraphQL converters.
   - C1–C5: re-apply per-component substitutions when copying chart code.
   - P1–P3: same for plurality.
   - I1–I2: same for IRV.

3. **Update the adapter.** Any new field on a lifted component prop must
   be filled in by `velvetTallyAdapter.ts`. Watch the units (especially the
   `percentage_votes` ÷ 100 dance documented above).

4. **Verify the three contest types in the workbench.** Load
   `bundled:mixed-3contests`, run the tally for:
   - Mayor (plurality, 3 candidates, 1 winner) — `…00c1`
   - City council (plurality-at-large, 4 candidates, 2 winners) — `…00c2`
   - Park-funding (instant-runoff, 3 candidates, 1 winner) — `…00c3`

   Each should show: participation pie (100% for a single ballot),
   summary line with census/valid/blank/invalid and counting algorithm,
   candidates pie chart with proper "Others" rollup at >5 candidates,
   DataGrid with `Options / Number of Votes / Percent of Votes /
   Winning position` columns, and (IRV only) the round-by-round table
   with a green `Winner` chip.

5. **Run `get_errors`** on every file under
   `packages/ui-essentials/src/components/TallyResults/` plus
   `packages/workbench/app/src/lib/velvetTallyAdapter.ts` and
   `packages/workbench/app/src/WorkbenchInspector.tsx`.

---

## File inventory (where the lift lives)

```
packages/ui-essentials/src/components/TallyResults/
├── index.ts                                 # re-exports
├── strings.ts                               # i18n shim (L1)
├── types.ts                                 # T1, T2
├── utils.ts                                 # U1, U2
├── TallyResultsCharts.tsx                   # C1–C5
├── TallyResultsCandidatesPlurality.tsx      # P1–P3
├── TallyResultsCandidatesIRV.tsx            # I1, I2
└── TallyResultsView.tsx                     # V1 (workbench composition root)

packages/workbench/app/src/
├── lib/velvetTallyAdapter.ts                # velvet ↔ ui-essentials glue
└── WorkbenchInspector.tsx                   # call site (single render block)
```

`packages/ui-essentials/src/index.tsx` re-exports the 13 public symbols
(`TallyResultsView`, `ParticipationSummaryChart`, `CandidatesResultsCharts`,
`TallyResultsCandidatesPlurality`, `TallyResultsCandidatesIRV`,
`winningPositionComparator`, plus the types `TallyCandidate`,
`TallyParticipationSummary`, `TallyResultsViewModel`, `CandidateOutcome`,
`CandidatesOutcomes`, `Round`, `RunoffStatus`).
