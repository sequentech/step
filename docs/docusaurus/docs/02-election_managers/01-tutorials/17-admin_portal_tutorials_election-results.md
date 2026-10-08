---
id: admin_portal_tutorials_election-results
title: Election results
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The main path is in [06-RESULTS](../03-procedures/06-results.md). This page gives the details:
the sections of the results page, all the export formats and levels, and the publication on the
results website.

## Before you start

- The tally status is `SUCCESS`. See [Tally ceremony](16-admin_portal_tutorials_tally-ceremony.md).
- To see the results, your account needs **Read Tally Results**.
- To download the documents, your account needs **Export Ceremony**.

## The results page

Open the tally from the **Tally** tab and go to the **Results** step. The page has these sections.

### General Information

- **Tally Date**: the date of the tally.
- A table with one row for each election: **Elections**, **Eligible Voters**, **Total Voters**
  and **Participation**. Some elections also show **Total Declined to Vote** or
  **Total Blank Ballots**.

### Results & Participation

Select an election, then a contest, then an area. The area tab **Global** shows all the areas
together. For each selection, the page shows:

- The **Participation Summary**: **Eligible Voters**, **Total Voters**, **Total Valid Votes**,
  **Total Invalid Votes** and **Blank Votes**, with their percentages. The invalid and blank votes
  can show as explicit and implicit votes.
- The **Participation by channel**: the votes of each channel, for example **Online**, **Kiosk**,
  **Early voting** and **Telephone**. Votes from tally sheets show as **Paper**, **Postal** or
  **In person**.
- The **Candidate Results**: the **Number of Votes**, the **Percent of Votes** and the
  **Winning position** of each candidate. With voter-weighted voting, the table also shows the
  **Weight**.
- Charts of the votes for the candidates, the blank votes and the invalid votes.

A contest that was won by acclamation shows a note. It was decided without a vote, so it has no
votes.

### Instant Runoff contests

For an **Instant Runoff** contest, the page also shows each **Round**. Each round shows the votes
of each **Candidate**, the candidate that is **Eliminated**, and at the end the **Winner**. If a
round stopped at a tie, see
[Resolve a tie](16-admin_portal_tutorials_tally-ceremony.md#resolve-a-tie).

## Download the result documents

Each level of the results has an actions menu. The menu shows only the formats that the tally made
for that item.

| Level | Where | Menu item |
| --- | --- | --- |
| Election event | The **Results & Participation** title | "Export in ... format - '...' results" |
| Election | The tab of the election | The same, plus "Export All Areas Results in ... format for '...'" |
| Contest | The tab of the contest | "Export in ... format - '...' results" |
| Area | The tab of the area | "Export in ... format - '...' results" |

| Format | Content |
| --- | --- |
| **PDF** | The results document, ready to print and sign. |
| **HTML** | The results document, for a browser. |
| PDF from **HTML** | The menu also has a PDF item next to each HTML item. The platform makes the PDF from the HTML document when you click it. |
| **JSON** | The results as data, for other systems. |
| **TAR_GZ** | A compressed archive with the result files. |
| All the areas, **HTML**, **JSON** or **PDF** | For an election: the results of all its areas in one file. Not available for an initialization report. |
| **XLSX** | For the election event only: the results in a spreadsheet. |

The PDF from HTML and the XLSX file are made by a task. The task shows in the task panel of the
admin portal, as **Render Document PDF** or **Export Tally Results in XLSX format**. When the task
is complete, the browser saves the file.

:::note
If the deployment uses transmission packages, the menu of an election also has items to make and
send them. See [Transmission](../02-reference/02-election-event/08-02-election_management_election-event_transmission.md).
:::

## Reports with results

The **Reports** tab makes documents from templates. These report types are about results and
participation:

| **Report Type** | **Election** | Actions |
| --- | --- | --- |
| **Electoral Results** | Required | **Preview** |
| **Initialization Report** | Required | **Preview** |
| **Participation Report** | Optional | **Generate** and **Preview**. You can also make it **Repeatable**. |

See [Reports and templates](18-reports_and_templates.md).

## Publish the results on the results website

The main steps are in
[06-RESULTS](../03-procedures/06-results.md#4-publish-the-results-on-the-results-website). These
rules apply:

- **Results Website** of the election event must be **Enabled**.
- The tally must be complete. Before that, the section shows "Results can be published after this
  tally has completed."
- **Route**: **Event results** publishes the elections of the tally. **Election results**
  publishes one **Election**.
- **Access**: **Public access** or **Authenticated access**. With **Public access**, the
  visibility is always **Full published scope**.
- **Visibility**: **Full published scope** shows all the published results. **Personal
  visibility** shows to each signed-in voter only the results of the area of that voter.
- If the election event sets **Results Website Access** or **Results Website Visibility**, the
  section uses these values and you cannot change them.
- **Contests**: only the contests with results show. All are selected by default.

The confirmation says "This will create a new publication from the current tally execution. The
existing voter-facing results stay active until this publish task succeeds."

Each publication gets a new **Version** in the **Publication history**. **Open** and **Revoke**
show only for a publication that is published. **Revoke** removes it from the results website.
The platform records each publication and each revocation in the electoral log.

| Permission | Lets you |
| --- | --- |
| **Edit Results Publication** | Publish and revoke. |
| **Read Results Publication** | See the **Publication history**. |

For the results website, see [Results website](../02-results-website.md).

## If there is a problem

| Problem | Action |
| --- | --- |
| The **Results** button is not available. | The tally is not complete. Wait until the status is `SUCCESS`. |
| The actions menu is not shown. | Ask for the permission **Export Ceremony**. |
| A format is not in the menu. | The tally did not make that document for the item. Contact Sequent support. |
| The PDF or XLSX file does not download. | Look at the task panel. If the task failed, try again, then contact Sequent support. |
| "Results website publishing is disabled for this election event. Enable it in the election event data before publishing results." | Set **Results Website** to **Enabled** on the **Data** tab of the election event. |
| "You need publish-results-write permission to publish or revoke results." | Ask for the permission **Edit Results Publication**. |
| "You need publish-results-read permission to view publication history." | Ask for the permission **Read Results Publication**. |
| "No tallied contests available." | The selected route has no contest with results. Select another route or election. |
| "Could not start results publication" | Try again. If the message continues, contact Sequent support. |
| "Could not revoke results publication" | Try again. If the message continues, contact Sequent support. |
| The number of votes is not what you expect. | Do not publish the results. Check the publication, the areas and the **Logs** tab, then contact Sequent support. |
