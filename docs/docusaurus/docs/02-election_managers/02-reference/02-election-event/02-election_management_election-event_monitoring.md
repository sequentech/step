---
id: election_management_election_event_monitoring
title: Monitoring
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Monitoring dashboards

Monitoring is a set of configurable dashboards shown in an election event's
**Dashboard** tab. A dashboard is made of widgets, and every widget is built
the same way:

- it reads one governed **data source**, such as voter turnout, poll status
  or enrollment decisions;
- it runs one or more **queries** over that source, each a template of the
  source plus parameters;
- it draws with **dbt Charts** YAML, which controls how it looks.

What is counted, and how, belongs to the data source and cannot be
configured. What is shown, and how it looks, is configuration an
administrator can change in the Admin Portal. The
[monitoring configuration reference](../12-monitoring-configuration.md)
describes every field.

## Standard dashboard or monitoring

The **Dashboard** tab shows the monitoring dashboards when both of these hold:

- the event has been set up for monitoring (its Dashboard tab is in
  *configured* mode); and
- the user has the `monitoring-view` permission.

Otherwise the tab shows the
[standard dashboard](./01-election_management_election-event_dashboard.md),
exactly as before. A user without `monitoring-view` never triggers a
monitoring request. If the monitoring dashboards cannot be loaded, the
standard dashboard is shown with a warning above it.

