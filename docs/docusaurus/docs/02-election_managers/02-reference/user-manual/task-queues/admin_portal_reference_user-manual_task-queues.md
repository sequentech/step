---
id: admin_portal_reference_user_manual_task_queues
title: Task Queues
---

<!--
-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Overview

The **Task Queues** page shows the installation's task queues: the background work
that the platform has waiting, what it processed and how long that took. It lets
authorized administrators replay or discard electoral-log events that could not be
stored.

The page belongs to the super-admin tenant. To open it, select **Task Queues** in the
Admin Portal's left panel while you are in the super-admin tenant. It requires the
`task-queues-read` permission.

The page's lists never show the arguments of a task, because they can contain voters'
data. Only queries read them; see [Query](#query).

## Queues

The table at the top lists every queue and updates every five seconds while **Live**
is on. Turn **Live** off to keep the figures still.

- **Ready:** messages waiting for a worker.
- **Running or scheduled:** messages a worker is processing, and messages waiting for
  their scheduled time or for a retry.
- **Oldest:** how long the oldest message has been in the queue.
- **Processed (last hour)** and **Outcomes (last hour):** messages processed in the
  last hour, by outcome.
- **Sent (total):** messages ever sent to the queue.

Outcomes are:

- **Succeeded:** the task completed.
- **Failed:** the task failed and was not retried.
- **Expired:** the task's time to run passed before a worker ran it.
- **Rejected:** the message could not be read, or names a task the workers do not
  have.
- **Discarded:** an administrator discarded the dead-lettered event.

Events of `electoral_log_event_queue` leave it in batches, which
`electoral_log_batch_queue` processes, so its processed counts stay at zero.

Select a queue to see its graphs and messages below the table.

## Throughput

Choose the **Period** to graph: the last hour, 6 hours, 24 hours or 7 days. Processed
messages are kept for seven days by default.

- **Processed messages by outcome** shows how many messages were processed in each
  interval, by outcome.
- **Mean wait and processing time** shows how long messages waited before a worker
  took them, and how long the worker took to process them.

## Messages

**Queued** lists the messages still in the queue; **Archived**, the processed ones,
with their outcome. Each page shows up to 50 messages, newest first: use **Older**,
**Newer** and **Newest** to move between pages, and **Refresh** to load them again.

For electoral-log events, the **Event** column shows the event's type and election
event.

## Dead-Lettered Electoral-Log Events

Electoral-log events that could not be stored wait in
`electoral_log_dead_letter_queue`, with the reason in the **Error** column. Point at a
reason to read all of it. A reason can quote the value that could not be read.

Users with the `task-queues-write` permission can select events in the queue's
**Queued** list and:

- **Replay** them: they go back to the electoral-log event queue, to be stored. Events
  that fail again come back to this queue. Replaying an event that was already stored
  does not store it twice.
- **Discard** them: they move to the archive with the **Discarded** outcome and are not
  added to the electoral log. Discarded events stay in the archive.

Both ask for confirmation. A worker applies the operation shortly after; the page
refreshes the list after a few seconds.

## Query

Users with the `task-queues-query` permission see a **Query** tab. It runs a read-only
SQL query on the environment's task-queue database and shows up to 1,000 rows. A query
runs for at most 30 seconds, and each one is recorded in the server logs with the user
who ran it. Press **Run Query**, or Ctrl+Enter, to run it; the table's toolbar exports
the rows as CSV.

Each queue has two tables: `pgmq.q_<queue>`, its waiting messages, and
`pgmq.a_<queue>`, its processed messages, with their outcome in the `x-step-outcome`
header. For example, the outcomes of the last hour in `short_queue`:

```sql
SELECT headers->>'x-step-outcome' AS outcome, count(*) AS messages
FROM pgmq.a_short_queue
WHERE archived_at > now() - interval '1 hour'
GROUP BY 1
ORDER BY 2 DESC
```

Unlike the rest of the page, queries read messages as they are stored, with their tasks'
arguments, which can include voters' data.
