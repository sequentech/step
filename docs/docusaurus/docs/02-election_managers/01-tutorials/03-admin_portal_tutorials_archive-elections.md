---
id: admin_portal_tutorials_archive-elections
title: Active, Archived and Trash Election Events
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The election event menu has three tabs:

- **Active** lists the election events in use.
- **Archived** lists the election events put aside with **Archive** in the
  election event's menu (`...`). Voting, scheduled voting changes and votes are
  refused while an event is archived. **Unarchive** brings it back to Active.
- **Trash** lists the deleted election events. It is shown to users with the
  `election-event-delete` permission.

## Deleting an election event

**Move to Trash** in the election event's menu deletes it. The confirmation
says how many days it stays in Trash. An election event can't be moved to
Trash while voting is open or paused in it or in any of its elections: close
voting first.

An election event in Trash:

- keeps all its data (elections, voters, ballots, documents, logs) until it is
  deleted permanently;
- disappears from the Active and Archived lists and can't be opened or edited;
- is archived, so its voters can't vote and its scheduled events and reports
  don't run;
- can't have its voting status changed.

## Restoring an election event

Each election event in Trash shows when it was deleted and when it will be
deleted permanently. **Restore** brings it back to the tab it was in: Active,
or Archived if it was archived when it was deleted. Restoring never changes
voting status. Scheduled events whose time passed while it was in Trash are
stopped, not run late.

## Permanent deletion

Election events are deleted permanently once their time in Trash is over,
seven days by default. From then on Trash shows **Permanent deletion in
progress** and the event can no longer be restored. Each permanent deletion
appears in **Tasks** as *Delete Election Event*, run by *Trash retention*.
Restores appear as *Restore Election Event*.

The time in Trash is set by the deployment with the
`WINDMILL_ELECTION_EVENT_TRASH_RETENTION_DAYS` environment variable (see
[Backend components](../../05-reference/03-software-architecture/backend-components.md#election-event-trash)).
An election event keeps the permanent deletion date it got when it was
deleted, even if the setting changes later.