An event is set up for monitoring by **resetting it to a preset**: the
preset's settings, themes, widgets and dashboards are written to the event as
its first revisions, and the Dashboard tab switches to configured mode. A
user with the configure permissions does this with the
`monitoringResetToPreset` GraphQL action; once an event shows monitoring,
[Reset to preset](#reset-to-preset) in the dashboard editor does the same.
The platform ships two presets:

| Preset | For |
|---|---|
| `comelec` | An overview and one dashboard for each monitoring record of an overseas voting package. |
| `campus` | A university election: participation by faculty and role, and the day's operations. |

Switching the tab back to the standard dashboard keeps the configuration and
its history; switching again shows the same dashboards.

## Viewing a dashboard

### Header

The header shows, from left to right:

- the dashboard's title, with a menu to switch to another dashboard of the
  event;
- how many widgets it has, and the requirement IDs it answers, if any;
- **Updated** and a time: when the figures on screen were counted (see
  [Freshness](#freshness));
- **Export**, to export the dashboard's data;
- **Edit dashboard**, for users who may configure monitoring.

### Dashboard selectors

A dashboard can offer up to three selectors that apply to every widget on it:

| Selector | Chooses | "All" option |
|---|---|---|
| **Region** | A region of the event. | All regions |
| **Post** | An election of the event. | All Posts, or All authorized Posts for a restricted user |
| **Country** | The voters' country. | All countries |

A preset may give them other names (the `campus` preset calls them Campus,
Polling station and Nationality); what they do stays the same. Choosing a
Post narrows the view to that election, so the Region selector no longer
applies. A country can be combined with a region or with a Post.

Each widget follows some or all of the dashboard selectors. A widget that
does not follow a selector, or whose data source cannot be narrowed by it,
keeps showing its own scope. For example, sign-in figures have no country,
and attack detections always cover the whole event.

On an election's own Dashboard tab the Post is fixed to that election and
only Country remains. See
[election monitoring](../03-election/02-election_management_election_monitoring.md).

### Widgets

Widgets sit on a grid 12 columns wide. Each takes 1 to 12 columns; widgets
fill a row and wrap to the next, and on a narrow screen they stack. A widget
card shows:

- its title, its data source and the requirement IDs it answers;
- its own **widget selectors**, if it has any: a dropdown or a toggle, such
  as Breakdown (Sex, Age, Status abroad), Show (which ratio), or Interval
  (Hourly, Daily). Some selectors appear only while another has a given
  value; for example, the Day picker shows only when Interval is Hourly;
- the chart;
- a **⋯** menu with **Configure widget**, **View data**, **Export CSV** and
  **Duplicate**. Configure widget and Duplicate appear only for users who may
  configure monitoring.

A dashboard can open the same widget with different selector values. For
example, a "Voted vs pre-enrolled" dashboard opens Turnout by group on
"Voted of pre-enrolled", while another dashboard opens it on "Voted of
registered". The values a viewer picks are kept while they stay in the
browser tab.

### How figures are counted

The counting rules belong to the data sources and are the same on every
dashboard:

- **Voters are counted once per scope.** A region's voters are counted over
  the region, never added up from its Posts, so a voter who can vote in two
  Posts is still one voter of the region.
- **Ratios are a numerator over a denominator**, both counted at the same
  scope. Where the denominator is zero the ratio shows as **—**, not 0%.
- **Missing values are shown.** A voter with no recorded sex, age or country
  appears in an **Unknown** group (or the preset's word for it), always last
  and never dropped by a row limit.
- **Turnout counts each voter's first valid vote.** Revotes do not count
  again, and activity charts place each voter in the hour or day of their
  first vote.
- **Hourly and daily buckets** are in the event's time zone, each from its
  start up to, but not including, its end, so they add up to the totals.
- **Sign-in figures are attempts, not people.** Two failed sign-ins by one
  voter count twice. Attempts with a username that matches no voter belong to
  no Post, so they are counted for the whole event only, and the widget says
  so.
- **Milestones count Posts.** "Posts opened" is out of the Posts in scope.

Charts show rounded numbers (such as 647 K); KPIs show exact counts and
percentages to one decimal (874,624 and 53.2%). **View data** shows the exact
values behind a chart.

### Freshness

Figures come from a snapshot, not from a live query. A background job counts
every configured event about every 30 seconds and stores a new snapshot only
when something changed. Every widget on the screen shows the same snapshot,
and the header's **Updated** time says when it was counted, in the event's
time zone. Before the first count finishes, the header shows **Not counted
yet**.

The dashboard checks for a new snapshot every 30 seconds while its browser
tab is visible, and redraws a widget only when its figures changed. The
number of people watching a dashboard does not change how often the data is
read.

### Widget states

A widget that cannot show a chart says why:

| Message | Meaning |
|---|---|
| **Not connected** · reason. Nothing is shown until it is. | The data source's producer is not available yet. The widget never shows zero in its place. |
| **Not counted yet** | No snapshot exists yet. The widget updates on its own. |
| **Counting this selection** | This combination of allowed elections has not been counted yet; it is counted in the next pass, within about a minute. |
| **The chart could not be drawn** | The chart engine failed or took too long. The figures are shown as a table instead. |
| **This widget cannot be shown** | Its configuration has a problem. |
| **This widget could not be loaded** | The request failed. It is tried again when the dashboard updates. |

Five data sources are declared but not connected yet, so their widgets show
**Not connected** with one of these reasons:

| Data source | Reason |
|---|---|
| Test voting | Test elections cannot be marked yet. |
| Voting credentials | Issuing credentials is not recorded yet. |
| Final testing and lockdown | Final testing and lockdown are not recorded yet. |
| Attack detections | No attack detection feed is connected. |
| Helpdesk | No helpdesk system is connected. |

### View data

**View data** in a widget's ⋯ menu opens a table of the rows the widget's
query returned, with exact values, for the snapshot and selections on
screen.

### Export

**Export** in the header exports the dashboard's data; **Export CSV** in a
widget's menu exports only that widget. The **Export monitoring data** dialog
asks for:

- the format, **CSV** or **SQL** (`CREATE TABLE` and `INSERT` statements);
- a **From** and **To** time. Times are in the event's time zone. Hourly and
  daily activity is exported from the start time up to, but not including,
  the end time; status and totals as recorded at each hour in the range.

The export uses the snapshot the dashboard shows, with its selectors, so the
file matches the screen. It runs as a task (Export Monitoring Data in the
**Tasks** tab), and the file downloads when it is ready. A snapshot stays
available for export for two hours after a newer one replaces it; after that,
refresh the dashboard and export again.

Every widget and query of the export is in the file. A query with nothing to
show has one row with its name and no figures, with the notice
`NO_ROWS_IN_RANGE` (activity with no hour in the range) or `NO_ROWS`; a widget
whose data source is not connected has one row with `NOT_CONNECTED`.

### What a restricted user sees

Permission labels limit monitoring the same way they limit the rest of the
Admin Portal. A user whose roles carry labels sees only the elections those
labels allow, and the elections that have no label:

- the Post selector lists only those elections, under **All authorized
  Posts**;
- the Region and Country selectors list only what those elections contain;
- every figure, including the "All" totals, is counted over those elections
  only, so it can differ from what an unrestricted user sees;
- asking for a Post, region or country outside them is refused.

The first time a combination of allowed elections is viewed, its widgets
show **Counting this selection** until the next pass has counted it.

## Configuring dashboards

Users who may configure monitoring see **Edit dashboard** in the header and
**Configure widget** and **Duplicate** in each widget's menu. Configuring
needs all of:

- the `monitoring-configure` permission;
- the `election-event-write` permission;
- an election event that is not locked down. Once the event's **Lockdown
  Status** is enabled, its monitoring configuration can no longer change.

### How a change is saved

Dashboards, widgets and themes are stored per event as YAML documents. Every
save is checked on the server before it takes effect:

1. against the platform's policy: no SQL, URLs, links, markup or typed-in
   data, and only what the data source offers;
2. against the rest of the event's configuration: a dashboard names widgets
   that exist, a group-by names a configured dimension;
3. by the chart engine: the dbt Charts schema, and a test render with sample
   figures.

If any check fails, nothing is saved, the problems are listed by where they
are in the YAML, and the previous revision stays live. Warnings (for example
a dashboard selector that narrows none of its widgets) do not block a save.

Each accepted save becomes a new **revision** of the document, with its
author and time, and is recorded in the event's electoral log (see the
[Logs](./11-election_management_election-event_logs.md) tab). Earlier
revisions are kept.

If someone else saved the same document while you were editing, the save is
refused and **Someone saved first** opens, naming who saved which revision.
Choose **Reload** to start again from their revision, **Copy my YAML** to
keep your text on the clipboard, or **Keep editing**. If the server is busy
with other changes to the same event, or the chart engine cannot be reached,
nothing is saved and the save can be tried again.

### Configure widget

**Configure widget** opens the widget's definition next to a live preview.
The three tabs edit the same YAML:

- **Data & query**: title, data source, query template, measures,
  numerator and denominator, group by, sort and row limit, and which dashboard
  selectors the widget follows. **Query result · first rows** shows what the
  query returns. The help under the data source states its counting unit,
  for example "Counts distinct voters; counting rules are fixed by the data
  source."
- **Selectors**: each widget selector's label, control (dropdown or toggle),
  options and default. Selectors appear in the widget header; their values
  feed the query.
- **YAML**: the whole definition, including the dbt Charts `chart` block.

While the YAML has a syntax error, the form tabs are read-only; fix it in
the YAML tab to use them again. Problems are listed under **Checks**, each
marked as a YAML syntax error, a browser check or a server check.

The footer shows the preview's state: the scope it was drawn for, **Valid**
or the number of errors, chart warnings, how long the render took, and the
revision being edited with who saved it and when. **Validate** runs the
server's checks without saving; **Save widget** saves. A widget can appear on
several dashboards, and a saved change shows on all of them.

### Edit dashboard

**Edit dashboard** turns the dashboard into an editor where you can:

- change its **Title**;
- choose its **Dashboard selectors** (Region, Post, Country);
- choose its **Theme**, and open the theme editor;
- set each widget's **Width**, shown as "6 of 12";
- reorder widgets with **Move up**, **Move down** or by dragging;
- **Remove** a widget from the dashboard; the widget itself stays in the
  catalog;
- **Duplicate** a widget, which adds a copy under a new id that can be
  changed without affecting the original;
- **Add widget** from the catalog, which lists every widget of the event
  grouped by data source and can be searched by title, data source or
  requirement ID.

**Save dashboard** saves it as a new revision; **Cancel** discards the
changes. While editing, the dashboard does not refresh.

Per-dashboard selector defaults (the `values` of a layout entry) and the
dashboard's requirement IDs are part of its YAML; see the
[configuration reference](../12-monitoring-configuration.md#dashboard).

### Theme editor

**Dashboard theme** shows the theme's YAML next to real widgets drawn with
it, and says how many widgets it applies to. A theme holds the dbt Charts
style (base theme, fonts, palette, category colours) merged into every
widget on the dashboards that use it; a widget's own style wins over the
theme's. **Apply theme** saves it. The `default` theme cannot be removed,
because widgets shown outside a dashboard use it.

### Reset to preset

**Reset to preset** replaces every dashboard, widget and theme of the event,
and its settings, with the chosen preset's. The current configuration stays
in the history. The event's **settings** (time zone, where region and country
are read, who counts as pre-enrolled, the voter dimensions) can only be
changed this way, because they decide who is counted.

## Permissions

| Permission | Allows |
|---|---|
| `admin-dashboard-view` | The event's Dashboard tab, standard or monitoring. |
| `monitoring-view` | See the monitoring dashboards, view a widget's data and export it. |
| `monitoring-configure` | Edit widgets, dashboards and themes, and reset the event to a preset. Also needs `election-event-write`. |

See [Permissions](../user-manual/users-and-roles/users-and-roles_permissions.md#monitoring-dashboard-permissions)
for which roles receive them.
