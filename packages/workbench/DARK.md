<!--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Dark theme palette (implemented 2026-05-26, `800dcdbec5`)

> This began as the implementation plan and was executed the same day.
> The **palette tables below are the live reference** for the workbench's
> colour semantics; the step-by-step procedure is retained as history and
> its counts/details reflect the code as of implementation day.

## Goal

Permanently switch the workbench UI to a dark color scheme. No light/dark
toggle — just replace the current white-background look with dark surfaces
and light text. The target appearance matches Brave's "Auto Dark Mode for
Web Contents" applied to the current workbench (screenshots captured
2026-05-26).

## Scope

Only the **workbench's own UI**. The lifted voting-portal booth screens
(`BoothSpike.tsx`) already have their own `ThemeProvider` from
`@sequentech/ui-essentials` and are **out of scope** — do not touch them.

### Files to modify

| File | ~Color literals | Notes |
|------|-----------------|-------|
| `workbench/app/src/WorkbenchInspector.tsx` | ~98 | Largest file; sidebar, nav bar, all detail pages (snapshot, contest, ballot-style, voter, diagnostics) |
| `workbench/app/src/BallotPipeline.tsx` | ~20 | Stage sections, buttons, textareas, Section component |
| `workbench/app/src/ContestPolicyOverridesPanel.tsx` | ~18 | Policy table, bounds row, dropdowns |
| `workbench/app/src/TallyPage.tsx` | ~14 | Textareas, buttons, visualization wrapper |
| `workbench/app/src/main.tsx` | 2 | Root/body background |

**Total: ~152 hardcoded color values to replace.**

All styles are inline `React.CSSProperties` objects (no CSS files, no CSS
modules, no CSS custom properties). The replacement is a direct value swap
in each style object — no architectural changes required.

### Additional change: MUI dark theme for tally visualization

The tally output section renders MUI DataGrid and chart components from
`ui-essentials`. These inherit MUI's palette. Wrap the visualization area
in:

```tsx
import {createTheme, ThemeProvider} from "@mui/material/styles"

const darkTheme = createTheme({palette: {mode: "dark"}})

// Around the tally visualization output:
<ThemeProvider theme={darkTheme}>
  {/* TallyResultsView, DataGrid, charts */}
</ThemeProvider>
```

This is a one-liner addition in `TallyPage.tsx` (or wherever the
visualization renders). No other MUI theming is needed.

---

## Target palette

Neutral grays derived from Brave Auto Dark Mode screenshots. No blue or
purple tint — pure neutral values throughout.

### Surfaces

| Role | Hex | Usage |
|------|-----|-------|
| Page background | `#1e1e1e` | `<body>`, main content area, sidebar |
| Elevated surface / cards | `#2a2a2a` | Diagnostics cards, tally boxes, policy overrides panel |
| Textarea / code blocks | `#252525` | JSON textareas, monospace code areas |
| Input fields | `#303030` | Dropdowns, text inputs (policy overrides, bounds row) |
| Sidebar active item | `#383838` | Highlight band on selected sidebar entry |
| Secondary button fill | `#383838` | "Encode all", "Decrypt all", "Add ballot", "Load", "Save…" |

### Text

| Role | Hex | Usage |
|------|-----|-------|
| Primary text | `#e0e0e0` | Headings, body text, table cell values, stage headings |
| Secondary / muted | `#999` | Subtitle UUIDs, "· Contest", "· Voter", timestamps |
| Section labels (sidebar) | `#888` | "SNAPSHOTS", "ELECTIONS", "VOTERS" caps labels |

### Accents

| Role | Hex | Usage |
|------|-----|-------|
| Link / primary accent | `#5b9aff` | Nav links, entity links, ballot-style links |
| Primary button fill | `#2563eb` | "Recast in…", "Run tally", "Render output" |
| Primary button text | `#ffffff` | White text on blue buttons |
| Secondary button text | `#e0e0e0` | Light text on gray buttons |
| Secondary button border | `#555` | Subtle border on secondary buttons |
| Success / green | `#4ade80` | Checkmarks ("contest ✓ · keypair ✓") |
| Error | `#ef4444` | Error states (the dirty-snapshot asterisk was later switched to gold `#f0c200`, `8e81c76b9b`) |
| Warning / gold | `#f0c200` | Alert-level validation messages |
| Warning badge background | `#3d3000` | Dark amber bg for warning banners |
| Filled row accent | `#1f7a8c` | Teal left-border on filled pipeline rows |

### Borders & dividers

| Role | Hex | Usage |
|------|-----|-------|
| Subtle divider | `#3a3a3a` | Horizontal rules, table row borders, section separators, card borders |
| Input border | `#4a4a4a` | Textarea borders, dropdown borders, bounds row inputs |
| Input focus border | `#5b9aff` | Focused input ring (matches link color) |

---

## Implementation procedure

### Step 1: `main.tsx` — root background

```tsx
document.body.style.backgroundColor = "#1e1e1e"
document.body.style.color = "#e0e0e0"
```

