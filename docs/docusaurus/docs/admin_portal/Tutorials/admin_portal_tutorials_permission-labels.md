---
id: admin_portal_tutorials_permission-labels
title: Permission Labels
description: "A permission label limits the elections that an administrator can see and use. Use permission labels when different teams manage different elections of the same election event."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

A permission label limits the elections that an administrator can see and use. Use permission
labels when different teams manage different elections of the same election event. For
example, the team of "Area North" sees only the elections with the label `north`.

Permission labels are not permissions. Permissions say what a user can do. Permission labels
say on which elections the user can do it.

## How permission labels work

- An election has one permission label, or none. It is in **Permission Label** on the **Data**
  tab of the election.
- A user has zero, one or more permission labels. They are in **Permission Label** in the user
  form on the **Users** tab.
- A user sees an election when the election has no label, or when the user has the label of
  the election. A user without labels sees only the elections without a label.
- The labels also limit the key ceremonies, the tallies, the reports and the enrollment
  applications. A key ceremony or a tally gets the labels of its elections.

| Screen | Effect of the labels |
| --- | --- |
| Election tabs | A user without the label of the election sees "You don't have permission to access this election.". |
| **Keys** tab | The **Permission Labels** column shows the labels of the ceremony. To create a ceremony, the user needs the labels of its elections. Without them: "Cannot create keys ceremony: one or more permission labels are missing." |
| **Tally** tab | The **Permission Labels** column shows the labels of the tally. The tally accepts only elections with the labels of the user. |
| **Reports** tab | A report has its own **Permission Label**. |
| **Approvals** tab of an election | Shows only the applications with the label of the election. |

## Before you start

- Your role has the **Edit Permission Label** permission. Without it, the **Permission Label**
  fields do not show.
- The user profile of the tenant has the permission label attribute. If **Permission Label** is
  not in the user form, ask Sequent support.

## Give a label to an election

1. Open the election.
2. Click the **Data** tab.
3. Open **Advanced Configuration**.
4. Type the label in **Permission Label**, for example `north`.
5. Click **Save**.

**Expected result:** only users with the label `north` see the election. A user without
labels does not see it.

## Give labels to a user

1. In the menu on the left, click **Users and Roles**.
2. On the **Users** tab, click **Edit** in the actions menu of the user.
3. In **Permission Label**, type a label and press Enter. Do this for each label of the user.
4. Click **Save**.
5. Ask the user to sign out and sign in again. The labels apply at the next sign-in.

**Expected result:** the user sees the elections with these labels and the elections without a
label.

:::caution CAUTION
Type the labels exactly the same in the election and in the user. A difference in one letter
hides the election from the user.
:::

## If there is a problem

| Problem | Action |
| --- | --- |
| "You don't have permission to access this election." | Give the user the label of the election, or remove the label from the election. |
| "Cannot create keys ceremony: one or more permission labels are missing." | Give your account the labels of the elections of the ceremony. |
| "Some elections don't have the required permission label or are not published" | Select only published elections with your labels for the tally. |
| **Permission Label** is not in the form. | Your role does not have **Edit Permission Label**, or the user profile has no label attribute. |

## Related pages

- [Election: Data](../Reference/User-Manual/Election-Management/Election/Data/election_management_election_data.md)
- [Users](../Reference/User-Manual/Users-and-Roles/Users/admin_portal_reference_user-manual_users-and-roles_users.md)
- [Permissions](../Reference/User-Manual/Users-and-Roles/Permissions/admin_portal_reference_user-manual_users-and-roles_permissions.md)
- [Run the Key Ceremony](../procedures/04-keys.md)
