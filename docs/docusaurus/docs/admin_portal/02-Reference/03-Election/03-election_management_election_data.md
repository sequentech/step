---
id: election_management_election_data
title: Data
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## 1. General

Manage data related to the Election. This tab allows configuration of core settings for an individual Election, similar in structure to the Election Event Data tab but scoped to this specific Election.

- **General**: Set up basic details and multilingual names.
- **Dates**: Configure start and end dates/times.
- **Language**: Define supported and default languages.
- **Voting Channels Allowed**: Specify applicable voting methods.
- **Ballot Design**: Manage presentation and ordering of contests and audit options.
- **Receipts**: Configure how voters receive confirmation or receipts.
- **Image**: Upload an image associated with this Election.
- **Advanced Configuration**: Import advanced settings and define vote limits, confirmations, and permissions.

More information on each section is available below.

---

## General

Setup basic details and configure multilingual names for this Election. This mirrors the Election Event “General” section but applies only to the current Election.

- **Languages Tabs**: Configure how this Election appears in different languages in the Voting Portal.
- **Name**: Enter the official name of the Election.
- **Alias (optional)**: Specify an internal alias for display in side menus.
- **Description (optional)**: Provide a description to clarify the purpose or scope of this Election.

---

## Dates

Set the start and end dates for your Election.

- **Start Date & Time**: Commence the voting period for this Election.
- **End Date & Time**: Conclude the voting period for this Election.
- Use the calendar/date-time pickers to select the exact timestamps.

---

## Language

Manage language options for this Election. Similar to Election Event, but scoped per Election.

- **Define Languages**: Select which languages are enabled for this Election via radio buttons.
- **Set Default Language**: Mark one language as default by selecting “Default” next to it.
- Note: Available languages here are constrained by the Election Event’s language settings.

---

## Ballot Design

Manage presentation of ballot elements for this Election.

- **Audit Button Display Options**: Toggle whether the “Audit Ballot” button appears in the Voting Portal.
- **Presentation Contests Order**:  
  - **Random**: Contests appear in a random order for each voter session.  
  - **Alphabetical**: Contests are listed alphabetically by name.  
  - **Custom**: Manually reorder contests. When “Custom” is selected, a drag-and-drop or similar interface appears to arrange contests in the desired sequence.
- **Additional Design Settings** (if applicable):  
  - Logo, CSS overrides, or other display options (depending on system capabilities).  
  - Receipt format settings may be covered under “Receipts” if separate.

---

## Voting Channels Allowed

Specify which voting channels (methods) are applicable for this Election.

- Use radio buttons or checkboxes to enable/disable channels (e.g., Online, Paper, Telephone, Postal).
- Only the selected channels will accept votes in this Election.

---

## Receipts

Configure how voters receive confirmation of their vote for this Election.

- **Receipt Method**: Choose whether voters receive an on-screen confirmation, email receipt, SMS receipt, or a combination (depending on system capabilities).
- **Audit Support**: Enable or disable voter’s ability to verify encryption (if separate from Ballot Design audit button).
- **Customization**: If system allows, configure templates or text shown on receipts.

---

## Image

Upload an image to be displayed within the system for this Election (e.g., a logo or banner).

- **Upload**: Drag and drop or select a `.png` (or supported format) file.
- **Preview**: After upload, preview how the image appears in the portal or admin interface.
- **Optional**: If no image is needed, this section can be left blank.

---

## Advanced Configuration

Import advanced settings for this Election and define vote limits, confirmations, and permission labels.

- **Cast Vote Confirmation Modal**:  
  - Toggle display of a confirmation dialog before finalizing a vote.  
  - If enabled, configure the confirmation text or warning message.

- **Number of Allowed Votes**:  
  - Specify how many times a voter may revote (e.g., allow re-submissions up to N times).  
  - If unlimited revotes are not permitted, set the maximum count here.

- **Permission Label**:  
  - Assign a permission label to this Election, controlling which Admin Portal users can access/manage this Election.  
  - Example workflow:  
    1. Create or select a permission label (e.g., “Election A Manager”).  
    2. Associate one or more admin users with this label.  
    3. Only users with this permission label see or modify this Election in the Admin Portal.

- **Upload Advanced Configuration** (Optional):  
  - Drag and drop a configuration file (e.g., JSON or system-specific format) to apply pre-defined advanced settings.  
  - This may include settings like contest encryption policies, custom validations, or integrations.
  
- **Start Screen Title Policy**

  - Set the title of the Start Screen (Voting Portal) to either the election title or the Election Event Title. The default value is the Election title.

---

### Notes & Best Practices

- Many settings here depend on or inherit from Election Event configurations (e.g., available languages, global voting channels). Always verify consistency between the Election Event and individual Election settings.
- Before enabling advanced options (e.g., revote limits, permission labels), ensure you understand their impact on voter experience and security.
- Use descriptive names and aliases to help administrators quickly identify Elections in multi-election events.
- Test changes in a staging environment when possible, especially for advanced configurations or ballot design modifications.
