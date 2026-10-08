---
id: value_to_template_association
title: Value to Template Association
description: "This tutorial explains how the platform finds the template of a document, and how the values of the election get into the template. Read it when a document does not use the template that you expect."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This tutorial explains how the platform finds the template of a document, and how the values
of the election get into the template. Read it when a document does not use the template that
you expect.

## How the platform selects the template

A template is not used only because it exists. The platform links a document to a template
through a row of the **Reports** tab:

1. The platform needs a document of a type, for example **Electoral Results**, for an election
   event and, if there is one, an election.
2. It looks on the **Reports** tab of the election event for a report of that type. If the
   document is for one election, the report must be for that election.
3. If it finds a report with a **Template**, it uses the template with that alias from the
   **Templates** page.
4. If it finds no report, or the report has no template, it uses the default template of the
   type.

The platform uses the first report that matches. Make only one report of each type for each
election, so that the result is clear.

| What you have | Template that the platform uses |
| --- | --- |
| No report of the type | The default template. |
| A report of the type without a **Template** | The default template. |
| A report of the type with a **Template** | Your template. |
| A report for another election | The default template, for documents of the other elections. |

## How the values get into the template

The texts of a template contain variables between double braces. When the platform makes the
document, it replaces each variable with a value of the election event, for example a name, a
date or a number of votes. The default template of each type shows the variables of that type.

To see the result with sample values, click **Preview** in the actions menu of the report on
the **Reports** tab. Preview uses sample data, not the real data.

## Check the association

1. Open the election event.
2. Click the **Reports** tab.
3. Find the report of the document type. Check the **Election** and the **Template** columns.
4. Click **Preview** in the actions menu of the report.
5. Make sure that the document has the text of your template.

## If there is a problem

| Problem | Action |
| --- | --- |
| The document has the default text. | Make or change the report of that type on the **Reports** tab. Select your template in **Template**. |
| Your template is not in the **Template** list of the report. | The **Type** of the template is not the type of the report. Change the template. |
| The document shows an error or an empty value. | A variable of the template is not correct. Compare it with the default template of the type. |

## Related pages

- [Reports and Templates](reports_and_templates.md)
- [Templates](../../Reference/User-Manual/Templates/admin_portal_reference_user-manual_templates.md)
- [Election event: Reports](../../Reference/User-Manual/Election-Management/Election-Event/Reports/election_management_election-event_reports.md)
