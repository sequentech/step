---
id: admin_portal_reference_user_manual_electoral_log
title: Electoral Log
---

<!--
-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Overview

The **Electoral Log** page shows the electoral log and the ballot box of your tenant's election events. Administrators of the super-admin tenant can browse any tenant's events and run read-only SQL queries on the electoral log. Nothing on this page changes any data.

To open it, select **Electoral Log** in the Admin Portal's left panel. It requires the `electoral-log-console-read` permission.

## Browsing Tables

1. In the super-admin tenant, choose the **Tenant** whose events to browse; other administrators see their own tenant's events.
2. Choose the **Election Event** and the **Table**:
   - **Records:** every entry of the event's electoral log.
   - **Ballots:** the ballots the event's ballot box accepted, with their status, voting channel and size, but not their content.
   - **Voters:** each voter who voted, with the number of times and their latest ballot.
   - **Sequencer Queue:** accepted ballots that are not yet recorded in the electoral log. It is normally empty or short while voting runs.
3. To narrow the table, fill in filters and select **Apply Filters**. **Clear** removes them.
4. Choose **Newest first** or **Oldest first** in **Order**.
5. Use the arrows below the table to move to the next or previous page. Above the table, the page shows about how many rows the table has before filters.
6. In **Records**, select the eye icon of an entry to see all its fields and its decoded message. **Copy JSON** copies them.

## Personal Data

Usernames, IP addresses and countries show as `hidden` unless you have the `electoral-log-personal-data-read` permission.

## Queries

The **Query** tab appears for users of the super-admin tenant with both the `electoral-log-console-query` and `electoral-log-personal-data-read` permissions.

1. Write a SQL query.
2. Select **Run Query**, or press Ctrl+Enter. The page shows the rows, how long the query took and, when there were more than 1,000 rows, that only the first 1,000 are shown.

- Queries can only read. They run in a read-only transaction, as a database user that can only read, and stop after 30 seconds.
- Only queries that return rows run: `SELECT`, `VALUES`, `TABLE` or `WITH`.
- Queries read the electoral-log database, which holds every tenant's election events. Its main tables are `electoral_log_messages`, `ballot_box_ballot`, `ballot_box_voter`, `ballot_box_pending` and `ballot_box_sequencer`.
- Each query is recorded in the server logs with the user who ran it.

Examples:

```sql
-- Records of each kind, across every election event
SELECT statement_kind, count(*) AS records
FROM electoral_log_messages
GROUP BY statement_kind
ORDER BY records DESC;

-- Ballots accepted per hour and voting channel in an election event
SELECT date_trunc('hour', accepted_at) AS hour, voting_channel, count(*) AS ballots
FROM ballot_box_ballot
WHERE election_event_id = '<election event ID>'
GROUP BY 1, 2
ORDER BY 1, 2;
```

## Exporting

Select **Export** above a table to download it as a CSV file. In **Tables** it contains the current page; in **Query**, all the rows the query returned.
