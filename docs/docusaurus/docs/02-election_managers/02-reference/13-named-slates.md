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

Slates are set up in the Admin Portal, in the Election's **Ballot Design** and on a page for each slate. See [Slate Configuration](./14-slates-configuration.md). The Admin Portal stores them as a JSON text in the Election annotation `sequent.slates`, which is also what an exported Election Event contains:

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
| `slates[].alias` | Optional. A short name of up to 120 characters, on one line. |
| `slates[].description` | Optional. A description of up to 1000 characters. |
| `slates[].external_id` | Optional. An identifier from another system, of up to 255 characters, on one line. |
| `slates[].members` | The candidates of the slate, by contest. Both are the identifiers of existing contests and candidates of the Election. |

A slate does not need a candidate in every contest, and a candidate belongs to one slate at most. Candidates in no slate are independent.

### Display name

Each slate has a name per language. A name is one line of plain text of 1 to 120 characters, and can include a motto. Two slates cannot have the same name in the same language.

Voters see the name in their language. If the slate has no name in that language, they see the name in the Election's default language, and otherwise the first name configured. Give every slate a name in the default language of the Election. The **Name** field of a slate in the Admin Portal is the name in that language; names in other languages are kept as they are.

### Validation

The configuration is refused when:

- it is not valid JSON, is larger than 64 KB, or has a `version` other than `1`;
- it has a field that is not listed above, or an alias, description or external ID that is too long or, for the alias and external ID, more than one line;
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

When the Election has slates, the voting screen has two tabs, **Choose a slate** and **Individual candidates**. Both work on the same ballot. Elections without slates show the ordinary ballot, without tabs.

Above the tabs, the number of selected candidates out of the most the ballot allows is shown, for example **3 / 7 candidates selected**. It is hidden on phones.

In every contest where a slate has candidates, each candidate shows the name of their slate under their own name, on both tabs and on the review screen. Candidates in no slate show **Independent**. Write-in, blank and invalid options show no label. The label always comes from the candidate, so every screen shows the same name.

---

## Complete and partial slates

A slate can name candidates in only some contests, for example only trustees, and fewer candidates than a contest has seats. Whether a slate is complete or partial is never configured: the platform derives it from the contests a voter can vote in.

