---
id: named_slates
title: Named Slates
sidebar_position: 13
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

A **slate** is a named group of candidates who run together across the contests of one Election, for example a candidate for President and three candidates for Trustees. Slates are configuration: they name candidates that are on the ballot anyway, and they are never a vote of their own. An Election without slates keeps its ordinary ballot.

---

## Configuration

Slates are stored as a JSON text in the Election annotation `sequent.slates`:

```json
{
  "version": 1,
  "mobile_candidate_lists": "collapsed",
  "slates": [
    {
      "id": "forward",
      "name": {"en": "Forward Together", "es": "Adelante Juntos"},
      "members": {
        "<president contest id>": ["<candidate id>"],
        "<trustees contest id>": ["<candidate id>", "<candidate id>", "<candidate id>"]
      }
    },
    {
      "id": "voices",
      "name": {"en": "Independent Voices"},
      "members": {
        "<trustees contest id>": ["<candidate id>", "<candidate id>"]
      }
    }
  ]
}
```

| Field | Meaning |
| --- | --- |
| `version` | Always `1`. |
| `mobile_candidate_lists` | `collapsed` (default) or `expanded`: whether phones show each slate's candidates before the voter asks. |
| `slates` | The slates, in the order voters see them. |
| `slates[].id` | An identifier of up to 64 letters, digits, hyphens or underscores, starting with a letter or digit. Unique in the Election. |
| `slates[].name` | The display name by language code. |
| `slates[].members` | The candidates of the slate, by contest. Both are the identifiers of existing contests and candidates of the Election. |

A slate does not need a candidate in every contest, and a candidate belongs to one slate at most. Candidates in no slate are independent.

### Display name

Each slate has a name per language. A name is one line of plain text of 1 to 120 characters, and can include a motto. Two slates cannot have the same name in the same language.

Voters see the name in their language. If the slate has no name in that language, they see the name in the Election's default language, and otherwise the first name configured. Give every slate a name in the default language of the Election.

### Validation

The configuration is refused when:

- it is not valid JSON, is larger than 64 KB, or has a `version` other than `1`;
- it has a field that is not listed above;
- a slate identifier is invalid or repeated;
- a slate has no name, an empty name, a name that is too long or not plain text, or a name another slate already uses in that language;
- a slate has no candidates, or lists a contest without candidates;
- a contest or candidate does not exist in the Election, or a candidate is not in the contest it is listed under;
- a candidate is listed twice, or in two slates.

The Voting Portal does not show a ballot whose slates are invalid. It reports a configuration error with the reason instead of leaving a slate out.

Slates are part of the published ballot. A change to the slates reaches voters with a new ballot publication.

The Election adds rules of its own, such as contest limits, and the configuration is checked again when it is saved, imported, generated and published. See [Slate Configuration](./14-slates-configuration.md).

---

## Voting Portal

When the Election has slates:

- The voting screen lists the slates in their configured order. Each slate shows its name and its candidates, grouped by contest.
- In every contest where a slate has candidates, each candidate shows the name of their slate under their own name. Candidates in no slate show **Independent**. Write-in, blank and invalid options show no label.
- The review screen shows the same slate name, or **Independent**, next to each selected candidate.

The label always comes from the candidate, so the ballot, the list of slates and the review screen show the same name.

---

## Complete and partial slates

A slate can name candidates in only some contests, for example only trustees, and fewer candidates than a contest has seats. Whether a slate is complete or partial is never configured: the platform derives it from the contests a voter can vote in, so a slate cannot claim an office it has no candidate for.

### How coverage is derived

