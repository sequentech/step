---
id: election_management_election_approvals
title: Approvals
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Approvals

The **Approvals** tab of an election shows the enrollment applications of voters. An election
administrator reviews each application, and then approves or rejects it. This tab works like the
**Approvals** tab of the election event. This page gives only the differences. For the columns,
the filters, the buttons and the steps, see
[Election event approvals](../02-election-event/14-election_management_election-event_approvals.md).

## Open the tab

1. In the menu on the left, open the election event.
2. Click the election.
3. Click the **Approvals** tab.

The tab shows only if your role has permission to see the approvals tab of the election. To
approve or reject an application, your role must also have permission to change applications.

## Differences from the election event tab

| Item | Election event tab | Election tab |
| --- | --- | --- |
| Applications in the list | All the applications of the election event. | If the election has a **Permission Label**, only the applications with the same permission label. If the election has no permission label, all the applications of the election event. |
| **Voters** list on the application page | The voters of the election event. | The voters of this election. |
| **Export** | All the applications of the election event. | All the applications of the election event, not only the applications in the list. |

The **Permission Label** is in the **Advanced Configuration** section of the **Data** tab of the
election.

## Main actions

| Action | Where |
| --- | --- |
| Approve an application | [Approve an application](../02-election-event/14-election_management_election-event_approvals.md#approve-an-application) |
| Reject an application | [Reject an application](../02-election-event/14-election_management_election-event_approvals.md#reject-an-application) |
| Export the applications | [Export the applications](../02-election-event/14-election_management_election-event_approvals.md#export-the-applications) |

## If there is a problem

| Problem | Action |
| --- | --- |
| You do not see the **Approvals** tab. | Ask your tenant administrator for permission to see the approvals tab of the election. |
| "You don't have permission to access this election." | The election has a **Permission Label** that your user does not have. Ask your tenant administrator. |
| An application that you expect is not in the list. | Change the **Status** filter. Make sure that the application has the permission label of the election, or open the **Approvals** tab of the election event. |

For the messages of the approve, reject, import and export actions, see
[If there is a problem](../02-election-event/14-election_management_election-event_approvals.md#if-there-is-a-problem).

## Related pages

- [Election voters](04-election_management_election_voters.md)
- [02-EVENT: Create the election event](../../03-procedures/02-event.md)
