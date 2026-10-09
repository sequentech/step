---
id: admin_portal_tutorials_create-election
title: Create Election
description: "This tutorial guides you through the process of creating a new election event, setting up specific elections, and adding contests and candidates within the Sequent Admin Portal."
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

import GoogleVideo from '@site/src/components/GoogleVideo';

<GoogleVideo id="1-LhcFt6-RG6X0wvkO5EvOJzgT2hL2jpK" />

This tutorial guides you through the process of creating a new election event, setting up specific elections, and adding contests and candidates within the Sequent Admin Portal.

## Step 1: Initiate Election Event Creation

You can start from the sidebar of the Sequent Admin Portal in these ways:

![Create Election Event](./assets/elections_create_election_event.png)

* **Election Events Menu**: Click the **+** icon next to **Election Events**, then click **Create an Election Event**.
* **Election Event Tree**: Click **Create an Election Event** at the end of the election event tree, then click **Create an Election Event** in the menu that opens.
* **Three-Dot Icon**: Click the three-dot icon next to an election event, then click **Create an Election Event**.


## Step 2: Configure Event Details

After initiating the process, the system will prompt you for the basic configuration of the event.

![Name the Election Event](./assets/elections_name_desc.png)

1.  Enter the **Name** of the election event.
2.  Provide a **Description**.
3.  Click the save button.

:::info
**Post-Creation Options**: Once the event is created, you will see different menu options at the election event level, for example **Dashboard**, **Data**, **Localization** and **Voters**. For the full list, see [Create the Election Event](../03-procedures/02-event.md#1-create-the-election-event).
:::

## Step 3: Set Event Data and Preferences

In the **Data** tab, you can manage the core properties of the event.

![Election Event Data Tab](./assets/elections_data.png)

* **General**: Define the name, alias and description in each language, such as English and Spanish.
* **Language**: Set up the languages and the default language of the event.
* **Ballot Design**: Customize visual options for the digital ballot.
* **Advanced Configurations**: Manage specific policies such as the **Voting Portal Countdown policy** or the **Contest encryption policy**.

## Step 4: Create a Specific Election

An Election Event acts as a container that can hold one or more specific elections.

![Create an Election](./assets/elections_create_election.png)

1.  In the sidebar under your new event, click **Create an Election**.
2.  Enter the **Name**, the **External ID** and the **Description** of the election. **Name** and **External ID** are required.
3.  Click the save button.
4.  (Optional) Continue customizing your election using the various configurations available in the Data tab.

:::tip
**Election Level Dashboards**: Each specific election has its own dashboard to track metrics like eligible voters, actual voters, and voting trends by day or channel.
:::

## Step 5: Add Contests and Candidates

Finally, define the structure of your ballot by adding contests (questions) and their respective candidates.

![Create a Contest](./assets/elections_create_contest.png)

1.  **Create a Contest**: Click **Create a Contest** under the specific election. Enter the **Name** (e.g., "Question 1") and click the save button.

![Create a Candidate](./assets/elections_create_candidate.png)

2.  **Add Candidates**: Under the newly created contest, click **Create a Candidate**.
3.  Enter the **Name** and the other details, then click the save button.
4. (Optional) Customize your contest using the various configurations available in the Data tab. See [Create Contests and Candidates](05-admin_portal_tutorials_contests-candidates.md).

