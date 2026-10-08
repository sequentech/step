---
id: election_management_election_event_approvals
title: Approvals
description: "The Approvals tab shows the enrollment applications of the election event. A voter sends an application when they enroll through the voter enrollment page."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Approvals

The **Approvals** tab shows the enrollment applications of the election event. A voter sends an
application when they enroll through the voter enrollment page. The platform can accept or reject
some applications automatically. An election administrator reviews the other applications on
this tab. To approve an application, you link it to a voter in the voter list. To reject an
application, you give a reason.

:::note
The **Approvals** tab is for voter enrollment only. It does not approve actions of other
administrators.
:::

## Open the tab

1. Open the election event.
2. Click the **Approvals** tab.

The tab shows only when these conditions are true:

- Your role has permission to see the approvals tab of the election event.
- The election event is not locked down. See **Lockdown Status** on the **Data** tab.

To approve or reject an application, your role must also have permission to change
applications. The **Import** and **Export** buttons show only if your role has permission to
import or export applications.

Each election also has an **Approvals** tab. It shows a part of these applications. See
[Election approvals](../03-election/06-election_management_election_approvals.md).

## Application list

The list shows 10 applications on each page, the newest first. When you open the tab, the list
shows the applications with the status **Pending**.

| Column | Meaning |
| --- | --- |
| **Id** | The identifier of the application. |
| **Created at** | The date and time when the voter sent the application. |
| **Updated at** | The date and time of the last change to the application. |
| **Applicant** | The identifier of the voter that the application is linked to. A dash (-) shows if there is no voter. |
| **Verification type** | `AUTOMATIC` if the platform made the decision. `MANUAL` if an administrator made the decision. |
| **Status** | `PENDING`, `ACCEPTED` or `REJECTED`. |
| **Verified By** | The name of the administrator who approved or rejected the application. A dash (-) shows if nobody did. |
| Voter attributes | One column for each attribute of the voter profile, with the value that the voter typed. The first name, the last name, the email, the username and the date of birth show by default. Use **Columns** to show the other attributes. |
| **Actions** | The view icon. It opens the application. |

### Filters

Click **Add filter** to add a filter.

| Filter | Use |
| --- | --- |
| **Status** | Select **Pending**, **Accepted** or **Rejected**. The admin portal keeps your selection on this browser. |
| **Verification Type** | Select **Manual** or **Automatic**. |
| **Applicant ID** | Type the identifier of the linked voter. |
| **ID** | Type the identifier of the application. |
| Voter attributes | One filter for each attribute of the voter profile. |

### Buttons

| Button | Use |
| --- | --- |
| **Columns** | Show or hide columns. |
| **Add filter** | Add a filter to the list. |
| **Import** | Import applications from a CSV file. The **Import applications** panel opens. |
| **Export** | Export all the applications of the election event to the file `export-applications.csv`. |

## Application page

When you click the view icon, the application page opens. It has two parts:

- **Approval Request**: a table with the data that the voter typed. If the application is
  rejected, the table also shows the **Rejection Reason**.
- **Voters** ("Find matching voters"): the voter list of the election event. The admin portal
  fills the filters with the data of the application, so the list shows the voters that match.
  You can change the filters.

| Control | Use |
| --- | --- |
| **Reject Application** | Reject the application. Shows only when the status is `PENDING`. |
| Approve icon (**Approve**) | In the **Actions** column of a voter. Approves the application and links it to this voter. Shows only when the status is `PENDING` or `REJECTED`. |
| **Back** | Go back to the application list. |

## Approve an application

When you approve an application, the platform does these things:

- It links the application to the voter that you select.
- It sets the status to `ACCEPTED` and the verification type to `MANUAL`.
- It copies the configured attributes from the application to the voter.
- It sends the configured response message to the voter.

:::caution CAUTION
Make sure that you select the correct voter before you approve. You cannot reverse an approval.
:::

1. On the **Approvals** tab, click the view icon of the application.
2. Compare the data in **Approval Request** with the voters in the **Voters** list.
3. If no voter shows, change the filters of the **Voters** list.
4. In the row of the correct voter, click the approve icon.
5. Read the message "Are you sure you want to approve this voter? This action is not
   reversible."
6. Click **Approve**.

**Expected result:** the message "Voter approved" shows. The admin portal opens the application
list.

## Reject an application

:::caution CAUTION
Make sure that the application is not valid before you reject it. The platform sends the
rejection message to the voter immediately.
:::

1. On the **Approvals** tab, click the view icon of the application.
2. Click **Reject Application**.
3. Read the message "Are you sure you want to reject this voter? This action is not
   reversible."
4. In **Rejection Reason**, select the reason. The table gives the values.
5. If the reason is **Other**, type the reason in **Write here the disapproval reason**.
6. Click **Reject Application**.

| **Rejection Reason** | Use |
| --- | --- |
| **Missing Data** | The application does not have enough data. |
| **No Matching Voter** | No voter in the voter list matches the application. |
| **Already Approved** | The voter is already approved. |
| **Other** | Another reason. You must type the reason. |

**Expected result:** the message "Voter rejected" shows. The admin portal opens the application
list. The platform sends the configured response message to the voter.

:::note
The admin portal shows the approve icon also for a rejected application. If you rejected an
application by mistake, open it and approve it. See
[Approve an application](#approve-an-application).
:::

## Export the applications

1. On the **Approvals** tab, click **Export**.
2. Read the message "Export can be a long operation. Are you sure you want to export records?".
3. Click **Export**.
4. Wait until the download starts.

**Expected result:** the file `export-applications.csv` downloads. The message "Applications
export finished successfully" shows.

## If there is a problem

| Problem | Action |
| --- | --- |
| You do not see the **Approvals** tab. | Make sure that the election event is not locked down. Ask your tenant administrator for permission to see the approvals tab. |
| "Voter is already approved." | The voter that you selected is already approved. Select a different voter, or reject the application with **Already Approved**. |
| "Error approving voter" | Make sure that your role has permission to change applications. Try again. If the problem continues, contact Sequent support. |
| "Error rejecting voter" | Make sure that your role has permission to change applications. Try again. |
| "A rejection message is required for the 'Other' option." | Type the reason in **Write here the disapproval reason**. |
| "Error exporting applications" | Try again. Look at the **Tasks** tab for the status of the export. |
| "Error importing applications" | Make sure that the file is a CSV file. Look at the **Tasks** tab for the error. |
| The list is empty. | Change the **Status** filter. The list shows only **Pending** applications when you open it. |

## Related pages

- [Voters](05-election_management_election-event_voters.md)
- [Tasks](10-election_management_election-event_tasks.md)
- [Create the Election Event](../../03-procedures/02-event.md)
