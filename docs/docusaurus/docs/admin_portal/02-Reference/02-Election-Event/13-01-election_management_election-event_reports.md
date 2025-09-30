---
id: election_management_election_event_reports
title: Reports
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
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
6. Select **Save**.

### Repeatable Reports

Enabling the **Repeatable** option allows generating and sending reports repeatedly according to a schedule (Cron expression).

1. Enable **Repeatable**.
2. Define your **Cron Expression**.  
   - See “Setting up a Cron Expression” below for guidance.
3. Set the **Email Recipients** for this report.
4. Select **Save**.

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

