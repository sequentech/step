---
id: complete_and_partial_slates
title: Complete and Partial Slates
sidebar_position: 14
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

A slate does not need a candidate for every office. It can name candidates in only some contests, for example only trustees, and it can name fewer candidates than a contest has seats. Both are valid configurations.

Whether a slate is complete or partial is never configured. The platform derives it from the contests a voter can vote in, so a slate cannot claim an office it has no candidate for.

## How coverage is derived

- A slate is **complete** when it has a candidate for every seat (the contest's maximum number of choices) of every contest on the voter's ballot.
- Otherwise it is **partial**. A slate with one trustee out of three is partial even if it has a candidate in every office.
- Coverage is computed for each ballot style. Contests that are not on a voter's ballot are ignored, so the same slate can be complete for one area and partial for another.
- Acclaimed contests are displayed but not voted, so they count neither as covered nor as missing.
- Disabled candidates, write-in placeholders and the explicit blank or invalid options are not counted as members.
- A slate with no candidate that the voter can choose is not shown to that voter.

## What the voter sees

Under its name, each slate states what it covers:

| Text | Meaning |
| --- | --- |
| **Full slate** | A candidate for every seat of every contest |
| **_Contest_ only**, for example **Trustees only** | Candidates in a single contest, with other contests on the ballot |
| **Partial slate** | Any other partial slate |

The text is followed by the number of candidates and of offices the slate has candidates in, for example `Trustees only · 3 candidates · 1 office`.

On wide screens every slate lists every contest of the ballot in ballot order, so that the same office is in the same position on each slate. An office the slate has no candidate for shows **No candidate**. On phones those offices are left out.

Choosing a partial slate only changes the contests it has candidates for. The other offices keep the voter's own choices, or stay empty and are reported as such when the voter reviews the ballot.

## Example

Five contests elect seven people: President, Vice President, Secretary-Treasurer and Recording Secretary with one seat each, and Trustees with three.

```json
{
  "version": 1,
  "slates": [
    {
      "id": "forward",
      "name": {"en": "Forward Together"},
      "members": {
        "president-id": ["jordan-ellis-id"],
        "vice-president-id": ["taylor-morgan-id"],
        "secretary-treasurer-id": ["cameron-lee-id"],
        "recording-secretary-id": ["alex-parker-id"],
        "trustees-id": ["rowan-scott-id", "charlie-kim-id", "dakota-reed-id"]
      }
    },
    {
      "id": "voices",
      "name": {"en": "Independent Voices"},
      "members": {
        "trustees-id": ["harper-lane-id", "sage-murphy-id", "river-adams-id"]
      }
    }
  ]
}
```

Forward Together is shown as `Full slate · 7 candidates · 5 offices`. Independent Voices is shown as `Trustees only · 3 candidates · 1 office`, with **No candidate** under the other four offices.

## Custom CSS

| Class | Element |
| --- | --- |
| `slate-coverage` | The line stating what the slate covers. Its `data-coverage` attribute is `complete` or `partial` |
| `slate-coverage-kind` | **Full slate**, **_Contest_ only** or **Partial slate** |
| `slate-coverage-count` | The number of candidates and offices |
| `slate-contest-uncovered` | An office the slate has no candidate for |
| `slate-no-candidate` | The **No candidate** text |
