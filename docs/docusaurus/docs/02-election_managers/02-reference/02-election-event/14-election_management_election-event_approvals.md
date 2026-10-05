---
id: election_management_election_event_approvals
title: Approvals
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Approvals** tab of an election event lists the enrollments of its voters. Each enrollment is
compared with the voter registry when the voter enrolls, and is then:

- **Approved**: the voter is enrolled without anybody reviewing the enrollment.
- **Needs review**: an election officer reviews the enrollment and approves or rejects it.
- **Rejected**: the voter is told why and is not enrolled.

Which of the three happens is decided by the election event's **approval matrix**.

## The queue

The tab opens on the enrollments that need review. Each row shows:

| Column | What it shows |
|---|---|
| Voter | The name and email on the enrollment |
| What happened | Which details differ from the registry, or that the voter typed their details by hand; for a decided enrollment, who decided it |
| Post | The area of the enrollment, or the Post the voter named |
| When | How long the enrollment has been waiting, and the day it was sent |
| Status | Needs review, Approved or Rejected |

- **Search** finds enrollments by any text the voter gave: name, email or ID number.
- **Status** switches between the enrollments that need review and those already approved or
  rejected. **Add filter** filters by verification, IDs or any voter attribute.
- **Columns** shows more columns (verification, verified by, IDs and every voter attribute) and
  changes their order.
- The menu at the end of a row opens the enrollment, or the rule that decided it.

**Import** and **Export** work on the enrollments as CSV files.

## Review an enrollment

Select a row to review the enrollment. The top of the page says why it needs a person, for example
that the first name is "Juan Carlos" on the enrollment and "Juan" in the registry, and names the
matrix version and rule that sent it to review. **See the rule** opens the approval matrix on that
rule.

The review has three steps:

1. **Check the identity.** Shows how the voter's identity was established and the details on the
   enrollment. When the voter typed their details by hand instead of scanning an ID, meet them in
   person or by video call, compare their ID with the details, and tick the confirmation. The
   enrollment can't be approved before that.
2. **Find the voter.** Lists the registry voters closest to the enrollment, the best match first,
   with how many of the compared details match. A voter who is already enrolled can't be chosen.
   Choosing a voter compares each detail on the enrollment with the registry. If the voter is not
   listed, search the registry by name or email, or choose **None of these is the voter**.
3. **Decide.** **Approve** links the enrollment to the chosen voter, after a confirmation; the
   voter is told and can sign in to vote when voting opens. **Reject** asks for the reason (missing
   data, no matching voter, already approved, or other with your own message) and shows the message
   the voter will get. Neither can be undone.

An approved enrollment opens with how it was decided and its details. A rejected enrollment can
still be approved with the same steps.

The registry comparison on this page ignores capital letters, accents and hyphens in the same way
the rules do, and compares first and middle name together for a driver's license and a seafarer's
book.

## Approval matrix

Select **Approval matrix** in the toolbar of the Approvals tab to see the matrix in force. The page
shows its version and who saved it, and has three parts.

### What we compare

The voter details that are compared with the registry voter, for example first name, middle name,
last name, date of birth and embassy. Names ignore capital letters, accents and hyphens. For a
driver's license and a seafarer's book, first and middle name are compared together: the first name
carries the result and the middle name has none of its own.

Select a detail to stop or start comparing it, and **Compare another detail** to add any other
voter attribute. Conditions on a detail that is no longer compared are removed from the rules.

### Rules

Rules are checked from the top and the first rule that fits decides the enrollment. When none
fits, the last rule, **Otherwise**, applies. Each rule reads as a sentence: **When** its conditions
hold, **Then** its decision, with what the voter is told.

A rule can have any of these conditions; a condition that doesn't matter is left out:

| Condition | Values |
|---|---|
| Identity check | Verified by ID scan, or typed by hand |
| Voter in the registry | Yes or no |
| Already enrolled | Yes or no |
| ID type | The type of identity document presented |
| Details that differ | None, exactly 1, at most 1, exactly 2, at most 2, 3 or more |
| Each compared detail | Same or different |

