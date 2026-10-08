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

### Voters CSV

**Import** reads a CSV file, or a tab-separated one if its name ends in `.tsv`. The first row names
the columns. **Export** writes the same format, and an election event export that includes voters
carries the same file, so either can be edited and imported again into this or another election
event.

| Column | Meaning |
| --- | --- |
| `id` | Written by export. Import ignores it and gives each voter a new ID. |
| `username`, `email`, `first_name`, `last_name` | The voter's account. Without a `username` column each voter gets a random one. |
| `enabled`, `email_verified` | `true` or `false`; `true` if blank or missing. |
| `area_name` | The voter's area, spelled exactly as in the **Areas** tab. A name that matches no area leaves the voter without one. |
| `authorized-election-ids` | The elections the voter may vote in, separated by `\|`. See below. |
| `password` | A password for the voter. |
| `hashed_password`, `password_salt`, `num_of_iterations` | Instead of `password`, a base64 PBKDF2-SHA256 hash and its salt. `num_of_iterations` is 27500 if missing. |
| `group_name` | The voter's group; `voter` if missing. |
| `vote-weight`, `delegate-vote-to` | See the weighted and delegated voting policies under **Data > Advanced Configuration**. |
| Any other column | Stored as the voter attribute of the same name. |

Column names may only contain letters, digits, `.`, `_` and `-`, and two names that differ only in
case or in those three characters are rejected as duplicates.

In `authorized-election-ids`, name each election by its external ID, or by its ID if it has no
external ID. An election's ID is also accepted when it has an external ID, and the external ID is
stored instead. An election is named and stored by its ID when another election shares its external
ID, or when the external ID starts or ends with a space, contains `|` or starts with `"`. It is also
stored by its ID when its external ID is longer than 255 characters, the most a voter attribute
holds, or starts with `=`, `+`, `-` or `@`, which spreadsheets read as a formula. If an election's
ID is another election's external ID, that value names the other election, as in the token mapper,
so the first one can only be named and stored by its own external ID, and not at all if it has none
or if it would be stored by its ID. A blank cell leaves the voter unrestricted: they can vote in the
elections of their area. A value that names no election in the election event, or more than one, or
an election that cannot be stored, or that starts with `"`, is rejected, with its row and the value,
and nothing is imported.

Export writes `authorized-election-ids` the same way, and adds one column per election, named
`election__` followed by the value that names the election, holding when the voter last voted in it.
Import ignores these columns. A stored value that names no election, or more than one, is exported
in double quotes, so that importing it into any election event fails instead of giving the voter
other elections.

When an election event export is imported, its elections get new IDs, and so do external IDs shaped
like an ID. Voters whose `authorized-election-ids` name elections by the values they had in the
exported event keep those elections.

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

#### Adding and editing a voter

Fields are checked as they are left. One that is required and left empty, or whose value breaks a
limit configured for that attribute, is marked and explains what it expects, and the voter cannot be
saved until it is corrected. Where an attribute sets a limit worth knowing in advance, it is stated
under the field before it is reached.

If a voter still cannot be saved, the editor stays open with the values as entered, so nothing has to
be typed again, and reports why. Where the rejection concerns particular fields, each one is named
along with the rule it broke — up to ten of them, with any beyond that counted rather than listed.

Limits come from the attribute's configuration in Keycloak; see
[Adding User Attributes to Keycloak](../../01-tutorials/99-admin_portal_tutorials_add-user-attributes-to-keycloak.md)
for how they are set and which ones the portal states up front.

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

You can add additional fields like the voter's birthday or sex. [Check out the tutorial for that](../../01-tutorials/99-admin_portal_tutorials_add-user-attributes-to-keycloak.md).

**Important:** Additional attributes for voters must be added before the enrollment process. For instance, if the sex attribute is not added, this trait will not be reflected in the reports and statistics.

### External system (i.e. Datafix) voter-list reconciliation

Users with the `election-event-voter-list-reconciliation` permission can open
the reconciliation wizard from the Voters tab. Run reconciliation only during
an agreed external-system freeze window:

1. Upload the complete external-system reconciliation CSV. The first line must
   be the `#META` line and file channels must be uppercase.
2. Review the external-system and Sequent tables. If the external-system table
   is non-empty, download its patch, have the external system apply it, and
   upload the newly generated reconciliation file. Never apply the Sequent
   side first.
3. When the external-system table is empty, review the category totals and
   explicitly apply the Sequent changes.
4. Review the row-failure table. A completed apply can contain rows that were
   safely rejected; these are business-level reconciliation results, not a
   failed background task. Resolve them and retry the same `Sequence`.
5. Upload the last file again as a diff-only convergence check. Both tables
   must be empty before the round is considered converged.
6. Compare the source and patch hashes with the electoral logs before ending
   the freeze.

An Internet ballot that is still `in-progress` is deliberately reported as a
row failure. During a hard-down external-system freeze it cannot resolve
because the review beat must reach the external system. If all remaining
failures have this cause, complete the freeze hash checks, restore
external-system connectivity, wait for the review beat to resolve the
ballots, and retry the same `Sequence`. Apply-time snapshot validation
prevents that retry from overwriting voter data changed in the meantime.

Both the real-time `/unmark-voted` operation and file reconciliation clear the
voted-channel marker. They only re-enable a voter when the corresponding
`MarkVoted` operation owns the disable. An independent administrator disable
and its comment are preserved, but the channel is still cleared so the voter
does not remain permanently blocked in an unresolved external-system state.
The administrator can then decide separately whether the account should be
re-enabled.

The task result shows at most the first 1,000 apply-time row-failure details
and always reports the complete failure count. Resolve the common cause and
retry the same `Sequence` to reveal any remaining failures.

#### Capacity validation

Before production use, UAT must exercise the largest expected voter roll,
including a first synchronization where most voters are additions. For a
100,000-voter test, record the generated envelope and audit-artifact sizes,
backend peak memory, browser peak memory, wizard rendering responsiveness, and
total Keycloak apply duration. These measurements determine the operational
batch size and freeze-window duration for that deployment.
