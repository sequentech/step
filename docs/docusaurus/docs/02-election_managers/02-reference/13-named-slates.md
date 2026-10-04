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

### Custom CSS

| Class | Element |
| --- | --- |
| `slate-coverage` | The line stating what the slate covers. Its `data-coverage` attribute is `complete` or `partial`. |
| `slate-coverage-kind` | **Full slate**, **_Contest_ only** or **Partial slate**. |
| `slate-coverage-count` | The number of candidates and offices. |
| `slate-contest-uncovered` | An office the slate has no candidate for. |
| `slate-no-candidate` | The **No candidate** text. |
