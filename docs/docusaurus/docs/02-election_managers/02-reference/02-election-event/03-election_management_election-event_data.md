---
id: election_management_election_event_data
title: Data
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


The Data tab is similar across multiple entities in the system (Election Events, Elections, Contests, and Candidates). In this tab, you can configure the main values of each entity. Specifically for Election Events, all related data can be managed here.


## Actions buttons in the Election Event Data Tab

- **Export**: Export election event data.
- **Import candidates**: Import candidates for the election event.
- **Google meet**: Generate google meet link and create event in google calendar.


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
- **Language Detection Policy**: 
  Affects the default language in the Voting Portal.
  - **Browser Detect**: The default language will be determined by the browser.
  - **Force Default**: The default language will be the one selected as **Default**.

## Ballot Design

Manage how the ballot appears in the Voting Portal.

- **Disable Ballot Audit Support**: Enable or disable the ability for voters to verify ballot encryption.
- **Skip Election List Screen**: Skip election selection in the portal.
- **Show User Profile**: Show user profile information in the Voting Portal.
- **Show Cast Vote Logs Tab**: Policy to enable the CastVote Immutable logs in the Ballot Locator.
- **Logo URL (optional)**: Provide a link to a logo to display.
- **Redirect Finish URL (optional)**: Redirect users to a URL after completing voting.
- **Custom CSS**: Apply custom styles to the ballot design. Ballot error and warning messages expose stable CSS classes that can be targeted here — see [Styling Ballot Errors and Warnings with Custom CSS](../08-ballot-errors-custom-css.md).

## Voting Channels Allowed

Define the voting methods available for this Election Event.

- Use radio buttons to enable applicable voting channels.
- **Online**: Main remote voting channel. Starting/closing Online also governs Early Voting lifecycle (see below).
- **Kiosk**: In-person device-based voting. Its status is independent from the others.
- **EARLY_VOTING**: Enables an early voting period prior to the Online voting window for voters whose Areas have Early Voting enabled.
  - Appears only if allowed here; when started in Publish, only voters assigned to Areas with the Early Voting policy enabled can vote.
  - Online channel governs Early Voting:
    - When Online is started or closed, Early Voting will automatically close if it was enabled.
    - Early Voting cannot be started once Online voting has been started (and thereafter).
  - If a channel that is already started is later manually disallowed in this section, no immediate action is taken; action buttons remain disabled until the channel is allowed again.

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
- **Weighted Voting Policy**:
  - **Weighted Voting for Areas**: Enable weighted voting for areas.
  - **Weighted Voting for Voters**: Give each voter their own weight, so that a
    voter with weight `w` contributes `w` votes. Add a `vote-weight` column to
    the imported voters csv holding a whole number between 1 and 100000. A voter
    with no column, or with a blank cell, votes with weight 1. `vote_weight` and
    other near spellings are rejected rather than imported, because they would be
    stored under a name the tally does not read. The column is
    named after the `vote-weight` voter attribute it becomes, so an exported
    voters file can be edited and re-imported unchanged. Cannot be combined with
    Delegate Voting, and every contest in the election event must use the
    Plurality at Large counting algorithm — counting a ballot more than once
    has no defined meaning for the others, so the tally refuses to run. Decoded ballots cannot
    be included in the published results either, for the reason in the warning
    below. Check both before ballots are published: neither the counting
    algorithm nor a published ballot can be changed once voting has begun.
    The weights of everyone who votes in one area of a contest
    must also add up to no more than 1000000, so a small number of voters on very
    large weights is rejected even though each weight is individually allowed;
    this is checked when the tally runs. Turnout figures under this policy count
    voting power rather
    than voters: both the eligible-voter census and the cast-ballot total are
    sums of weights, so they will not match a headcount shown elsewhere.

    If any area still carries a Weight from a previous Weighted Voting for Areas
    configuration, the tally is refused until it is cleared, because the two
    weightings would multiply. The Weight field is hidden under this policy, so
    clearing it means switching back to Weighted Voting for Areas, clearing the
    Weight on each area, switching to Weighted Voting for Voters again, and
    republishing the ballots.

    :::danger Voter weights are public, and results are attributable
    A voter's weight is applied by splitting it into powers of two. A contest
    area is mixed as up to 17 batches, the batch at position `n` counting each
    ballot in it `2^n` times, and a voter's ballot is placed in the batch for
    each power of two that adds up to their weight — weight 21 goes into the
    batches for 1, 4 and 16. No batch ever holds the same ballot twice, so
    nothing on the bulletin board repeats in a way that spells out a weight.

    That is not enough to keep a weight private. The board is public so that the
    mix can be verified, the same ciphertext appears in every batch its weight
    selects, and each ciphertext is linkable to the voter who cast it, so adding
    up the batches a ballot appears in recovers that voter's weight exactly.
    **A voter's weight is therefore public, and it is public who holds it.** This
    is the flip side of a property some bodies want: published weights can be
    audited against the board.

    The published per-candidate totals are sums over the weights of the voters
    who chose each candidate. Where weights differ from one another, that sum
    frequently identifies exactly who voted for whom — with distinct weights
    such as 1, 2, 4, 8 it always does. Turning off decoded ballots in the results
    does not prevent this; the totals alone are enough. Write-in answers are
    worse again, since an uncommon write-in appears with exactly the weight of
    the voter who wrote it.

    Only the largest weights reach the highest batches, so those batches can hold
    very few ballots — and a batch holding one ballot publishes that voter's
    choice when it is decrypted, because a mix of one hides nothing. The tally
    logs a warning naming any batch below five ballots. It does not refuse, since
    by then voting has closed and there is no remedy left; check the spread of
    weights before opening voting instead.

    Do not use this policy where ballot secrecy is required. It suits bodies
    that already publish how each member voted, such as some shareholder or
    delegate votes. Where secrecy matters and voters fall into a small number of
    weight classes, Weighted Voting for Areas gives weighting without this
    disclosure, because every voter in an area shares one weight.
    :::

    :::caution
    The `vote-weight` attribute must be declared in the election event realm's
    Keycloak user profile before use. Until it is, the weight column is missing
    from the voters export, the field does not appear when editing a voter, and
    editing a voter through the Admin Portal clears any weight that voter had.
    Importing weights and tallying them work regardless. Realm user profiles are
    not managed from this application, so declaring the attribute is an
    administrator step on the realm configuration.
    :::
  - **Disabled Weighted Voting**: Disable weighted voting.
- **Delegate Voting Policy**:
  - Allows for voters to delegate their vote to another voter. An additional column needs to be included in the voters imported csv with the name `delegate-vote-to` with the username of the voter to delgate the vote to.