Nav bar background set to `#1e1e1e` with `borderBottom: "1px solid #3a3a3a"`.

### Step 2: `WorkbenchInspector.tsx`

Bulk replace of ~98 color values. Key areas:

- **Nav bar**: background `#1e1e1e`, link color `#5b9aff`
- **Sidebar**: background inherited, labels `#888`, item text `#e0e0e0`, active bg `#383838`
- **Snapshot table**: header bg `#2a2a2a`, row border `#3a3a3a`, link color `#5b9aff`
- **Buttons**: secondary bg `#383838`, border `#555`, text `#e0e0e0`; primary bg `#2563eb`, text `white`
- **Detail pages**: card bg `#2a2a2a`, borders `#3a3a3a`, code blocks bg `#252525`
- **Text**: primary `#e0e0e0`, secondary `#999`, error `#ef4444`, success `#4ade80`

### Step 3: `BallotPipeline.tsx`

- Stage headings → `#e0e0e0`
- Textareas → bg `#252525`, border `#4a4a4a`, text `#e0e0e0`
- Row cards → bg `#2a2a2a`, left-border `#3a3a3a` (filled: `#1f7a8c`)
- Buttons → secondary style (`#383838` bg, `#555` border)
- Dividers → `#3a3a3a`

### Step 4: `ContestPolicyOverridesPanel.tsx`

- Panel background → `#2a2a2a`
- Input fields → bg `#303030`, border `#4a4a4a`, text `#e0e0e0`
- Labels → `#e0e0e0`, muted text → `#999`
- Borders → `#3a3a3a`

### Step 5: `TallyPage.tsx`

- Textareas → bg `#252525`, border `#4a4a4a`, text `#e0e0e0`
- Section headings → `#e0e0e0`
- Buttons → primary blue style
- MUI dark `ThemeProvider` wrapping tally visualization
- Error box → bg `#3a1c1c`, border/text `#ef4444`

### Step 6: Verify

- Load each page and compare against the Brave Auto Dark screenshots
- Check: contrast on all text (WCAG AA: 4.5:1 for body text)
- Check: active sidebar item visibility
- Check: form input readability (dropdowns, textareas, number inputs)
- Check: pipeline stage colors remain distinguishable
- Check: tally charts/grid render correctly with MUI dark palette
- Check: error/warning colors still pop against dark background

---

## What NOT to change

- `BoothSpike.tsx` — lifted voting-portal screens; they have their own
  MUI `ThemeProvider` and are visually separate from the workbench chrome.
  (`BoothLayout` later gained a light-reset wrapper — `607823ca78` —
  keeping the booth's production light appearance inside the dark chrome;
  that wrapper is workbench chrome, not a lifted-screen change.)
- Any files outside `workbench/app/src/`.
- No new dependencies needed (MUI's `createTheme` is already available).
- No CSS files to create — keep the inline-style-object pattern.
- No theme toggling infrastructure — this is a permanent one-way switch.

---

## Mapping reference: light → dark

Common substitutions applied across all files:

| Light value(s) | Dark replacement | Semantic role |
|----------------|-----------------|--------------|
| `#fff`, `#ffffff`, `white` (backgrounds) | `#1e1e1e` or remove (inherit) | Page/card bg |
| `#fff`, `#ffffff`, `white` (text on buttons) | `#ffffff` (keep) | Button text |
| `#f9f9f9`, `#f4f6f8`, `#fafafa` (surface) | `#2a2a2a` | Elevated surface |
| `#f4f4f4`, `#f4f9ff` (code/textarea bg) | `#252525` | Code bg |
| `#000`, `#111`, `#222`, `#333`, `#444` (text) | `#e0e0e0` | Primary text |
| `#555`, `#666` (text) | `#999` | Secondary text |
| `#888`, `#999`, `#aaa` (muted) | `#888` or `#999` | Muted/label |
| `#ddd`, `#ccc`, `#eee`, `#e0e0e0`, `#e4e4e4` (borders) | `#3a3a3a` | Dividers |
| `#d0d0d0` (input borders) | `#4a4a4a` | Input borders |
| `#dde9ff` (active sidebar bg) | `#383838` | Active item |
| `#0066cc`, `#1976d2`, `blue` (links) | `#5b9aff` | Links |
| `#2c7a2c`, `green` (success) | `#4ade80` | Success state |
| `#a33`, `#b00020`, `#b22222`, `red` (error) | `#ef4444` | Error state |
| `#1976d2`, `#1a73e8` (primary button) | `#2563eb` | Primary button |
| `#fffbe6` (warning badge bg) | `#3d3000` | Warning bg |

---

## Revert

The initial swap was one commit (`800dcdbec5`, purely cosmetic, no logic
changes), but **a plain `git revert 800dcdbec5` no longer applies
cleanly**: the themed files have been edited many times since (and the
dirty indicator recoloured, the booth light-reset wrapper added), so the
revert would conflict and would not restore a coherent light theme.
Reverting today means using the light→dark mapping table above in
reverse, file by file.
