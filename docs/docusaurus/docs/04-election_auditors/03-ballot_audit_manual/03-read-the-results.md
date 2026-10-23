---
title: Read the Results
sidebar_position: 3
description: "What each exit status, receipt lookup outcome and exclusion reason means, and what to do next."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Read the Results

This page tells you what each result means and what to do next.

## Exit statuses

All five ballot audit commands use the same exit statuses.

| Status | Meaning | Action |
| --- | --- | --- |
| `0` | The check passed. | Continue to the next procedure. |
| `1` | Contradiction. The evidence disagrees with itself, with the pins or with the scope. | Stop. Keep the evidence. Read the message and report the finding. |
| `2` | Usage error. A command option is wrong or missing. | Correct the command and run it again. |
| `3` | Incomplete evidence. Part of the evidence is missing, so the check cannot pass or fail. | Read the message. Report it as incomplete, not as a pass and not as a finding. |
| `4` | Input refused. A file is not valid, a pin is malformed, a connection is refused or the setup is not supported. | Correct the input, or write in the audit report that the tools cannot audit this setup. |

The Braid verifier returns a status other than `0` when a required check fails or is incomplete.

:::caution CAUTION
Do not count a `3` as a pass. An audit with an incomplete result did not pass.
:::

## Contradiction or a change after the tally?

An exit status `1` does not always show a change of a vote. The saved tally is a record of one
moment. If a voter or a ballot changed after the tally, the copies disagree with the saved
counts. Examples:

- An administrator disabled a voter after the tally. The saved number of voters is then larger
  than the frozen list.
- An administrator discarded the ballots of a voter after the tally.

Before you report a finding, ask the election authority about changes after the tally. Write
the answer in the audit report.

## Receipt lookup outcomes

| Outcome | Meaning |
| --- | --- |
| `selected` | The tally counted the ballot. |
| `excluded` | The ballot is stored, but the tally did not count it. The report gives a reason. |
| `missing` | The ballot is not in the scope. It can be in another area or another election. |
| `ambiguous` | More than one counted ballot has this receipt. |

## Reasons for an excluded ballot

| Reason | Meaning |
| --- | --- |
| `superseded_revote` | The voter voted again later. Only the last valid vote counts. |
| `discarded` | An administrator discarded the ballot. |
| `no_eligible_voter` | The voter is not in the frozen list of voters of the area. For example, the voter was disabled. |
| `outside_scope` | The ballot belongs to another area or another election. |

## Results of the Braid verifier

The verifier shows how many required checks passed. All checks must pass. A failed check of the
mix or of the decryption is a finding, even if the ballot audit passed. The ballot audit does not
check the mix or the decryption.
