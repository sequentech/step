---
sidebar_label: Event timezones migration
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Event timezones migration

Migration `1791000000100_event_time_zones` makes the election event's
presentation (`presentation.timezones`) the only place that names the event's
timezone. Monitoring, signature appearances, the signing panel, transmission
packages (EML and ACM offsets) and their CCS log lines read the primary zone
from there.

## What it changes

- Every election event without `presentation.timezones` gets one zone as
  its configured and primary zone, with logs shown per election: the zone
  its live monitoring settings named (`time_zone: <zone>`), else `UTC`.
- Each live monitoring settings document that named a zone gets a new
  revision without the `time_zone` line. The event's configuration
  generation moves on by one; earlier revisions stay as they were saved. The
  new revision's author is `system:1791000000100_event_time_zones`.
- A presentation that isn't a JSON object becomes one holding only its
  timezones.

## Before you migrate

- **Finish or cancel every waiting Approve configuration request.** The
  publication digest covers the event presentation, so a request started
  before the migration no longer matches the configuration once
  `presentation.timezones` is added, and its approval is refused.
- **Check events without monitoring settings.** They get `UTC`. Before this
  release, their transmission packages were dated `+08:00`; from now on they
  are dated in the primary zone, so set the event's primary zone before the
  next transmission.

## After you migrate

- Read the migration's `WARNING` lines. They name each event whose settings
  wrote the zone in a form the migration doesn't rewrite (those settings no
  longer parse until you remove the line), whose zone wasn't a zone name
  (the event got `UTC`), or whose presentation wasn't an object.
- Each configured monitoring event makes one full pass over its voters and
  counts again, because its settings changed.

`down.sql` gives each settings document the migration rewrote, and that
nobody changed since, its previous text as a further revision. It leaves
`presentation.timezones` in place: code from before this release doesn't
read it.

## Known gaps

- **No electoral log entry for the migration's revisions.** A migration
  can't write the electoral log (immudb). The revisions it writes are told
  apart by their author id (`system:1791000000100_event_time_zones`, and
  `…:down` for the ones `down.sql` writes) and by their generation, which
  no logged change claims.
- **Rolling back is lossy for settings saved after the deploy.** Code from
  before this release requires `time_zone` in the settings, and `down.sql`
  only restores documents the migration itself rewrote. Settings reset to a
  preset after the deploy carry no zone, so after a rollback they no longer
  parse until they are reset again under the old release.
