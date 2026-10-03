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

---

## Voting Portal

When the Election has slates:

- The voting screen lists the slates in their configured order. Each slate shows its name and its candidates, grouped by contest.
- In every contest where a slate has candidates, each candidate shows the name of their slate under their own name. Candidates in no slate show **Independent**. Write-in, blank and invalid options show no label.
- The review screen shows the same slate name, or **Independent**, next to each selected candidate.

The label always comes from the candidate, so the ballot, the list of slates and the review screen show the same name.
