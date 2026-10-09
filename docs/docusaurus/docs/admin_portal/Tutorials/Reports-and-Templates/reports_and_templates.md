---
id: reports_and_templates
title: Reports and Templates
description: "This tutorial makes a report document with your own template. You make the template on the Templates page, link it to a report on the Reports tab, check it with Preview, and generate the document."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This tutorial makes a report document with your own template. You make the template on the
**Templates** page, link it to a report on the **Reports** tab, check it with **Preview**, and
generate the document. The same method applies to all report types, for example
**Electoral Results** or **Statistical Report**.

## Before you start

- Your role can edit templates (**Edit Communication Template**) and create and generate
  reports (**Create Report**, **Generate Report**, **Preview Report**).
- The election event exists. For results reports, the tally is complete.

## 1. Make the template

1. In the menu on the left, click **Templates**.
2. Click **Add**. If the list is empty, click **Create Template**.
3. Type a **Template Alias**, for example `results-election-1`.
4. Type the **Template Name**.
5. In **Type**, select the type of the report, for example **Electoral Results**.
6. In **Choose Methods**, select **Document**.
7. Type the body of the document in **Document**.
8. If necessary, set **PDF Options** and **Report Options** in JSON format.
9. Click **Save**.

**Expected result:** the message "Template created" shows.

:::note
If you do not make a template, the platform uses its default template for the report type.
Make a template only when you must change the text or the layout.
:::

## 2. Link the template to a report

1. Open the election event.
2. Click the **Reports** tab.
3. Click **Add**. If the list is empty, click **Create Report**.
4. In **Type**, select the same type as the template.
5. In **Election**, select the election, if the type uses one.
6. In **Template**, select the alias of your template.
7. In **Encryption Policy**, select **Unencrypted** or **Configured Password**.
8. Click **Save**.

**Expected result:** the message "Report created successfully" shows. The report shows in the
list.

## 3. Check the template with sample data

1. In the row of the report, open the actions menu.
2. Click **Preview**.
3. Wait for the task to finish, then download the file.
4. Check the text and the layout.

If the document is not correct, change the template on the **Templates** page and preview
again.

## 4. Generate the document

1. In the row of the report, open the actions menu.
2. Click **Generate**.
3. Wait for the task "Generate Report" to finish.
4. Click **Download File**.

**Expected result:** you have the document with the real data.

## Repeatable reports

Some report types can run on a schedule. For these types, the report form shows
**Repeatable**. Turn it on, type the **Cron Expression**, and add the **Email Recipients**.
Type each address and press Enter.

## If there is a problem

| Problem | Action |
| --- | --- |
| The template is not in **Template**. | Check that the **Type** of the template is the same as the type of the report. |
| The document does not use your template. | See [Value to Template Association](value_to_template_association.md). |
| The task shows `FAILED`. | Open the task on the **Tasks** tab and read the logs. Check the variables of the template. |

## Related pages

- [Templates](../../Reference/User-Manual/Templates/admin_portal_reference_user-manual_templates.md)
- [Election event: Reports](../../Reference/User-Manual/Election-Management/Election-Event/Reports/election_management_election-event_reports.md)
- [Get the Results](../../procedures/07-results.md)