- A slate is **complete** when it has a candidate for every seat (the contest's maximum number of choices) of every contest on the voter's ballot.
- Otherwise it is **partial**. A slate with one trustee out of three is partial even if it has a candidate in every office.
- Coverage is computed for each ballot style. Contests that are not on a voter's ballot are ignored, so the same slate can be complete for one area and partial for another.
- Acclaimed contests are displayed but not voted, so they count neither as covered nor as missing.
- Disabled candidates, write-in placeholders and the explicit blank or invalid options are not counted as members.
- A slate with no candidate that the voter can choose is not shown to that voter.

### What the voter sees

Under its name, each slate states what it covers:

| Text | Meaning |
| --- | --- |
| **Full slate** | A candidate for every seat of every contest. |
| **_Contest_ only**, for example **Trustees only** | Candidates in a single contest, with other contests on the ballot. |
| **Partial slate** | Any other partial slate. |

The text is followed by the number of candidates and of offices the slate has candidates in, for example `Trustees only · 3 candidates · 1 office`.

On wide screens every slate lists every contest of the ballot in ballot order, so that the same office is in the same position on each slate. An office the slate has no candidate for shows **No candidate**. On phones those offices are left out.

A partial slate never fills the offices it has no candidate for. They keep the voter's own choices, or stay empty.

---

## Slates and individual choices

When an election has slates, the voting screen has two tabs:

- **Choose a slate** shows the slates with their candidates by contest.
- **Individual candidates** is the ordinary ballot, contest by contest.

Both tabs work on the same ballot. A voter can choose a slate and then change it by hand: replace a candidate, deselect one, or add candidates of other slates and independent candidates, within each contest's limits. Slate members and independent candidates are selected with the same controls and count the same. See [Choosing a Slate](./13-choosing-a-slate.md) for what the **Choose this slate** button does.

Elections without slates show the ordinary ballot, without tabs.

### What Choose a slate shows

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

### Moving to the next step

**Next** works on both tabs. On a ballot whose contests are split in several pages, pressing **Next** on **Choose a slate** opens **Individual candidates**, so the voter goes through every page before the review screen.

### Slate cards on wider screens

On screens 750px wide or wider, the slates are shown side by side, one card per slate. Every candidate of every slate is visible without clicking.

The cards list the same contests in the same order: the contests of the voter's ballot that at least one slate has candidates for, in ballot order. Each contest starts at the same height in every card of a row, so a voter can read across the cards to compare the candidates for one office. The height of a contest follows the card that needs the most room, for example a slate with three trustees next to a slate with one, or a long name that takes two lines.

A slate without candidates for one of those contests shows the contest with **No candidate**. A contest no slate has candidates for is not listed in the cards; it is on the **Individual candidates** tab.

When there are more slates than fit in one row, the cards continue on the next row, and the contests line up within each row.

### Candidate lists on phones

On screens narrower than 750px, the slate cards are stacked one under the other and list only the contests their slate has candidates for. The candidates of each slate are in a list that the voter can show or hide. The `mobile_candidate_lists` field of the configuration decides how the lists start:

| `mobile_candidate_lists` | Lists start |
| --- | --- |
| `collapsed` (default) | Hidden, with a **Show candidates** button |
| `expanded` | Shown, with a **Hide candidates** button |

Showing or hiding a list never changes the ballot. The voter's choice for each list is kept while moving between the tabs and the review screen. The summary of the slate stays visible above a hidden list.

On wider screens the lists are always shown, whatever `mobile_candidate_lists` says.

### Screen sizes and keyboard

The slate cards, the individual candidates and the review screen fit the screen from 320px wide: long slate and candidate names continue on the next line and the page never scrolls sideways. On a phone the two tabs share the width of the screen.

The tabs, the **Show candidates** and **Hide candidates** buttons and the buttons of each slate are at least 44px tall. All of them can be reached with the Tab key and operated with Enter or Space, and show which one has the focus.

### Custom CSS

| Class | Element |
| --- | --- |
| `slate-coverage` | The line stating what the slate covers. Its `data-coverage` attribute is `complete` or `partial`. |
| `slate-coverage-kind` | **Full slate**, **_Contest_ only** or **Partial slate**. |
| `slate-coverage-count` | The number of candidates and offices. |
| `slate-contest-uncovered` | An office the slate has no candidate for. |
| `slate-no-candidate` | The **No candidate** text. |
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