A rule decides one of:

| Decision | Result | What the voter is told |
|---|---|---|
| Approve automatically | Approved | Nothing |
| Send to a person | Needs review | Required |
| Reject | Rejected | Required |

What the voter is told is one of: No matching voter, Already approved, Missing data, Identity not
verified and Other. The rule editor shows the message the voter sees for each.

When the registry returns several voters for an enrollment, each one is checked against the rules,
and the results are combined always in the same way, whatever the order of the voters:

1. Exactly one voter approved: the enrollment is approved for that voter.
2. Several voters approved: the enrollment goes to a person.
3. Otherwise, any voter sent to a person: the enrollment goes to a person.
4. Otherwise the enrollment is rejected. If one of the voters is already enrolled, that is the
   reason.

When the registry returns no voter, only the rules that compare no details can apply, and then
Otherwise.

### What a matrix can never do

These limits are part of the platform. A matrix that breaks one can't be saved, and enrollment
applies them again when it decides:

- An enrollment whose identity was typed by hand is never approved automatically.
- A voter who is already enrolled is never approved again.
- Nobody is approved without a voter in the registry.
- The last rule can send enrollments to a person or reject them, not approve them.
- Every decision other than approving says what the voter is told.

The rule editor also refuses a rule without conditions, because it would decide every enrollment
that reaches it and hide the rules below.

### Try an example

**Try an example** describes an enrollment (how the identity was checked, whether the voter is in
the registry and already enrolled, the ID type, and which details are the same) and shows which
rule decides it and what the voter would be told. The rule is marked **Applies to your example** in
the list. It uses the rules on the screen, including changes that are not saved yet, and decides
exactly as a real enrollment with the same data would. Nothing is stored.

### Change the matrix

Changing the matrix needs the `approval-matrix-write` permission. Without it the matrix is **View
only** and the example still works.

1. Add a rule with **Add rule**, or edit, move up, move down or delete a rule with the buttons on
   it. The editor says the rule in one sentence while you change it: add or remove conditions,
   choose the decision and what the voter is told, and select **Apply**. The compared details and
   the last rule can be changed too.
2. Try the changes with **Try an example**.
3. The page shows that there are unsaved changes and what they are. **Discard changes** goes back
   to the saved version. **Save as version N** lists what changed and asks for confirmation.

Each save creates a new version and an entry in the election event's electoral log with the
version's number and SHA-256. Saved versions are never changed or deleted. New enrollments are
decided with the latest version from that moment on; enrollments that were already decided keep
their decision.

An election event that has never saved a matrix uses the built-in **version 1**, which decides as
the platform did before matrices were configurable:

| # | When | Then | The voter is told |
|---|---|---|---|
| 1 | Already enrolled, at most 1 detail differs | Reject | Already approved |
| 2 | Identity typed by hand | Send to a person | Identity not verified |
| 3 | All details match | Approve automatically | |
| 4 | Exactly 1 detail differs, embassy differs | Approve automatically | |
| 5 | Exactly 1 detail differs, embassy matches | Send to a person | No matching voter |
| 6 | Exactly 2 details differ, embassy differs | Send to a person | No matching voter |
| 7 | Exactly 2 details differ, middle name and last name differ | Send to a person | No matching voter |
| | Otherwise | Reject | No matching voter |

Until a version is saved, the details compared are the ones configured in the enrollment flow of
the election event's realm (`search-attributes`). The first save creates version 2.

A matrix can change while enrollment is open. Reviewers are not notified by the platform, so agree
on when changes may happen before enrollment opens.

## Why an enrollment was decided

The review of an enrollment names the approval matrix version and the rule that decided it, with
the rule's conditions, for example:

> Rule 5 of matrix version 1 · Exactly 1 detail differs · Embassy matches

**See the rule** opens the matrix with that rule marked **Decided the enrollment you came from**,
as long as that version is still the one in force. Enrollments decided before this information was
recorded don't show a rule.

## Election event exports

An export of the election event carries the matrix in force. Importing it saves that matrix as
version 1 of the new election event. Earlier versions are not exported.
