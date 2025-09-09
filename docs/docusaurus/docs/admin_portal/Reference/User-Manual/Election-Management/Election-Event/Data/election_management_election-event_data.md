---
id: election_management_election_event_data
title: Data
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


The Data tab is similar across multiple entities in the system (Election Events, Elections, Contests, and Candidates). In this tab, you can configure the main values of each entity. Specifically for Election Events, all related data can be managed here.

## Sections in the Election Event Data Tab

Each section serves a specific purpose and provides a comprehensive breakdown of information:

- **General**: Includes basic details.
- **Dates**: Start and End dates of the election event.
- **Language**: Supported languages for this event.
- **Ballot Design**: Custom ballot features including design, logos, links, and more.
- **Voting Channels Allowed**: Applicable voting methods.
- **Custom URLs Prefix**: Define custom URLs for the Voting / Enrollment portals and SAML endpoint.
- **Support Materials**: Documents available in the Voting Portal for voters to review.
- **Advanced Configurations**: Enable system lockdown, Voting Portal session timeout, and forced logout.

Detailed descriptions of each section are provided below.

---

## General

Set up basic details and configure multilingual names for your Election Event.

- **Languages Tabs**: Configure how your Election Event appears in different languages in the Voting Portal.
- **Name**: Enter the official name of your Election Event.
- **Alias (optional)**: Internal alias used only in the system's side menu.
- **Description (optional)**: Provide a description for your Election Event.

## Language

Manage language options for your Election Event. The selected languages will be available for elections within this event.

- Use radio buttons to select the languages available.
- Set the default language by selecting **Default** next to the appropriate language.

## Ballot Design

Manage how the ballot appears in the Voting Portal.

- **Disable Ballot Audit Support**: Enable or disable the ability for voters to verify ballot encryption.
- **Skip Election List Screen**: Skip election selection in the portal.
- **Show User Profile**: Show user profile information in the Voting Portal.
- **Show Cast Vote Logs Tab**: Policy to enable the CastVote Immutable logs in the Ballot Locator.
- **Logo URL (optional)**: Provide a link to a logo to display.
- **Redirect Finish URL (optional)**: Redirect users to a URL after completing voting.
- **Custom CSS**: Apply custom styles to the ballot design.

## Voting Channels Allowed

Define the voting methods available for this Election Event.

- Use radio buttons to enable applicable voting channels.

## Custom URLs Prefix

Create custom URL prefixes for the Voting and Enrollment portals, and SAML endpoint.

- Input the desired prefix for each endpoint.

**Examples:**

- Input "myelection" into **Login**:  
  URL becomes `https://myelection.sequent.vote`
- Input "enrollment" into **Enrollment**:  
  URL becomes `https://enrollment.sequent.vote`

## Support Materials

Provide documents that voters can access in the Voting Portal.

- **Support Materials Activated**: Enable or disable additional support documents.
- **Add**: Attach documents with the following fields:
  - Title
  - Subtitle
  - **Is Hidden**: Controls visibility in the portal
  - Drag and drop the file
  - Save

## Advanced Configuration

Configure advanced system behaviors for this Election Event.

- **Contest Encryption Policy**:
  - **Single Contests**: Encrypt contests individually.
  - **Multiple Contests**: Encrypt multiple contests together to enable ballot-level audit.
- **Lockdown Status**: When enabled, no changes can be made to this Election Event. This action is irreversible.
- **Voting Portal Countdown Policy**:
  - Define the session timeout duration in seconds.
  - Configure the countdown warning and logout alert thresholds.
- **Keys/Tally Ceremonies Policy**:
  - Allow for the automatic generation of keys and tallies, eliminating the need for trustees involvement.
