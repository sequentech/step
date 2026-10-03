---
id: slate_and_individual_choices
title: Mixing Slates and Individual Choices
sidebar_position: 15
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

When an election has slates, the voting screen has two tabs:

- **Choose a slate** shows the slates with their candidates by contest.
- **Individual candidates** is the ordinary ballot, contest by contest.

Both tabs work on the same ballot. A voter can choose a slate and then change it by hand: replace a candidate, deselect one, or add candidates of other slates and independent candidates, within each contest's limits. Slate members and independent candidates are selected with the same controls and count the same. See [Choosing a Slate](./13-choosing-a-slate.md) for what the **Choose this slate** button does.

Elections without slates show the ordinary ballot, without tabs.

## What Choose a slate shows

The slate cards always reflect the current ballot, however the candidates were selected:

- Every selected member of a slate has a checkmark and a highlight. Screen readers announce it as selected.
- Each slate states how much of it is selected:

| Summary | Meaning |
| --- | --- |
| **All N selected** | In every contest the slate has candidates for, the selection is exactly the slate's candidates. |
| **Mixed · n of N selected** | Some of its candidates are selected, and one of its contests also has a candidate from outside the slate or is marked as an invalid vote. |
| **Partly selected · n of N** | Some of its candidates are selected and nothing else is selected in its contests. |
| No summary | None of its candidates is selected. |

N is the number of candidates of the slate in the voter's ballot. Only the contests a slate has candidates for are taken into account, so a slate with only trustee candidates reads **All N selected** when exactly its trustees are selected, whatever the other contests hold.

A voter who selects one trustee of each of three slates sees that trustee marked in each card, and each slate reads **Mixed** with its own count.

The marks and the summaries are computed from the ballot every time it changes. The Voting Portal does not remember which slate was chosen, so they stay correct after going to **Individual candidates** or to the review screen and coming back.

A slate that is fully selected offers **Edit selections** instead of **Choose this slate**. It opens the **Individual candidates** tab.

## Moving to the next step

**Next** works on both tabs. On a ballot whose contests are split in several pages, pressing **Next** on **Choose a slate** opens **Individual candidates**, so the voter goes through every page before the review screen.

## Candidate lists on phones

On screens narrower than 750px, the candidates of each slate are in a list that the voter can show or hide. The slate configuration decides how the lists start:

```json
{
  "version": 1,
  "mobile_candidate_lists": "collapsed",
  "slates": []
}
```

| `mobile_candidate_lists` | Lists start |
| --- | --- |
| `collapsed` (default) | Hidden, with a **Show candidates** button |
| `expanded` | Shown, with a **Hide candidates** button |

Showing or hiding a list never changes the ballot. The voter's choice for each list is kept while moving between the tabs and the review screen. The summary of the slate stays visible above a hidden list.

On wider screens the lists are always shown.

## Custom CSS

| Class | Element |
| --- | --- |
| `slate-ballot-tabs-container` | Container of the tabs and their panels |
| `slate-ballot-tabs` | The tab list |
| `slate-ballot-tab-slates`, `slate-ballot-tab-candidates` | The two tabs |
| `slate-ballot-panel`, `slate-ballot-panel-slates`, `slate-ballot-panel-candidates` | The panels |
| `slate-selection-status` | The summary of a slate |
| `slate-selection-status-all`, `slate-selection-status-mixed`, `slate-selection-status-partly` | The summary, by state |
| `slate-member-selected` | A selected member of a slate |
| `slate-member-check` | The checkmark of a selected member |
| `slate-candidate-list-toggle` | The button that shows or hides the candidates on phones |
| `slate-candidate-list` | The candidates of a slate |
| `slate-card-actions` | Container of the actions of a slate |
| `slate-edit-selections-button` | The **Edit selections** button |
