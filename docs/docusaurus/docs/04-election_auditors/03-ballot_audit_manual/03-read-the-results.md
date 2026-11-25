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
For the evidence and the connection between the checks, read
[What the Ballot Audit Verifies](../04-ballot_audit_explanation.md#what-a-successful-run-establishes).

## Exit statuses

All six ballot audit commands use the same exit statuses.

| Status | Meaning | Action |
| --- | --- | --- |
| `0` | The check passed. | Continue to the next procedure. |
| `1` | Contradiction. The evidence disagrees with itself, with the pins or with the scope. | Stop. Keep the evidence. Read the message and report the finding. |
| `2` | Usage error. A command option is wrong or missing. | Correct the command and run it again. |
| `3` | Incomplete evidence. Part of the evidence is missing, so the check cannot pass or fail. | Read the message. Report it as incomplete, not as a pass and not as a finding. |
| `4` | Input refused. A file is not valid, a pin is malformed, a connection is refused or the setup is not supported. | Correct the input, or write in the audit report that the tools cannot audit this setup. |

The Braid verifier returns a status other than `0` when a required check fails or is incomplete.

A successful Braid status includes the verifier's own recomputation of the plaintexts of every batch.
The `DECRYPTION_VALID` and `PLAINTEXTS_VALID` fields check references in plaintext records; the recomputation is a separate condition of the success.

Read the restore logs and provenance reports even when the command returns `0`.
The image does not audit copies with restore errors, but some older annotation fields can be unavailable without failing the command.
Check that all required output files exist and cover the expected scope before reporting a pass.
In the reviewed implementation, a report-save problem does not necessarily change the recorded child-command status.

For `audit-tally`, a nonzero Braid exit becomes `verification_failed`, including connection failures.
A child command crash can also have that status.
The image itself can return `1` before the audit starts. Read the message before classifying the cause as a contradiction.

:::caution CAUTION
Do not count a `3` as a pass. An audit with an incomplete result did not pass.
:::

## Statuses of audit-tally

`audit-tally` gives a status to each step in `audit-record.json`. It also gives a status to each
scope, to each election history, to the Braid verifier and to the full audit in
`public-summary.json`.

| Status | Meaning | Exit status |
| --- | --- | --- |
| `success` | The step passed. | `0` |
| `incomplete` | The step returned `3`. Part of the evidence is missing. | `3` |
| `not_run` | The step did not run, because an earlier step did not pass or an input is missing. The `message` gives the cause. | `3` |
| `invalid_input` | The step returned `2` or `4`, or the command could not start. | `4` |
| `verification_failed` | The step returned `1` or another status. For the Braid verifier: any status other than `0`. | `1` |

The status of a scope, of an election history or of the full audit is the most serious status of
its steps. The order is: `verification_failed`, then `invalid_input`, then `incomplete` and
`not_run`. The exit status of `audit-tally` is the exit status of the full audit.

:::caution CAUTION
Do not count `not_run` as a pass. A step that did not run makes the audit incomplete.
:::

When a step does not pass, `audit-tally` records the next steps of the same scope as `not_run`:

| Step that did not pass | Steps with `not_run` |
| --- | --- |
| `export-electoral-log` | `prepare-ballot-audit`, `ballot-audit` and `audit-cast-history` of all scopes. |
| `export-ballot-audit` of a scope | `prepare-ballot-audit` and `ballot-audit` of the scope. `audit-cast-history` of the election if no scope of the election was exported. |
| `prepare-ballot-audit` of a scope | `ballot-audit` of the scope. |

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

These outcomes describe the checked input bundle.
Exclusion reasons come from stored labels and are not independently verified by the lookup.

| Outcome | Meaning |
| --- | --- |
| `selected` | The selected ballot matches the checked encrypted tally inputs. The report lists its matching positions. |
| `excluded` | The receipt appears in the selection file, but not among the selected ballots. The lookup reports the stored reason labels. |
| `missing` | The receipt was not found in the selected bundle or the selection records searched. Check `selection_searched` before drawing a conclusion. |
| `ambiguous` | More than one selected cast row has this receipt. Duplicate selected receipts make the audit fail. |

## Reasons for an excluded ballot

| Reason | Meaning |
| --- | --- |
| `superseded_revote` | Another cast row is the one retained by the selection rules. |
| `discarded` | The retained cast row is marked discarded in the application database. |
| `no_eligible_voter` | The retained row's voter ID has no entry in the supplied eligibility list for this scope. |
| `outside_scope` | The stored row is outside the scope, such as another area or election. |

## Results of the Braid verifier

The verifier shows how many required checks passed. All required checks must have successful results.
A failed key, mix or plaintext-reference check is a finding, even if the ballot input audit passed.
See [Versions](01-before-you-start.md#versions) for the requirement before reporting verified decryption.

For completeness failures, see [Troubleshooting](04-troubleshooting.md#evidence-messages), including missing plaintext records in required batches.
