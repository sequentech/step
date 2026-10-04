---
id: election_management_election_event_approvals
title: Approvals
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Approvals** tab of an election event lists the enrollment applications of its voters. Each
application is compared with the voter registry when the voter enrolls, and is then:

- **Accepted**: the voter is enrolled without anybody reviewing the application.
- **Pending**: an election officer reviews the application and approves or rejects it.
- **Rejected**: the voter is told why and is not enrolled.

Which of the three happens is decided by the election event's **approval matrix**.

## Approval matrix

Select **Approval Matrix** in the toolbar of the Approvals tab to see the matrix in force. It has
three parts.

### Compared with the registry

The voter attributes that are compared with the registry voter, for example first name, middle
name, last name, date of birth and embassy. Names ignore case, accents and hyphens. For a Driver's
License and a Seafarer's Book, first and middle name are compared together: the first name carries
the result and the middle name has none of its own.

### Rules

Rules are checked in order and the first rule that applies decides the enrollment. When no rule
applies, the last rule, **Otherwise**, decides.

A rule can require any of these conditions. A condition left as **Any** is not checked.

| Condition | Values |
|---|---|
| Identity Verification | Verified, or entered manually |
| Voter Found In Registry | Yes or no |
| Voter Already Enrolled | Yes or no |
| Valid ID | The type of identity document presented |
| Fields That Differ | None, exactly 1, at most 1, exactly 2, at most 2, 3 or more |
| Each compared field | Matches or differs |

A rule decides one of:

| Decision | Result | Reason shown to the voter |
|---|---|---|
| Approve automatically | Accepted | None |
| Send to manual review | Pending | Required |
| Reject | Rejected | Required |

The reasons are: No Matching Voter, Already Approved, Missing Data, Identity Not Verified and Other.

When the registry returns several voters for an enrollment, each one is checked against the rules,
and the results are combined always in the same way, whatever the order of the voters:

1. Exactly one voter accepted: the enrollment is accepted for that voter.
2. Several voters accepted: the enrollment goes to manual review.
3. Otherwise, any voter sent to manual review: the enrollment goes to manual review.
4. Otherwise the enrollment is rejected. If one of the voters is already enrolled, that is the
   reason.

When the registry returns no voter, only the rules that compare no fields can apply, and then
Otherwise.

### What a matrix can never do

These limits are part of the platform. A matrix that breaks one can't be saved, and enrollment
applies them again when it decides:

- An enrollment whose identity was entered manually is never approved automatically.
- A voter who is already enrolled is never approved again.
- Nobody is approved without a voter in the registry.
- The last rule can send enrollments to manual review or reject them, not approve them.
- Every decision other than approving has a reason.

### Test the matrix

Under the rules, **Test The Matrix** lets you describe an enrollment (how the identity was
verified, whether the voter was found and is already enrolled, the identity document, and which
fields match) and shows which rule decides it. It uses the rules on the screen, including changes
that are not saved yet, and decides exactly as a real enrollment with the same data would. Nothing is
stored.

### Change the matrix

Changing the matrix needs the `approval-matrix-write` permission. Without it the matrix is shown
read-only and the test panel still works.

1. Add a rule with **Add Rule**, or edit, move up, move down or delete a rule with the actions of its
   row. The compared fields and the last rule can be edited too.
2. Try the changes with **Test The Matrix**.
3. Select **Save** and confirm. The rules are saved as a new version.

Each save creates a new version and an entry in the election event's electoral log with the
version's number and SHA-256. Saved versions are never changed or deleted. New enrollments are
decided with the latest version from that moment on; applications that were already decided keep
their decision.

An election event that has never saved a matrix uses the built-in **version 1**, which decides as
the platform did before matrices were configurable:

| # | Conditions | Decision | Reason |
|---|---|---|---|
| 1 | Voter already enrolled, at most 1 field differs | Rejected | Already Approved |
| 2 | Identity entered manually | Pending | Identity Not Verified |
| 3 | All compared fields match | Accepted | |
| 4 | Exactly 1 field differs, embassy differs | Accepted | |
| 5 | Exactly 1 field differs, embassy matches | Pending | No Matching Voter |
| 6 | Exactly 2 fields differ, embassy differs | Pending | No Matching Voter |
| 7 | Exactly 2 fields differ, middle name and last name differ | Pending | No Matching Voter |
| | Otherwise | Rejected | No Matching Voter |

Until a version is saved, the fields compared are the ones configured in the enrollment flow of the
election event's realm (`search-attributes`). The first save creates version 2.

A matrix can change while enrollment is open. Reviewers are not notified by the platform, so agree
on when changes may happen before enrollment opens.

## Why an application was decided

Open an application to see its details. **Decided By** names the approval matrix version and the
rule that decided it, with the rule's conditions, for example:

> Approval matrix version 1, rule 5: Exactly 1 field differs, Embassy matches

Applications decided before this information was recorded have no Decided By row.

## Election event exports

An export of the election event carries the matrix in force. Importing it saves that matrix as
version 1 of the new election event. Earlier versions are not exported.
