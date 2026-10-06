---
id: timezones_and_schedules
title: Timezones and schedules
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Set timezones before preparing the schedule. In the event's **Data** tab, configure
its available timezones and choose a primary zone. Each election (Post) can use one
of those zones; an election with no separate choice uses the primary. Use IANA
names such as `Europe/Madrid` or `Asia/Kolkata`, rather than a fixed UTC offset:
the offset can change with daylight saving time.

The primary zone is used for event-wide schedules, event-wide documents and
transmission timestamps. Log display can use the primary or the election's zone;
entries without an election use the primary. A timezone label always belongs to
the instant being displayed. An April offset need not equal the May offset.
Date-only fields remain dates.

## Entering a schedule

In **Scheduled Events**, select the type and target, then enter a local date and
time and its zone. A Post row defaults to the Post's zone; an event-wide row
defaults to the primary. Review the resulting instant before saving. The server
stores the local time, IANA zone and UTC instant together. A date supplied as an
instant must include `Z` or a numeric offset.

A local time can be missing when clocks move forward, or occur twice when clocks
move back. A missing time is shifted forward by the clock change with a warning;
the preview shows the time that will be saved. For an overlapping time, the
first occurrence is used and the two candidate instants are explained. Review
these notices and change the input when that result is not intended. Do not infer an
instant from the workstation's timezone. The optional **My time** display is a
conversion of the saved instant, not a second schedule.

A Post's voting opening takes precedence over an event-wide opening. An
event-wide voting close supplies the close for Posts without their own close.
Check any separate Post closes: they override that event-wide fallback. Voting
channels selected on a row apply only to channels enabled for its target. Older
rows with no channel selection use online and kiosk.

For example, Posts can open at local midnight on 9 April while an event-wide
close at 19:00 on 8 May in `Asia/Manila` gives them a common closing instant.
The local closing day and time differ across Posts. The 30-day and short-last-day
notices warn about those differences; they do not move the deadline or block a
save. Final testing lead-time notices are also warnings.

Enrollment openings can be per Post. Enrollment closes at the event-wide close,
which is a separate row from the voting close. To close enrollment one hour
before voting, configure both rows explicitly. There is no implicit one-hour
subtraction. The registration server checks the chosen Post's window; a page
left open across the opening time may need to be reloaded.

## Importing and exporting CSV

Export the current schedule to obtain the column format:

```csv
election_alias,event_type,local_date_time,timezone,voting_channels
council-canary,START_VOTING_PERIOD,2028-04-09T00:00,Atlantic/Canary,online|kiosk
ALL,END_VOTING_PERIOD,2028-05-08T19:00,Europe/Madrid,online|kiosk
```

Use the election's exact alias, or `ALL` for the event-wide target. Supported
window types are voting, enrollment, readiness, final testing and test voting,
using the type names in the export. `timezone` can be blank: the Post's zone or,
for `ALL`, the primary is used. `voting_channels` is optional and uses `|` between
values. Preserve quoted CSV fields when an alias contains a comma.

Preview before importing. Each row shows its local time, zone, resolved instant
and primary-zone time. Correct all errors first: an import with row errors cannot
be applied. Import creates or updates by target and event type; it does not remove
rows absent from the file. Export and review the result after importing. Malformed
legacy rows that lack a usable instant may be omitted from export with a server
warning; compare the list rather than treating export as a repair operation.

Creating or editing a voting row can change its signing authorization. Read the
outcome notice and **Why?** details before saving or importing; see
[Scheduled openings and closings with signatures](./18-signed-scheduled-transitions.md).

## Timezone database changes

Timezone rules can change after a schedule is saved. A daily check compares
future rows' stored instants with their local times under the installed timezone
database. A difference appears in Scheduled Events for review. **Apply** updates
the affected instants and writes `ScheduleRecomputeApplied` to the audit log.
Until Apply, the stored instant is retained. Applying a change to a signed row
can remove its coverage; publish and approve again where required.

Review enrollment synchronization after schedule changes. Database state and
Keycloak realm state are separate systems: an enrollment refresh failure must be
resolved before relying on the new restriction. The
[enrollment operations guide](../../../07-developers/06-keycloak/enrollment_windows.md)
describes the realm attribute, existing-realm migration and server checks.

## Labels and language

Use the event's language settings and text overrides to customize timezone
names and date/time messages. Keep the placeholders a message requires. Voter
screens show the Post's opening and the common close with their zones; they may
also show the Post-local close and device-local conversion. Event-wide reports
and messages use the event default language and primary zone. A Post-specific
report uses the Post's zone and names the primary where a common deadline is
shown. Voter account letters use a date-only issue date in the event's primary
zone. Ballot receipts, per-election participation reports and ballot images use
their election scope; event-wide participation reports use the primary. Check
the document's scope when it covers multiple Posts. Tally reports and ballot
images resolve their primary, election zones and text overrides together when
the report run is prepared. This changes report presentation; it does not
rewrite a signed ballot style or alter the voter schedule and zone held in a
published configuration. Publish the revised configuration to update voter
outputs.

For migration effects, including the former monitoring timezone and transmission
offsets, read the [event timezone migration guide](../../../07-developers/11-updates/event-time-zones-migration.md).
