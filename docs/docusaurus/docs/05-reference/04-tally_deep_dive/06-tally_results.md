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

## Acclaimed contests

A contest with `is_acclaimed: true` is decided without a vote and is absent
from decoded ballots. The tally therefore synthesizes its result from the
published contest configuration instead of reading ballots or applying the
configured counting algorithm.

The canonical acclaimed result has:

- zero census, auditable votes, total votes, valid votes, invalid votes, blank
  votes, candidate totals, and percentages;
- every eligible configured candidate marked as a winner in configured order,
  with deterministic winning positions `1..N`; and
- no counting-algorithm process data or participation-by-channel data.

Explicit blank and invalid markers, disabled candidates, and empty write-in
slots are configuration artefacts and are excluded from the candidate result.
Results screens and reports replace the normal participation summary with an
acclamation notice. Paper, postal, and other tally sheets are rejected because
there is no vote to add to an acclaimed contest.

## Vote classifications

Contest results use these vote classifications:

- **Explicit blank vote**: the voter selected the contest's explicit blank
  candidate and did not select any regular candidate in that contest.
- **Implicit blank vote**: the voter made no selection in the contest.
- **Explicit invalid vote**: the voter selected the explicit invalid option, or
  the decoded ballot is otherwise marked as explicitly invalid.
- **Implicit invalid vote**: the ballot is invalid because of the decoded
  contest contents, for example selecting the explicit blank candidate together
  with a regular candidate.
- **Declined vote**: a ballot covered by decline-to-vote policy. Declined
  ballots are not valid votes, invalid votes, or blank votes at contest level.

`total_blank_votes` is the sum of explicit and implicit blank votes.

## Valid votes and candidate votes

`total_valid_votes` counts every ballot that is not invalid and not declined.
Explicit and implicit blank votes are included in `total_valid_votes`.

Votes for candidates are calculated as:

```text
votes_for_candidates = total_valid_votes - total_blank_votes
```

Candidate percentages use votes for candidates when the counting algorithm
reports candidate rows against participating non-blank ballots. Summary rows
labelled as valid votes include blank votes.

## Compatibility note

This is a breaking results-semantics change for 9.x. Before this change,
Instant Runoff results reported `total_valid_votes` excluding blank votes.
Plurality-at-large already counted blank votes as valid. Both algorithms now use
the same definition: blank votes are valid votes, and a mixed blank plus regular
candidate selection is an implicit invalid vote.
