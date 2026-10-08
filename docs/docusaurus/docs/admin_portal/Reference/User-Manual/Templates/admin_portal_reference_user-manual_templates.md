---
id: admin_portal_reference_user_manual_templates
title: Templates
description: "The Templates page holds the templates of the tenant. A template is the text of a message (email or text message) or of a document (for example a report or a receipt)."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Templates** page holds the templates of the tenant. A template is the text of a message
(email or text message) or of a document (for example a report or a receipt). The platform uses
a template when it sends a message to voters or makes a document. If there is no template for a
document, the platform uses its default template.

## Open the page

In the menu on the left, click **Templates**. The page has the header **Templates** and "List
of templates".

The **Templates** menu item needs the **View templates** permission. Without it, the page shows
"You don't have permission to access templates.". To create and change templates, your role
needs **Edit Communication Template**.

## List

When the list is empty, the page shows "No Template Yet" with **Create Template** and
**Import**. When there are templates, the toolbar has **Add**, **Import** and **Export**.

| Column | Content |
| --- | --- |
| **Alias** | The unique short name of the template. Other screens select the template by this alias. |
| **Name** | The name of the template. |
| **Type** | The type of the template. |
| **Actions** | Edit and delete icons. |

## Template form

The form is **Create a Template** or **Edit a Template**.

| Field | Use |
| --- | --- |
| **Template Alias** | The unique short name of the template. |
| **Template Name** | The name of the template. |
| **Type** | What the template is for, for example **Credentials**, **Ballot Receipt**, **Electoral Results**, **Statistical Report**, **Initialization Report**, **Activity Logs**, **Audit Logs** or **Manually verify voter**. |
| **Choose Methods** | **Email**, **SMS** and **Document**. Select one or more. Each method adds its own section. |
| **Email** | **Email Subject**, and the body in **Rich Text Body** and **Plain Text Body**. |
| **SMS Message** | The text of the text message. |
| **Document** | The body of the document. |
| **PDF Options** | The options of the PDF file, in JSON format. |
| **Report Options** | The options of the report, in JSON format. |
| **Save** | Saves the template. |

The texts can contain variables in double braces, for example `{{...}}`. The platform replaces
each variable with its value when it makes the message or the document. Use **Preview** on the
**Reports** tab of an election event to check a document template with sample data.

## Create a template

1. Click **Add**. If the list is empty, click **Create Template**.
2. Type the **Template Alias** and the **Template Name**.
3. Select the **Type**.
4. In **Choose Methods**, select the methods.
5. Type the texts of each method.
6. Click **Save**.

**Expected result:** the message "Template created" shows.

## Import templates

1. Click **Import**. The **Import Templates** panel opens.
2. Type the SHA-256 hash of the file in **Integrity Check (SHA-256)**.
3. Upload the file and click **Import**.

## If there is a problem

| Problem | Action |
| --- | --- |
| "Error creating Template" or "Error updating Template" | Check the alias and the required fields, then save again. |
| "Hashes don't match. Integrity check failure." | The file is not the file that you expect. Get the file again from its source. |
| A document does not use your template. | Select the template in a report of the **Reports** tab. See [Value to Template Association](../../../Tutorials/Reports-and-Templates/value_to_template_association.md). |

## Related pages

- [Reports and Templates](../../../Tutorials/Reports-and-Templates/reports_and_templates.md)
- [Election event: Reports](../Election-Management/Election-Event/Reports/election_management_election-event_reports.md)
- [Election event: Voters](../Election-Management/Election-Event/Voters/election_management_election-event_voters.md)
- [Templates (Deprecated)](../Settings/templates-deprecated/admin_portal_reference_user-manual_settings_templates-deprecated.md)
