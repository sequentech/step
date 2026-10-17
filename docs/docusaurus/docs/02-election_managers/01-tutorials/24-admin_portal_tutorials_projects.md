---
id: admin_portal_tutorials_projects
title: Plan Work with Projects
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Projects is a tenant plugin for planning work on a timeline: preparation
phases, readiness tests, logistics or training. It keeps its own projects and
tasks and **never changes elections, their schedules or their state**. Use
[Scheduled Events](13-admin_portal_tutorials_scheduled-events.md) to set the
dates the platform acts on.

## Turn Projects on

1. Open **Settings → Plugins**. Changing plugins needs the `tenant-write`
   permission.
2. Select **Turn on** on the Projects card and confirm.

Turning Projects on adds a **Projects** menu item after Templates. It creates
three permissions in the tenant and grants them to the `admin` group:

| Permission | Allows |
|---|---|
| `projects-read` | Open projects, read tasks and their history, export CSV |
| `projects-write` | Add, change, move and delete tasks; import a CSV as a new project |
| `projects-manage` | Create projects, archive and restore them |

Grant them to other groups in **Users and Roles** as needed. Users need to sign
in again, or refresh their session, to receive newly granted permissions.

Turning Projects off hides it and refuses its requests. Projects, tasks,
history and permissions are kept, and are back when it is turned on again.

## Projects

The list shows active and archived projects with their dates, task count,
timezone and last change. **New project** asks for a name and an IANA
timezone, such as `Asia/Manila`. Every date in a project is entered and shown in
its timezone, and the timezone is always shown on the project's page.

Archiving a project keeps it readable and exportable but prevents task
changes until it is restored.

## The timeline

A project's page shows its tasks on a timeline the full width of the page,
with each task's name on or beside its bar. Milestones are diamonds.

- **Point at** or **move the keyboard focus to** a task to see a short summary:
  its status, duration and dates.
- **Select** it (click, tap, or Enter or Space) to see its details below the
  timeline, where users with `projects-write` can edit them.
- With the keyboard, **↑** and **↓** move between tasks, **←** and **→** move
  the focused task by 24 hours, and **Shift+←** and **Shift+→** change its
  end. You can also drag a bar. Moving a task that is not open saves it at
  once; moving the open task changes its unsaved details.

A move is always 24 elapsed hours. Across a daylight-saving change the task's
local time shifts by the change; edit the dates below for a different time.

## Editing a task

A task has a title, a type (task or milestone), start and end (a milestone has
one date and time), a status, notes and an order. If a date and time is
skipped by a clock change in the project's timezone, you are asked for another
time; if it happens twice, you choose which.

Unsaved changes are kept while you point at, focus or move tasks. Leaving
them, by selecting another task, adding one, closing the details, using the
menu or the browser's Back button, asks you to keep editing or discard them.

If someone else saved the task first, your changes stay in the form and
nothing is overwritten. Select **Load their version** to see theirs, then make
your changes again.

Every change, deletion, import and export is recorded with who made it and
when. The task's details list its history; with no task selected, the
project's history lists every change to the project and its tasks, including
deleted ones.

## Import and export

**Export** downloads the project's tasks as CSV with the columns
`id,title,type,start,end,status,notes`. Dates are written in the project's
timezone with their UTC offset, for example `2028-03-06T08:00:00+08:00`.
Text that a spreadsheet could run as a formula is prefixed with an apostrophe,
and import removes it again.

**Import** creates a new project from a file in that format; it never changes
an existing project. Every row is checked before anything is saved, and every
problem is listed by line. The project is created only when all rows are
valid, and all of its tasks are saved together.

| Column | Values |
|---|---|
| `id` | Empty, or a unique UUID (kept only to detect duplicate rows) |
| `title` | Required, at most 200 characters |
| `type` | `task` or `milestone` |
| `start` | ISO 8601 date and time with a UTC offset |
| `end` | Required for a task, after `start`; empty for a milestone |
| `status` | `not_started`, `in_progress` or `done` |
| `notes` | Optional, at most 16,000 characters |

Files are UTF-8, at most 1 MiB and 1000 tasks.
