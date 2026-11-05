---
id: import_format_templates
title: Templates File
sidebar_position: 6
description: "The format of the templates file: the ten columns, the JSON of the template, the allowed types and communication methods, and what the import does."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The templates file holds the templates of a tenant: the documents, emails and SMS messages that
the platform makes for reports, credentials, ballot receipts and other uses. You make the file
with **Export** on the **Templates** page. You import it with **Import** on the same page. Do not
make the file by hand.

Use the import to copy the templates to another tenant, or to restore a backup. For the screen,
see [Templates](../02-election-event/13-02-election_management_election-event_templates.md#exporting-and-importing-templates).

:::caution CAUTION: tenant in the templates file
The import reads the tenant ID from each row. It does not use the tenant that you are signed in
to. Import the file only into the tenant where you exported it.
:::

## Fields

The file has a header row and ten columns. The order of the columns is fixed. The import reads
the columns by position, not by name.

| Column | Position | Required | Allowed values or format | Default | Meaning | Example |
| --- | --- | --- | --- | --- | --- | --- |
| `alias` | 1 | Required | Text | None | The alias of the template. Reports and other functions find the template by its alias. | `ballot-receipt-1` |
| `tenant_id` | 2 | Required | A version 4 UUID | None | The tenant of the template. A row with a value that is not valid is skipped. | `90505c8a-23a9-4cdf-a26b-4e19f6a097d5` |
| `template` | 3 | Required | A JSON object, see below | `null` | The content of the template. | See below. |
| `created_by` | 4 | Required | Text | None | The user that made the template. | `admin@example.com` |
| `labels` | 5 | Optional | Text (JSON) | Empty | Extra data. You can leave it empty. | `{}` |
| `annotations` | 6 | Optional | Text (JSON) | Empty | Extra data. You can leave it empty. | `{}` |
| `created_at` | 7 | Optional | Date and time | Not used | The import ignores the value. It sets the date of the import. | `2026-10-01 09:30:00 +00:00` |
| `updated_at` | 8 | Optional | Date and time | Not used | The import ignores the value. It sets the date of the import. | `2026-10-01 09:30:00 +00:00` |
| `communication_method` | 9 | Required | `EMAIL`, `SMS` or `DOCUMENT` | None | The communication method of the template. | `EMAIL` |
| `type` | 10 | Required | One of the types in the table below | None | The **Template Type**. | `BALLOT_RECEIPT` |

### Values of `type`

| Value | Label in the admin portal |
| --- | --- |
| `CREDENTIALS` | Credentials |
| `INITIALIZATION_REPORT` | Initialization Report |
| `ELECTORAL_RESULTS` | Electoral Results |
| `BALLOT_IMAGES` | Ballot Images |
| `BALLOT_RECEIPT` | Ballot Receipt |
| `ACTIVITY_LOGS` | Activity Logs |
| `MANUAL_VERIFICATION` | Manual Verification |
| `PARTICIPATION_REPORT` | Participation Report |

The import does not check the value of `type`. Use only the values in the table.

The import does not check `type` or `communication_method`. A value that is not in the tables above is imported, but the admin portal does not use the template.

### Values of `communication_method`

| Value | Meaning |
| --- | --- |
| `EMAIL` | The template makes an email. |
| `SMS` | The template makes an SMS message. |
| `DOCUMENT` | The template makes a document (PDF). |

When you make a template in the admin portal, the admin portal writes `EMAIL` in this column,
also if you select other methods. The methods that you select are in the JSON of the template
(`selected_methods`).


### The column `template`

`template` is a JSON object. These are the keys that the admin portal writes:

| Key | Type | Meaning |
| --- | --- | --- |
| `alias` | Text | The alias of the template. It is the same as the column `alias`. |
| `name` | Text | The name of the template. |
| `selected_methods` | Object | Three keys, `EMAIL`, `SMS` and `DOCUMENT`. Each value is `true` or `false`. It shows which parts the template has. |
| `email` | Object | The email part. It has `subject`, `plaintext_body` and `html_body`. |
| `sms` | Object | The SMS part. It has the key `message`. |
| `document` | Text | The document part: the text of the document, as an HTML template. |
| `pdf_options` | Object | The PDF settings of the document, for example `landscape`, `print_background`, `scale`, `paper_width`, `paper_height`, `margin_top`, `margin_bottom`, `margin_left`, `margin_right`, `page_ranges`, `header_template`, `footer_template`. |
| `report_options` | Object | The report limits. It has `max_items_per_report` and `max_threads`. |

The texts of the email, the SMS and the document can contain variables. The platform replaces them
when it makes the message or the document.

For a message to a voter, the platform fills these variables: `user.first_name`, `user.last_name`, `user.username`, `user.email`, `user.attributes`, `tenant_id`, and each attribute of the voter as `user.<attribute name>`. If the message belongs to an election event, it also gets `election_event.id`, `election_event.name` and `vote_url`.

Example of the value of `template`:

```json
{
  "alias": "ballot-receipt-1",
  "name": "Ballot receipt 1",
  "selected_methods": {"EMAIL": true, "SMS": false, "DOCUMENT": true},
  "email": {"subject": "Your ballot receipt", "plaintext_body": "Your receipt is attached.", "html_body": "<p>Your receipt is attached.</p>"},
  "sms": {"message": ""},
  "document": "<html><body><h1>Receipt</h1></body></html>",
  "pdf_options": {"landscape": false, "print_background": true},
  "report_options": {"max_items_per_report": 1000, "max_threads": 4}
}
```

## Rules

| Rule | Value |
| --- | --- |
| File type | `.csv`. The separator is a comma. |
| Header | The export writes a header row. The import does not skip it. It skips it because the value `tenant_id` is not a valid UUID. A file without a header row works. |
| Encoding | UTF-8. |
| Quotes | The JSON columns are in double quotes. Each quote inside is doubled. Do not change this. |
| Order of columns | Fixed. |
| Duplicate aliases | The import does not check them. If a template with the same alias exists, the import adds a second one. |
| Size | The size is limited by the deployment setting for file uploads. |
| Empty export | If the tenant has no templates, the export fails with the error `No templates available to write`. |

:::caution CAUTION: template aliases
Use a different alias for each template. If two templates have the same alias, the admin portal can use either of them.
:::

## Example file

```csv
alias,tenant_id,template,created_by,labels,annotations,created_at,updated_at,communication_method,type
ballot-receipt-1,90505c8a-23a9-4cdf-a26b-4e19f6a097d5,"{""alias"":""ballot-receipt-1"",""name"":""Ballot receipt 1"",""selected_methods"":{""EMAIL"":false,""SMS"":false,""DOCUMENT"":true},""document"":""<html><body><h1>Receipt</h1></body></html>""}",admin@example.com,{},{},2026-10-01 09:30:00 +00:00,2026-10-01 09:30:00 +00:00,DOCUMENT,BALLOT_RECEIPT
```

## What the import does

1. The admin portal uploads the file and asks for the **Integrity Check (SHA-256)**. The task
   checks the hash if you typed one.
2. The task **Import Templates** starts. Look at the task widget or the **Tasks** tab for the
   result. The task does not try again if it fails.
3. For each row, the task reads the ten columns. It skips a row if `tenant_id` is not a valid UUID.
4. The task adds all the other rows in one database transaction. If the database refuses one row,
   the task adds nothing and the status is **Failed**.
5. Each row becomes a new template. The import never changes or removes a template that exists.
6. The dates `created_at` and `updated_at` are the date of the import.

If the JSON in `template` is not valid, the import saves `null` and does not show an error. The
template then does not work. Open the template in the admin portal to check it.

Known effect: the import stores `labels` and `annotations` as a JSON text and not as a JSON object.


### Messages and errors

| Message | Where | Meaning and action |
| --- | --- | --- |
| `Failed to verify the integrity: Hash of voters file: ... does not match with the input hash: ...` | Task log | The hash you typed is not the hash of the file. (The message says "voters file" for this file too.) |
| `Error reading CSV record: ...` | Task log | A row has wrong quotes. Export the file again. |
| `Error inserting template: ...` | Task log | The database refused a row. |
| `document not found` | Task log | The upload failed. Upload the file again. |

## Related pages

- [Templates](../02-election-event/13-02-election_management_election-event_templates.md): the **Templates** page, **Import** and **Export**.
- [Reports](../02-election-event/13-01-election_management_election-event_reports.md): reports use a template.
- [Import File Formats](index.md): all import files.
