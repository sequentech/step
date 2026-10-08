---
id: election_management_election_event_approvals
title: Approvals
description: "The Approvals tab lists the enrollment applications of the election event. When voters enroll in the voting portal, each application gets the status Pending, Accepted or Rejected."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Approvals** tab lists the enrollment applications of the election event. When voters
enroll in the voting portal, each application gets the status **Pending**, **Accepted** or
**Rejected**. On this tab you check an application, find the matching voter, and approve or
reject it.

The tab is useful only when **Enrollment** is **Enabled** on the **Data** tab, in **Advanced
Configurations** > **Voter Authentication**.

## Open the tab

1. In the menu on the left, click the election event.
2. Click the **Approvals** tab.

The tab shows with the **View Election Event Approvals** permission. It is hidden when the
election event is locked down. You see only the applications of the elections that your
permission labels permit.

## Toolbar

| Button | Use | Permission |
| --- | --- | --- |
| **Add filter** | Filters by **Status**, **Verification Type** (**Manual** or **Automatic**), **Applicant ID**, **ID** and the applicant data. | - |
| **Import** | Opens **Import applications** to import applications from a CSV file. | **Application Import** |
| **Export** | Opens **Export applications** to export the applications as a CSV file. | **Application Export** |

## Columns

| Column | Content |
| --- | --- |
| **Id** | The identifier of the application. |
| **Created at** and **Updated at** | The dates of the application. |
| **Applicant** | The identifier of the applicant, or `-`. |
| **Verification type** | `MANUAL` or `AUTOMATIC`. |
| **Status** | The status of the application. |
| **Verified By** | The administrator who approved or rejected the application. |
| Applicant data | The data that the voter typed, one column for each field. |
| **Actions** | The view icon. It opens the application. |

## Review an application

1. In the row of the application, click the view icon.
2. Read **Approval Request**: the data of the applicant.
3. Look at the list of voters below it. The list shows the voters that match the data.
4. To approve, click the approve icon in the row of the correct voter. Read "Are you sure you
   want to approve this voter? This action is not reversible." Click **Approve**.
5. To reject, click **Reject Application**. Select the **Rejection Reason**. If you select
   **Other**, type the reason. Confirm the action.

**Expected result:** the message "Voter approved" or "Voter rejected" shows.

| **Rejection Reason** | Use |
| --- | --- |
| **Missing Data** | The application does not have enough data. |
| **No Matching Voter** | No voter matches the application. |
| **Already Approved** | The voter is approved already. |
| **Other** | Another reason. You must type it. |

:::caution CAUTION
You cannot reverse an approval or a rejection. Check the data before you confirm.
:::

## If there is a problem

| Problem | Action |
| --- | --- |
| "Voter is already approved." | The voter was approved before. No action is necessary. |
| "A rejection message is required for the 'Other' option." | Type the reason of the rejection. |
| "Error approving voter" or "Error rejecting voter" | Try again. If the error stays, contact Sequent support. |
| The list is empty. | Check the **Status** filter. The tab remembers the last status that you selected. |

## Related pages

- [Election: Approvals](../../Election/Approvals/election_management_election_approvals.md)
- [Data](../Data/election_management_election-event_data.md)
- [Voters](../Voters/election_management_election-event_voters.md)
- [Monitoring](../Monitoring/election_management_election-event_monitoring.md)
