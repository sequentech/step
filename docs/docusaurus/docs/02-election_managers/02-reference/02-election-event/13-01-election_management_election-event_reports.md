---
id: election_management_election_event_reports
title: Reports
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


The Reports tab allows for the configuration, generation, and scheduling of reports using predefined Templates (see the Templates section for details). Some reports (e.g., Initialization Report, Manual Verification, Ballot Receipt) are triggered automatically by system actions. Other reports (e.g., Statistical Report, Voter List Summary) can be scheduled or generated manually as needed. This tab provides full control over report configuration and scheduling, ensuring flexibility and automation where required.

### Adding a Report Configuration

1. Select **Add**.
2. Select a **Report Type**.  
   - This determines the kind of data to be generated.  
   - Example: **Statistical Report** generates statistical data about Tally results.
3. Select an **Election** from which data will be gathered.  
   - If left blank, the configuration applies to ALL Elections.
4. Select the appropriate **Template** (optional).  
   - If left blank, a default template designed by Sequent will be used.  
   - Templates control how report data is displayed (HTML code). Templates are associated with Report Types in the Templates tab.
5. Add the appropriate **Permission Label** for the report.  
   - This determines which users can see, generate, and preview the report.  
   - You can only choose a label that your own user carries (see [Assigning a Permission Label](../03-election/03-election_management_election_data.md#assigning-a-permission-label)). Otherwise the report is not saved and the Admin Portal shows **Error submitting Report**.
6. Select **Save**.

### Generating and previewing

**Preview** draws the report's template with sample data and needs the
`report-preview` permission. **Generate** reads the event's data and needs
`report-generate`. Both need `report-read`. A report is generated only in the
tenant and election event it belongs to.

### List of Voters who Actually Voted

This report lists the voters who cast an accepted ballot, for every Post
(election) of the event or for the one Post the report names. One generation
writes a PDF and a CSV of the same voters.

1. Select **Add** and choose **List of Voters who Actually Voted** as the
   **Report Type**.
2. Leave **Election** blank to list **All Posts**, or choose one Post.
3. Select **Save**, then **Generate** in the report's row menu.
4. When the task finishes, download the result. A report configured with both
   formats gives a ZIP with the PDF and the CSV.

Generating it needs `voter-read` besides the report permissions, since it names
voters. A report without a Post needs every Post's permission label.

What the list contains:

- **Who voted, not when or how.** For each Post and each of its areas: number,
  name (last name, first name, suffix, middle name) and username. It never
  shows a ballot time, channel, ballot ID or choice. The 2025 report's *Date
  voted* column is not printed.
- **Accepted ballots only.** A voter who revoted is listed once. Ballots
  waiting for an external confirmation (pending) and rejected ballots are not
  accepted votes.
- **Per area:** voters who voted, registered voters who did not vote, and the
  total. Voters whose account is in no area of the Post close the Post's list,
  and a voter whose account no longer exists is shown as *(voter record not
  found)*.
- **Large lists** print the PDF in parts (by default 1,000 voters per part,
  the template's *Max items per report*), delivered in a ZIP. The CSV holds
  every voter.
- The list is kept in the event's documents only: it is never sent by email,
  and it is never generated on a schedule. A list report that carries a
  schedule (for example one imported from an older version) has its
  **Repeatable** setting turned off at its next run, with a failed task that
  says why.

In events whose ballots carry pseudonyms, the list is refused when a voter of a
Post has a pending or rejected ballot and no accepted one, or after the voter
identity mapping was purged: the system cannot then tell which voters cast an
accepted ballot without resolving pseudonyms. A voter with a rejected ballot
and an accepted one does not prevent the list. While voting is open, a ballot
arriving during the generation can also make it fail; generating it again
reads the ballot box anew.

Accented and other Latin, Greek and Cyrillic letters print in the PDF. The PDF
renderer has no fonts for Chinese, Japanese or Korean, so such characters print
as boxes there; the CSV keeps every character.

The list names voters, so its PDF is drawn with scripts turned off. It has no
QR code; the report hash, system hash, version and the user who generated it
are printed as text. A custom template from the 2025 version that draws a QR
code with a script prints an empty space there.

The CSV is UTF-8 with a header row, quoted as RFC 4180. Its columns are
`election_id`, `election_external_id`, `election`, `post`, `area_id`, `area`,
`number`, `last_name`, `first_name`, `middle_name`, `suffix`, `username` and
`record` (`found` or `missing`). A cell that starts with `=`, `+`, `-` or `@`
is written after a `'`, so a spreadsheet does not run it as a formula.

### Repeatable Reports

Enabling the **Repeatable** option allows generating and sending reports repeatedly according to a schedule (Cron expression).

1. Enable **Repeatable**.
2. Define your **Cron Expression**.  
   - See “Setting up a Cron Expression” below for guidance.
3. Set the **Email Recipients** for this report.
4. Select **Save**.

A repeatable report runs on behalf of the user who last saved it. Before each run, the system checks that this user still exists in the tenant, is enabled and has permission to read reports (`report-read`). If any of these checks fails, the report is not generated or sent, **Repeatable** is turned off and a failed task is recorded. To resume it, an authorized user enables **Repeatable** again and saves the report.

#### Setting up a Cron Expression

A cron expression is a sequence of five fields indicating when to run a task:
- **Minute** (0–59)
- **Hour** (0–23)
- **Day of Month** (1–31)
- **Month** (1–12)
- **Day of Week** (0–6, where 0 = Sunday, 6 = Saturday)

Examples:
- **Hourly Report**: `0 * * * *`  
  Runs every hour at minute 0 (e.g., 1:00, 2:00).
- **Daily Report**: `15 9 * * *`  
  Runs every day at 9:15 AM.
- **Monthly Report**: `30 10 1 * *`  
  Runs on the first day of every month at 10:30 AM.
- **Day-of-Week Report**: `0 14 * * 5`  
  Runs every Friday at 2:00 PM.

### Encrypted Reports

To protect sensitive information, reports can be encrypted with a user-chosen password. Encryption status and password are preserved during export/import, so re-encryption is not needed.

1. Click **Create Report** or **Add**.
2. Choose **Report Type**.
3. Choose **Election** (optional).
4. Choose **Template** (optional).
5. Set **Encryption Policy** to “Configured Password”.
6. Enter a password in the **Password** field.
7. Repeat the password in the **Repeat Password** field.  
   - This password will be required to decrypt all generated reports from this configuration.
8. Click **Save Password** (or **Save** depending on UI).

> **Note:** Encrypted reports retain their encryption when exported and re-imported. The same password must be used to decrypt them regardless of context.

---

With these instructions, users can configure one-off or scheduled reports, and choose to encrypt sensitive report outputs. Adjust terminology or examples according to your UI labels and workflow.

