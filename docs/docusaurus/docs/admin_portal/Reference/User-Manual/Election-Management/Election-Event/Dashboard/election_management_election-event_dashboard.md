<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

---
id: election_management_election_event_dashboard
title: Dashboard
---

# Election Event Dashboard Overview

At the top of the **Data Display** section on the dashboard of an Election Event, there is a **Step Crumb** that indicates the status of the election event. These statuses are **sequential** and described below:

## Election Status Steps

- **Created**  
  The initial stage where the election event is set up. Administrators configure:
  - Ballot designs
  - Voters
  - Areas (districts/precincts)

- **Keys**  
  After configuration, the administrator executes a **Key Ceremony**, generating cryptographic keys. Once complete, the election status changes to **Keys**.

- **Publish**  
  Once keys are generated:
  - Administrators **publish** ballot styles to the voting portal.
  - This is comparable to printing paper ballots.
  - Voters accessing the voter portal can now view the election.
  - Administrators may publish multiple times during edits.

- **Started**  
  The **voting period** has begun.

- **Ended**  
  The voting period has **concluded**.

- **Results**  
  - The **Tally Ceremony** is executed.
  - Previously generated cryptographic keys are used to **decipher encrypted ballots**.
  - **Results are made available** to administrators.

---

## Metrics

- **Eligible Voters**  
  Total number of voters imported into the system, including those not yet enabled.

- **Elections**  
  Number of individual elections within the event.

- **Areas**  
  Number of geographic areas involved in the event.

- **Emails Sent**  
  Total number of emails sent to voters.

- **SMS Sent**  
  Total number of SMS messages sent to voters.

---

## Charts

- **Votes by Day**  
  A graphical representation of **daily voting activity** per ballot cast.

- **Voters by Channel**  
  A pie chart showing the distribution of voters (who voted on at least one ballot) by voting channel:
  - Online
  - Paper
  - Telephone
  - Postal

---

This dashboard provides administrators with a **clear overview** of the election event’s progress and key statistics, ensuring **efficient management and oversight** throughout the election lifecycle.


