---
id: election_management_election_event_reports
title: Reports
description: "The Reports tab makes documents about the election event, for example the electoral results, a statistical report or the activity log."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Reports** tab makes documents about the election event, for example the electoral
results, a statistical report or the activity log. Each row of the list is a report
configuration: a type, an election, a template and a policy. You generate the document from the
row. The header of the tab says "Generate reports for the election events".

## Open the tab

1. In the menu on the left, click the election event.
2. Click the **Reports** tab.

The tab shows with the **View Election Event Reports** permission. It stays visible when the
election event is locked down. The actions need these permissions:

| Action | Permission |
| --- | --- |
| Create | **Create Report** |
| Edit | **Edit Reports** |
| Delete | **Delete Report** |
| Generate | **Generate Report** |
| Preview | **Preview Report** |
| Select the columns | **Election Event Reports Columns** |

## List

When the list is empty, the tab shows "No Reports yet." and **Create Report**. When there are
reports, the toolbar has **Add** and **Add filter**.

| Column | Content |
| --- | --- |
| **Report Type** | The type of the report. |
| **Template** | The alias of the template, if any. |
| **Election** | The election of the report, if any. |
| Encryption | An icon that shows if the report file is encrypted. |
| **Actions** | **Edit**, **Delete**, **Generate** and **Preview**. |

## Report form

| Field | Use |
| --- | --- |
| **Type** | The type of the report, for example **Electoral Results**, **Statistical Report**, **Activity Logs**, **Audit Logs**, **Ballot Receipt** or **Initialization Report**. |
| **Election** | The election of the report. Some types require an election. Other types do not permit one. |
| **Template** | The template of the document. If you do not select one, the platform uses its default template for the type. Some types require a template. |
| **Permission Label** | The permission label of the report. |
| **Repeatable** | Makes the report again on a schedule. It shows only for the types that permit it. |
| **Cron Expression** | The schedule of a repeatable report. |
| **Email Recipients** | The email addresses that get the repeatable report. Type an address and press Enter. |
| **Encryption Policy** | **Unencrypted** or **Configured Password**. |
| **Save** | Saves the report configuration. |

When you select **Configured Password**, a window asks for the password. Type it in
**Password:** and in **Repeat Password:**, then click **Save Password**. You cannot change the
policy of the report after you save a password.

## Actions

| Action | Result |
| --- | --- |
| **Generate** | Makes the document from the real data. The task "Generate Report" makes the file. Download it with **Download File**. |
| **Preview** | Makes an example of the document with sample data. Use it to check a template. |
| **Edit** | Opens the report form. |
| **Delete** | Deletes the report configuration after the question "Are you sure you want delete this Report?". |

For an encrypted report, the tab shows "How to decrypt the file" with the command options
`-in`, `-out` and `-pass`.

## If there is a problem

| Problem | Action |
| --- | --- |
| "Error creating Report" | Check the required fields, then save again. |
| "Password and confirm password do not match. Please ensure both fields contain the same password." | Type the same password in both fields. |
| "Error setting up report encryption" | Set the password again. |
| The document does not use your template. | Check that the report row has the template. See [Value to Template Association](../../../../../Tutorials/Reports-and-Templates/value_to_template_association.md). |

## Related pages

- [Reports and Templates](../../../../../Tutorials/Reports-and-Templates/reports_and_templates.md)
- [Templates](../../../Templates/admin_portal_reference_user-manual_templates.md)
- [Get the Results](../../../../../procedures/07-results.md#3-make-the-reports)
- [Tasks](../Tasks/election_management_election-event_tasks.md)