- A slate is **complete** when it has a candidate for every seat (the contest's maximum number of choices) of every contest on the voter's ballot.
- Otherwise it is **partial**. A slate with one trustee out of three is partial even if it has a candidate in every office.
- Coverage is computed for each ballot style. Contests that are not on a voter's ballot are ignored, so the same slate can be complete for one area and partial for another.
- Acclaimed contests count neither as covered nor as missing.
- Disabled candidates, write-in placeholders and the explicit blank or invalid options are not counted as members.
- A slate with no candidate that the voter can choose is not shown to that voter.

---

## Choose a slate

Each slate is a card, in the configured order. A card shows, from top to bottom:

- A chip stating what the slate covers: **Full slate**, **_Contest_ only** (for example **Trustees only**) when it has candidates in one contest only, or **Partial slate**.
- The slate name.
- A sentence: **Candidates for every office.**, or **Candidates for _offices_. Other offices are left blank.**
- The number of candidates and offices, for example **3 candidates · 1 office**.
- How much of the slate is selected (see below).
- Its candidates, office by office. An office the slate has no candidate for shows **No candidate**.
- The **Select slate** button. See [Choosing a Slate](./13-choosing-a-slate.md).

Under the cards a note says: *A slate selects its candidates and leaves offices without a slate candidate blank. You can change individual choices at any time.* Once something is selected, a strip under the note lists every office with its selected candidates, or **No selection**.

The cards list the contests of the voter's ballot that at least one slate has candidates for, in ballot order. Candidates are listed in the order the ballot shows them, following each contest's candidate order.

### What a slate says about the ballot

The cards always reflect the current ballot, however the candidates were selected. Every selected member has a checkmark and a highlight, and screen readers announce it as selected. Each slate states how much of it is selected, looking at the whole ballot:

| Summary | Meaning |
| --- | --- |
| **All N selected** | The ballot holds exactly the slate's candidates, and nothing in the offices it has no candidate for. |
| **Mixed · n of N selected** | Some of its candidates are selected, together with any other choice on the ballot, including an invalid vote mark. |
| **Partly selected · n of N** | Some of its candidates are selected and nothing else is. |
| No summary | None of its candidates is selected. |

N is the number of candidates of the slate in the voter's ballot. A slate with only trustee candidates reads **All N selected** only while the other offices are blank. A voter who selects one trustee of each of three slates sees each slate as **Mixed** with its own count.

A card whose slate is **All N selected** has a dark border and a check instead of the people icon, and offers **Edit selections** instead of **Select slate**. **Edit selections** opens **Individual candidates**. A card with only some of its candidates selected has a lighter border.

The summaries are computed from the ballot every time it changes. The Voting Portal does not remember which slate was chosen, so they stay correct after going to **Individual candidates** or to the review screen and coming back.

### Wider screens

On screens 750px wide or wider, the cards are side by side, up to three per row; more slates continue on the next row. Every candidate is visible without clicking. Each office starts at the same height in every card of a row, also when a name takes two lines, so a voter can read across the cards to compare the candidates for one office. The page never scrolls sideways.

### Phones

On screens narrower than 750px, the cards are stacked in the configured order and list only the offices their slate has candidates for. The candidates of each slate are in a list the voter can show or hide with **Show candidates** and **Hide candidates**. The **Mobile candidate lists** setting of the Election decides how the lists start:

| Mobile candidate lists | Lists start |
| --- | --- |
| **Collapsed** (default) | Hidden |
| **Expanded** | Shown |

Showing or hiding a list never changes the ballot, and the voter's choice for each list is kept while moving between the tabs and the review screen. The summary of a slate stays visible above a hidden list.

---

## Individual candidates

On a ballot with slates, **Individual candidates** shows one office at a time. A line at the top states what the ballot holds: the name of the slate it holds exactly, **Individual selections**, or **No candidates selected**, followed by the number of selected candidates.

- On wider screens a list of the offices is on the left, each with its count, for example **1/1**, and a check when it is full. On phones the list is an **Office** drop-down.
- Each office shows **Office n of N**, its name, **Select up to N candidates. k selected.**, and a **Clear** button that removes the choices of that office. **Clear** is disabled while the office has no choice.
- **Previous office** and **Next office** move between offices; the last office says **Last office**.

All offices stay on the voting screen, so **Next** at the bottom goes to the review screen from either tab. Ballots without slates keep their usual pages.

---

## Reviewing the ballot

When the Election has slates, the review screen lists every contest as usual and adds:

- Under each contest, the number of selected candidates out of the maximum, for example **2 of 3 selected**. A contest without a selection also reads **No candidate selected**.
- Under each contest, an edit button named **Edit _contest_**. It opens **Individual candidates** at that office.
- **Your selections**, next to the contests on wider screens: the number of selected candidates out of the most the ballot allows, the name of the slate the ballot holds exactly or **Individual selections** or **No candidates selected**, and the note *Your vote is recorded for each selected candidate.* It is not shown on phones.

Going back with an edit button or **Back** keeps every selection. Acclaimed contests have no count and no edit button, and a declined ballot shows neither the summary nor the counts.

The review screen always shows the selections of the encrypted ballot it is about to cast. If the selections change after the ballot was encrypted, for example with the back and forward buttons of the browser, the Voting Portal returns to the voting screen with the current selections.

### Unfilled offices

When a contest's **Under Vote Policy** is **Warn and Confirm in Review**, casting a ballot with fewer choices than allowed in that contest opens **Some selections are unfilled**. It lists each such office with, for example, **0 of 1 selected · No vote for this office**. **Review selections** closes it without casting; **Continue with these selections** casts. An entirely blank ballot is asked the same way. The answer is not kept: the question comes again on every cast.

---

## Screen sizes and keyboard

The slate cards, the individual candidates, the dialogs and the review screen fit the screen from 320px wide: long slate and candidate names continue on the next line and the page never scrolls sideways.

The tabs, the **Show candidates** and **Hide candidates** buttons and the buttons of each slate are at least 44px tall. All of them can be reached with the Tab key and operated with Enter or Space, and show which one has the focus.

---

## Custom CSS

| Class | Element |
| --- | --- |
| `slate-selection-count` | The number of selected candidates above the tabs and on the review screen |
| `slate-ballot-tabs-container` | Container of the tabs and their panels |
| `slate-ballot-tabs-bar`, `slate-ballot-tabs` | The tab bar and the tab list |
| `slate-ballot-tab-slates`, `slate-ballot-tab-candidates` | The two tabs |
| `slate-ballot-panel`, `slate-ballot-panel-slates`, `slate-ballot-panel-candidates` | The panels |
| `slate-chooser`, `slate-list` | The slates and their list |
| `slate-card` | A slate card |
| `slate-card-none`, `slate-card-some`, `slate-card-exact` | A card with none, some or exactly its candidates selected |
| `slate-card-header`, `slate-symbol`, `slate-mark` | The top of a card, its icon row and the round icon |
| `slate-coverage-chip` | **Full slate**, **_Contest_ only** or **Partial slate**. Its `data-coverage` attribute is `complete` or `partial`. |
| `slate-name` | The slate name |
| `slate-coverage-sentence` | **Candidates for every office.** or the offices the slate covers |
| `slate-count` | The number of candidates and offices |
| `slate-summary`, `slate-selection-status` | The summary of a slate |
| `slate-selection-status-all`, `slate-selection-status-mixed`, `slate-selection-status-partly` | The summary, by state |
| `slate-member-lists`, `slate-candidate-list` | The offices of a card |
| `slate-candidate-list-toggle` | **Show candidates** / **Hide candidates** on phones |
| `slate-contest`, `slate-contest-name` | An office of a card and its name |
| `slate-contest-uncovered` | An office the slate has no candidate for |
| `slate-no-candidate` | The **No candidate** text |
| `slate-members`, `slate-member`, `slate-member-name` | The candidates of an office |
| `slate-member-selected`, `slate-member-check` | A selected member and its checkmark |
| `slate-actions`, `slate-card-actions` | The actions of a card |
| `slate-edit-selections-button` | The **Edit selections** button |
| `slate-note` | The note under the cards |
| `slate-all-selections`, `slate-all-selections-office`, `slate-all-selections-name` | The strip of current selections |
| `slate-selection-label` | The line at the top of **Individual candidates** |
| `slate-office-navigator` | The office navigator |
| `slate-office-list`, `slate-office-button`, `slate-office-count`, `slate-office-full` | The list of offices, an office, its count and its check |
| `slate-office-picker`, `slate-office-select` | The **Office** drop-down on phones |
| `slate-office-index` | **Office n of N** |
| `slate-office-navigation`, `slate-office-previous`, `slate-office-next`, `slate-office-last` | **Previous office**, **Next office** and **Last office** |
| `contest-selection-count` | **Select up to N candidates. k selected.** |
| `contest-clear-button`, `contest-clear-status` | The **Clear** button of an office and its announcement |
| `review-slate-layout`, `review-slate-contests`, `review-slate-aside` | The review screen with **Your selections** next to the contests |
| `review-selection-summary` | **Your selections** |
| `review-selection-title`, `review-selection-total`, `review-selection-caption`, `review-selection-note` | Its heading, total, caption and note |
| `review-selection-label`, `review-selection-label-slate`, `review-selection-label-individual`, `review-selection-label-none` | What the ballot holds, by kind |
| `review-contest-footer` | The count and edit button of a contest |
| `review-contest-unfilled` | A contest with fewer choices than allowed |
| `review-contest-count`, `review-contest-blank`, `review-contest-edit` | The count, **No candidate selected** and the edit button |
| `unfilled-contests-dialog`, `unfilled-contest` | **Some selections are unfilled** and one of its offices |
