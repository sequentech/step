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

The first time you open the tab in a browser, it shows the enrollments that need review. After
that, it opens with the filters you last used for this election event (or election) in this
browser: status, search, any added filters, sorting, page size and page. A filter you removed stays
removed: after removing **Status**, the tab keeps showing every enrollment.

- Each administrator who signs in on the same browser keeps their own filters. Another browser
  starts again from the enrollments that need review.
- Nothing is stored on the server. The filters, including any search text or voter details typed
  into them, stay in this browser's storage until its site data is cleared, also after signing out.
- Saved filters that no longer apply, for example one on a voter attribute that was removed, are
  discarded and the tab opens as on a first visit. An administrator without permission to read
  voter attributes always starts from the enrollments that need review, because their saved filters
  can't be checked.
- The tab always shows the enrollments of the election event (or election) you are on, whatever was
  saved.
- A saved search for a support reference is looked up again when the tab opens; the enrollments it
  matched before are not reused.

Each row shows:

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
- The menu at the end of a row opens the enrollment or, with **See the rule that decided**, the
  approval matrix with that enrollment as its example.

**Import** and **Export** work on the enrollments as CSV files.

## Review an enrollment

Select a row to review the enrollment. The top of the page says why it needs a person, for example
that the first name is "Juan Carlos" on the enrollment and "Juan" in the registry, and names the
matrix version and rule that sent it to review. **See the rule** opens the approval matrix with
this enrollment as its example (see [Try an example](#try-an-example)).

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
the rules do, and compares first and middle name together for a driver's license, a seafarer's
book and a passport read by the scanner.

## Approval matrix

Select **Approval matrix** in the toolbar of the Approvals tab to see the matrix in force. The page
shows its version and who saved it, and has three parts.

### What we compare

The voter details that are compared with the registry voter, for example first name, middle name,
last name, date of birth and embassy. Names ignore capital letters, accents and hyphens. For a
driver's license and a seafarer's book, first and middle name are compared together: the first name
carries the result and the middle name has none of its own. So are those of a passport read by the
scanner: its machine-readable zone has a single field for the given names, the middle name included,
and they are compared with the registry voter's first and middle name joined. A passport enrollment
with a middle name of its own, typed apart, compares the two apart.

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

An enrollment the rules approve is still rejected, as Email used by another voter, when another
voter of the election event already has its email: each email can belong to one voter only. The
voter is told to enroll again with a different email.

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

**Try an example** decides an example enrollment (how the identity was checked, whether the voter
is in the registry and already enrolled, the ID type, and which details are the same) with the
rules on the screen, including changes that are not saved yet, exactly as a real enrollment with
the same data would be decided. Trying an example changes no enrollment and saves nothing.

- **Applied example.** The panel lists the example in use. Under **Current simulation** it shows
  which rule decides it and what the voter would be told; **Go to rule** scrolls to that rule. In
  the list, only that rule is highlighted, marked **Decides the applied example**. While the
  example is being decided, or when it can't be (the rules have a problem, or trying it fails), no
  rule is highlighted.
- **Edit example** opens the example's details below it, marked **Editing the example — not
  applied yet**. Changes there decide nothing until **Apply example**; **Cancel** leaves the
  applied example as it was. Editing, moving, adding or deleting rules, changing the compared
  details, saving and discarding never apply the edited details: the applied example is decided
  again with the changed rules.
- **Opened from the toolbar**, the example is the **sample enrollment**: a verified voter in the
  registry, not enrolled yet, whose compared details are all the same. After applying other
  details it is shown as **Your example**. In the editor, **Reset to sample** fills in the
  sample's details; like any change there, they decide nothing until **Apply example**.
- **Opened from an enrollment** (the row menu or **See the rule**), the example is the details the
  rules saw when they decided that enrollment, and the top of the panel has **Back to …'s
  enrollment**, which returns to that enrollment's review. **Approvals**, above the title, returns
  to the list. In the editor, **Restore recorded details** fills in the enrollment's recorded
  details; they decide nothing until **Apply example**.

When the example comes from an enrollment, the panel also shows the **recorded decision**: the
rule and matrix version that decided the enrollment and what they decided, and the enrollment's
status as the list or review showed it, when an officer had decided since. It is context only: no rule is highlighted for it. When the rules on
screen are given exactly the recorded details, the panel says whether they now give the same
decision. It doesn't compare when the details were changed, when several registry voters were
compared, or when a compared detail wasn't recorded:

- A compared detail that the decision didn't record, for example one compared only since, is shown
  as **Not recorded**. It is never assumed to be the same: the rules treat it as differing in
  conditions on that detail, and don't count it among the details that differ.
- When the enrollment's ID gives first and middle name together (seafarer's book, driver's
  license, passport without a middle name), the middle name shows **Compared with the first
  name**. This follows the enrollment's ID type as it is now.
- When the recorded decision can't be read, the panel says so and uses the sample enrollment.

The applied example is kept while the matrix is open. Leaving the matrix (back to the enrollment
or to the list) and opening it again starts from the enrollment's recorded details or the sample.
Reloading the page opens the election event on its first tab, or the tab named in the address, and
the example, its source enrollment and unsaved rule changes are lost.

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

**See the rule** opens the matrix with this enrollment's recorded details as the example. The
panel names the recorded rule and version as context, and the list highlights only the rule that
decides the example with the rules on screen (see [Try an example](#try-an-example)). Enrollments
decided before this information was recorded don't show a rule.

## Election event exports

An export of the election event carries the matrix in force. Importing it saves that matrix as
version 1 of the new election event. Earlier versions are not exported.

## Enrollments that match no voter

An enrollment that matches **no voter of the registry** is decided by the approval matrix like any
other, and its built-in rules reject it. An election event's realm can send it to review instead,
whatever the matrix decides, with the `no-matching-voter-policy` setting of its census lookup
(`lookup-and-update-user`) set to `PENDING_APPROVAL`: it then waits in the queue with the reason
*no matching voter*. The COMELEC template does this. See
[Scanovate Identity Verification](../../../integrations/scanovate_identity_verification_guide.md#after-the-identity-verification).

A voter who isn't in the registry yet has to be added to it before the enrollment can be approved;
otherwise, reject it.
