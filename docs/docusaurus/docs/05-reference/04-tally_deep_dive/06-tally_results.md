---
id: tally_results
title: Generating Results
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


Tally results report participation, blank votes, invalid votes, candidate
totals, and algorithm-specific process data.

## Vote classifications

Contest results use these vote classifications:

- **Explicit blank vote**: the voter selected the contest's explicit blank
  candidate and did not select any regular candidate in that contest.
- **Implicit blank vote**: the voter made no selection in the contest.
- **Explicit invalid vote**: the decoded contest has its explicit invalid flag
  set.
- **Implicit invalid vote**: the ballot is invalid because of the decoded
  contest contents, for example selecting the explicit blank candidate together
  with a regular candidate.
- **Declined vote**: a ballot covered by decline-to-vote policy. Declined
  ballots are not valid votes, invalid votes, or blank votes at contest level.

`blank_votes.explicit` and `blank_votes.implicit` retain the split in
tally output. Persisted result rows expose the same counts as
`explicit_blank_votes` and `implicit_blank_votes`.
`total_blank_votes` is their sum.

## Valid votes and candidate votes

`total_valid_votes` counts every ballot that is not invalid and not declined.
Explicit and implicit blank votes are included in `total_valid_votes`.

Valid non-blank participation is calculated as:

```text
valid_non_blank_ballots = total_valid_votes - total_blank_votes
```

`valid_non_blank_ballots` is a participation count, not a universal
candidate-percentage denominator:

- Instant Runoff regular candidate percentages use valid non-blank ballots.
- Plurality-at-large regular candidate percentages use
  `extended_metrics.total_weight`, the total weighted regular-candidate
  marks. A valid ballot can contribute more than one mark.

The explicit blank candidate row is reported separately and uses all submitted
ballots (`extended_metrics.total_ballots`) as its denominator. Contest
summary percentages use `total_votes`, which is valid plus invalid votes
and excludes declined ballots. Summary rows labelled as valid votes include
blank votes.

## Release 10.0 compatibility note

The release ballot layouts are unchanged: an explicit blank remains a selected
candidate in its existing encoded position. Instant Runoff previously reported
`total_valid_votes` without blank votes, while Plurality-at-large already
counted them as valid. Both algorithms now use the same definition: blank votes
are valid votes, and a mixed blank plus regular candidate selection is an
implicit invalid vote.
