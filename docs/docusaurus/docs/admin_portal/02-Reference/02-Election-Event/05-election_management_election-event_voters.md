---
id: election_management_election_event_voters
title: Voters
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


This section displays the currently configured voters for this Election Event.

### Overview

This tab displays and manages voters. The table lists all currently configured voters.

- **Columns**: Enable/Disable which columns are displayed in the table.
- **Add Filter**: Create a text filter by column.
- **Add**: Provide a voter’s information and add them to the Election Event.
- **Import**: Import voters using a CSV file.
- **Export**: Export the list of voters as a CSV file.
- **Send**: Message voters using the **Send Notification** tab.
- **Custom Filters**: Use preset custom filters.
  - These filters can be defined under:  
    `Election Event > Data > Advanced Configuration`

---

### Actions

The **Actions** column provides options to interact with voter records:

- **Send**: Send a notification to the voter.
- **Edit**: Modify voter details or change their assigned Area.
- **Delete**: Remove the voter from the system.
- **Manually Verify**: Confirm voter's identity (see below).
- **Change Password**: Update the voter's password (see below).
- **User’s Logs**: View actions performed by the voter.

---

#### Send Notifications

Use this tab to send notifications to voters through various methods, schedules, and templates.

- **Audience**: Define who the notification is for.
- **Schedule**: Set when the notification will be sent.
- **Communication Template**: Choose a preset message template.
- **Communication Method**: Email / SMS.
- **Communication Type**: Type of content (if applicable).
- **Template Alias**: Name of the preset template.
- **Email Subject**: Only applicable for email.
- **Message Body**: Plain or rich text to be sent.

---

#### Manual Verification

Confirm the voter's identity without requiring additional verification steps.

- A popup dialog will appear.
- Scanning the generated downloadable QR Code allows the voter to:
  - Set their password
  - Verify themselves
  - Bypass the KYC process
- Once complete, the voter will be eligible to vote.

---

#### Change Password

Change the voter's password.

- **Password and Repeat Password** must match.
- Enabling the **Temporary** radio button will require the voter to change their password on next login.

---

#### User’s Log

View logs of all actions performed by the voter.

### Additional User Data Fields

You can add additional fields like the voter's birthday or sex. [Check out the tutorial for that](../../03-Tutorials/06-admin_portal_tutorials_add-user-attributes-to-keycloak.md).

**Important:** Additional attributes for voters must be added before the enrollment process. For instance, if the sex attribute is not added, this trait will not be reflected in the reports and statistics.
