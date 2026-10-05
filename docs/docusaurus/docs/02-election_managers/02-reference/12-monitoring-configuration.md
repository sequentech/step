---
id: monitoring_configuration
title: Monitoring Configuration
sidebar_position: 12
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This page describes the YAML behind the
[monitoring dashboards](./02-election-event/02-election_management_election-event_monitoring.md):
every field of the four kinds of document, the data sources and their query
templates, what the platform refuses, and how a widget's dbt Charts block and
its dashboard's theme combine.

An election event's monitoring configuration is a set of YAML documents,
each stored and revisioned on its own:

| Kind | How many | Key | Changed by |
|---|---|---|---|
| `settings` | One per event | `settings` | Resetting the event to a preset only |
| `theme` | Any | The theme's `id` | The theme editor, or a reset |
| `widget` | Any | The widget's `id` | Configure widget, Duplicate, or a reset |
| `dashboard` | Any | The dashboard's `id` | Edit dashboard, or a reset |

Every document refuses fields it does not define, so a misspelt key is an
error rather than a setting that silently does nothing.

**Identifiers.** Widget, dashboard and theme ids start with a lowercase letter
or digit and use lowercase letters, digits, `-` and `_`, up to 64 characters
(`turnout-by-group`). Selector, query and dimension names start with a
lowercase letter and use lowercase letters, digits and `_`, up to 40
characters (`age_band`). Option values use letters, digits, `_`, `-` and `.`,
up to 64 characters.

## Widget

A widget reads one data source through one or more governed queries, offers
its own selectors, and draws with a dbt Charts block.

| Field | Type | Required | Meaning |
|---|---|---|---|
| `id` | id | Yes | The widget's id; also the key it is stored under. |
| `title` | text | Yes | Shown on the card. |
| `description` | text | No | A line under the title saying what the figures are taken of, such as "Percentages use pre-enrolled OVs." Not empty when given. |
| `source` | data source id | Yes | One of the [data sources](#data-sources). |
| `requirements` | list of text | No | Requirement IDs this widget answers, for the catalog search. |
| `follows` | list of `region`, `post`, `country` | No | Which dashboard selectors narrow this widget. Absent: every one its source can be narrowed by. An empty list: the widget always shows the whole event. Listing a selector the source cannot be narrowed by is an error. |
| `selectors` | map of name to [selector](#selectors) | No | Selectors shown in the widget header, in the order written. |
| `query` | [query](#queries) | One of `query`, `queries` | The widget's single query, which charts read as `data`. |
| `queries` | map of name to query | One of `query`, `queries` | Several named queries, for a widget whose charts read more than one. |
| `chart` | dbt Charts block | Yes | Everything about how the widget looks; see [The chart block](#the-chart-block). |
| `height` | integer, 80 to 2000 | No | Frame height in pixels. Absent: 280. |

Example, a turnout ratio broken down by a voter dimension:

```yaml
id: turnout-by-group
title: Turnout by group
source: voter_turnout
selectors:
  breakdown:
    label: Breakdown
    options: {sex: Sex, age_band: Age, status: Status abroad}
    default: age_band
  measure:
    label: Show
    options: {voted_reg: Voted of registered, voted_pre: Voted of pre-enrolled, pre_reg: Pre-enrolled of registered}
    default: voted_reg
    maps: {voted_reg: [voted, registered], voted_pre: [voted_pre_enrolled, pre_enrolled], pre_reg: [pre_enrolled, registered]}
query:
  template: by_group
  group_by: {selector: breakdown}
  ratio: {selector: measure}
height: 320
chart:
  charts:
    bars:
      type: bar
      query: data
      x: group
      y: pct
      # The query's order, Unknown last; unsorted, a bar orders by value.
      sort: {by: position, order: asc}
      y_label: Share
      style:
        orientation: horizontal
        number_format: percent
        marks: {bar: {labels: {visible: true, format: percent}}}
  rows: [bars]
```

## Queries

A query is a template of the widget's data source plus parameters. It never
says how to count: that is fixed by the source.

| Field | Type | Meaning |
|---|---|---|
| `template` | `summary`, `by_group`, `by_post`, `timeseries`, `by_measure` | The shape of the result. The source must offer it. |
| `measures` | list of measures | Columns to count. Each measure once; the source must have it. |
| `ratio` | `[numerator, denominator]` | Two measures of the source. Adds the `numerator`, `denominator`, `pct` and `pct_label` columns. |
| `group_by` | dimension name | What to group by, for `by_group`. |
| `filters` | map of dimension name to list of values, or null | Keep only voters with these values of a voter dimension. A null value keeps them all. |
| `grain` | `hour` or `day` | Bucket size, for `timeseries`. |
| `day` | `YYYY-MM-DD` | The day an hourly series covers. |
| `sort` | `by: label`, `value` or `ratio`, and `order: asc` or `desc` (default `desc`) | Order of a `by_group` or `by_post` list. Absent: the template's own order. `value` sorts by the first measure, or the ratio's numerator when there are no measures; `ratio` needs a `ratio`. |
| `limit` | integer, 1 to 1000, or null | Rows of a `by_group` or `by_post` list to keep, after sorting. Unknown is always kept, after them. |
| `labels` | map of measure or Post state to text | How to show a measure or a Post state: `login_failures: Failed sign-ins`. Unlabelled ones use the platform's words. |

Any parameter can take a selector's value instead of a literal one, written
`{selector: <name>}`: the selector's chosen option, or what its `maps` entry
says the option stands for. `measures`, `ratio`, `group_by` and `grain` are
needed to run the query, so they cannot come from a selector that has a
`when` condition and can be hidden.

### Templates

Each template reads only some parameters; any other parameter is an error,
so a figure is never shown as filtered when it is not. The result's columns
are fixed per template, which is how a chart names them.

| Template | Parameters | Result columns |
|---|---|---|
| `summary` | `measures` or `ratio` or both; `filters`; `labels` | One row: a column per measure, then the ratio's four columns. |
| `by_group` | `group_by` (required); `measures` or `ratio` or both; `filters`; `sort`; `limit`; `labels` | A row per group: `group`, `group_key`, `position`, a column per measure, the ratio's columns. |
| `by_post` | `measures` or `ratio` or both; `sort`; `limit`; `labels` | A row per Post in scope: `post`, `post_id`, `region`, `state` and `state_label` (sources that count Posts), `position`, a column per measure, the ratio's columns. |
| `timeseries` | `measures` (required); `grain` (required); `day` (hourly only); `labels` | A row per bucket: `bucket_start`, `bucket_label`, `bucket_utc`, and for each measure its value and a running total `<measure>_cumulative`. |
| `by_measure` | `measures` (required); `filters`; `labels` | A row per listed measure: `measure`, `label`, `position`, `value`. For a pie of outcomes or a funnel. |

The ratio's columns are `numerator`, `denominator`, `pct` (a fraction, or
null when the denominator is zero) and `pct_label` (such as `53.2%`, or `—`
when the denominator is zero).

List rows carry `position`, from 1, in the query's order with Unknown last.
A chart that should keep that order sorts by it
(`sort: {by: position, order: asc}`); a horizontal bar chart left unsorted
orders by value. A time series should sort by `bucket_utc`, which stays in
time order when clocks go back.

Groups and filters:

- `group_by` can name a built-in dimension the source has (`region`, `post`,
  `country`, `reason`, `state`, `category`; see
  [Data sources](#data-sources)), or, for a source that carries voter
  dimensions, a dimension the event's [settings](#settings) configure.
- Grouping by `state` counts the Posts in scope by where they stand; every
  state of the source is listed, in the order Posts move through them, and
  the groups add up to the Posts in scope.
- `filters` apply to voter dimensions only. Region, Post and country are
  narrowed with the dashboard selectors, and a query grouped by a built-in
  dimension cannot be filtered.

## Selectors

A widget selector is shown in the widget header. Its value feeds a query
parameter, and only the options it lists are accepted.

| Field | Type | Meaning |
|---|---|---|
| `label` | text | Shown beside the control. |
| `options` | map of value to label | The options, in display order. Exactly one of `options` and `options_from`. |
| `options_from` | `event_days` | Options taken from the data: the days of the event with activity, in its time zone, as `YYYY-MM-DD`. The default is the latest day. |
| `default` | option value | Required with `options`, and must be one of them. Not allowed with `options_from`. |
| `control` | `dropdown` (default) or `toggle` | A toggle switches between exactly two listed options. |
| `when` | `{selector: <name>, in: [<values>]}` | Show the selector only while another selector, declared earlier, has one of these values. |
| `maps` | map of option value to query value | What each option stands for in a query, when that is not the option itself. Every option needs an entry, and every entry an option. Not allowed with `options_from`. |

An option stands for its own value unless `maps` says otherwise, so
`{sex: Sex, age_band: Age}` feeds `group_by` directly, and `{10: Top 10}`
feeds a `limit` of 10. `maps` turns an option into something larger: a ratio
(`voted_reg: [voted, registered]`), a sort
(`ratio: {by: ratio, order: desc}`), or null to leave a parameter out
(`all: null` for "no limit").

A selector's value comes from, in order: what the viewer picked, the
dashboard's `values` for the widget, and the selector's `default`. A request
for a value that is not an option is refused. A hidden selector (its `when`
does not hold) feeds nothing.

A day picker narrows an hourly series. When `grain` comes from a selector,
the `day` selector must be shown for every hourly option of it and no other,
with `when`:

```yaml
selectors:
  grain:
    label: Resolution
    options: {hour: Hour, day: Day}
    default: hour
  day:
    label: Day
    options_from: event_days
    when: {selector: grain, in: [hour]}
query:
  template: timeseries
  measures: [voted]
  grain: {selector: grain}
  day: {selector: day}
```

## Dashboard

Widgets on a 12-column grid, with the dashboard selectors they share.

| Field | Type | Required | Meaning |
|---|---|---|---|
| `id` | id | Yes | The dashboard's id; also its key. |
| `title` | text | Yes | Shown as the dashboard's heading and in the **Dashboard** menu. |
| `description` | text | No | A line under the heading saying what the dashboard shows. Not empty when given. |
| `section` | text | No | The heading the **Dashboard** menu lists it under, such as "Voter turnout", also shown above its title. Give a section's dashboards consecutive `order`s. Not empty when given. |
| `requirements` | list of text | No | Requirement IDs the dashboard answers, shown in its footer. |
| `order` | integer | No | Position in the **Dashboard** menu, lowest first. Default 0. |
| `selectors` | list of `region`, `post`, `country` | No | The dashboard selectors shown. A selector that narrows none of the dashboard's widgets is a warning. |
| `theme` | theme id | No | The theme merged into every widget. Absent: `default`. It must exist. |
| `layout` | list of layout entries | Yes | At least one widget, in order. |

A layout entry:

| Field | Type | Meaning |
|---|---|---|
| `widget` | widget id | The widget to show. It must exist. |
| `width` | integer, 1 to 12 | Columns the widget takes. |
| `values` | map of selector name to option | This dashboard's defaults for the widget's selectors. Each must name a selector the widget declares and one of its listed options; a selector with `options_from` cannot be fixed in advance. |

Example:

```yaml
id: voted-vs-pre-enrolled
title: Voted vs pre-enrolled
description: Pre-enrolled OVs who voted, by group, Post and country.
requirements: [REQ-12]
order: 2
selectors: [region, post, country]
layout:
  - {widget: turnout-summary, width: 12}
  - {widget: turnout-by-group, width: 6, values: {measure: voted_pre}}
  - {widget: voting-activity, width: 6, values: {grain: day}}
  - {widget: turnout-by-post, width: 6, values: {measure: voted_pre}}
  - {widget: turnout-by-country, width: 6, values: {measure: voted_pre}}
```

## Theme

dbt Charts style merged into every widget on the dashboards that use it.

| Field | Type | Required | Meaning |
|---|---|---|---|
| `id` | id | Yes | The theme's id; also its key. The `default` theme cannot be removed. |
| `title` | text | No | Shown in the theme editor. |
| `base` | `clarity`, `neon`, `paper`, `stark` or `vivid` | No | A dbt Charts built-in theme to start from. |
| `style` | dbt Charts board style | No | Fonts, palette, category colours and the other board style fields: `accent`, `background`, `border`, `box_shadow`, `charts`, `font`, `formats`, `frame`, `gap`, `layout`, `margin`, `muted`, `opacity`, `padding`, `placeholder`, `roles`, `text`, `title`, `tones`. |

Example:

```yaml
id: default
title: Standard
base: clarity
style:
  accent: "#1d4ed8"
  # Labels are shown as written: the engine's title case is English-only.
  title: {font: {case: none}}
  charts:
    axis: {title: {font: {case: none}}}
    legend: {title: {visible: false}}
    kpi: {label: {font: {case: none}}}
    # Unknown in grey wherever a chart colours by group.
    category_colors:
      group:
        values:
          Unknown: dbt-grays.muted
```

The chart frame ships one typeface, `'Inter Variable'`. Any other family
falls back to the viewer's system font, which may be a serif: the built-in
themes name faces the frame does not have (Source Serif 4 on `clarity`, a
tabular sans on the KPI, support-table, measure-axis and donut-total slots).
The shipped presets set `font.family`, `text.font.family` and
`title.font.family` and those slots to `'Inter Variable'`, so charts read in
the portal's typeface.

## Settings

Where the event's voters and Posts keep each fact the data sources count by,
and the event's time zone. Settings decide who is counted (for example, who
counts as pre-enrolled, a denominator), so the editor cannot change them:
only resetting the event to a preset writes them.

| Field | Type | Required | Meaning |
|---|---|---|---|
| `time_zone` | IANA name, or `UTC` | Yes | Buckets and export ranges are in this zone, such as `Asia/Manila`. |
| `scope.region` | [dimension mapping](#dimension-mapping) | Yes | Where a voter's or Post's region is read. |
| `scope.country` | dimension mapping | Yes | Where a voter's country is read. |
| `pre_enrolled` | `{voter_attribute: <name>, equals: <value>}` | Yes | Which voters count as pre-enrolled. |
| `dimensions` | map of name to dimension mapping | No | The voter dimensions widgets may group and filter by, such as `sex` or `age_band`. A name may not be a built-in dimension. |
| `unknown_label` | text | No | How a missing value is shown. Default: Unknown. Its key stays `__unknown__` whatever the label. |
| `selectors` | map of `region`, `post`, `country` to `{label, all}` | No | What the dashboard selectors are called, such as `{label: Campus, all: All campuses}`. A selector not named here takes the portal's translated words. |

### Dimension mapping

Exactly one of the three origins is given:

| Field | Meaning |
|---|---|
| `voter_attribute` | A voter account attribute. |
| `election_annotation` | An annotation of the election (the Post). |
| `area_annotation` | An annotation of the voter's area. |
| `split` | `{separator, part}`: keep one part of a compound value, counting from 0 (`Spain/Madrid` with `/` and 0 gives `Spain`). |
| `age_bands` | Treat the value as a date of birth, and count the voter in the band their age falls in on the event's first day. Each band is `{label, to}`; a voter is in the first band whose `to` their age does not exceed. Bands rise, and only the last may leave out `to`. |
| `labels` | Display labels for raw values: `{M: Male, F: Female}`. Values with no label are shown as written. |

Only derived values are stored: an age band, never a date of birth.

Example:

```yaml
time_zone: Europe/Madrid
scope:
  region: {area_annotation: campus}
  country: {voter_attribute: nationality}
pre_enrolled: {voter_attribute: email_verified, equals: "true"}
dimensions:
  faculty:
    voter_attribute: faculty
    labels: {SCI: Science, ENG: Engineering, HUM: Humanities, LAW: Law}
  age_band:
    voter_attribute: dateOfBirth
    age_bands: [{label: "18-24", to: 24}, {label: "25-39", to: 39}, {label: "40-59", to: 59}, {label: "60+"}]
unknown_label: Not stated
selectors:
  region: {label: Campus, all: All campuses}
  post: {label: Polling station, all: All stations}
  country: {label: Nationality, all: All nationalities}
```

## Data sources

A data source is a set of facts with a fixed counting unit. Configuration
picks a source, one of its templates and that template's parameters; adding
a source is a code change.

| Source | Counting unit | Measures | Templates | Dimensions | Producer |
|---|---|---|---|---|---|
| `voter_turnout` | Distinct voters at the selected scope | `registered`, `pre_enrolled`, `voted`, `voted_pre_enrolled` | `summary`, `by_group`, `timeseries`, `by_measure` | `region`, `post`, `country`, voter dimensions | Connected |
| `test_voting` | Distinct pre-enrolled voters | `pre_enrolled`, `test_voted` | `summary`, `by_group`, `timeseries`, `by_measure` | `region`, `post`, `country`, voter dimensions | Not connected: test elections cannot be marked yet |
| `enrollment_decisions` | Latest decision per voter | `applications`, `pending`, `approved`, `disapproved` | `summary`, `by_group`, `timeseries`, `by_measure` | `region`, `post`, `country`, `reason` | Connected |
| `voting_credentials` | Approved voters | `approved`, `credentials_issued` | `summary`, `by_group`, `timeseries`, `by_measure` | `region`, `post`, `country`, voter dimensions | Connected, by an interim rule: issued when the password is set |
| `poll_status` | Posts in scope | `posts`, `initialized`, `opened`, `paused`, `closed` | `summary`, `by_group`, `by_post`, `by_measure` | `region`, `post`, `state` | Connected |
| `final_testing_lockdown` | Posts in scope | `posts`, `tested`, `locked_down` | `summary`, `by_group`, `by_post`, `by_measure` | `region`, `post`, `state` | Not connected: final testing and lockdown are not recorded yet |
| `counting_transmission` | Posts in scope | `posts`, `tallied`, `transmitted`, `transmission_failed` | `summary`, `by_group`, `by_post`, `by_measure` | `region`, `post`, `state` | Connected |
| `voting_enrollment_activity` | Each voter's first valid vote or approval, in the bucket it happened | `approved`, `voted` | `summary`, `timeseries`, `by_measure` | `region`, `post`, `country` | Connected |
| `access_security` | Attempts, not people | `logins`, `login_failures`, `login_failures_valid_user`, `login_failures_unregistered`, `password_resets`, `password_reset_requests` | `summary`, `by_group`, `timeseries`, `by_measure` | `region`, `post` | Connected |
| `attack_detections` | Detections | `detections` | `summary`, `by_group`, `timeseries`, `by_measure` | `category` | Not connected: no attack detection feed is connected |
| `helpdesk` | Reported issues | `issues`, `pending_issues` | `summary`, `by_group`, `timeseries`, `by_measure` | `region`, `post`, `category` | Not connected: no helpdesk system is connected |

Notes:

- The dimensions `region`, `post` and `country` are also the dashboard
  selectors that can narrow the source. A widget of a source without
  `country` ignores the Country selector.
- `voted_pre_enrolled` counts the pre-enrolled voters who voted. A share of
  the pre-enrolled who voted is `[voted_pre_enrolled, pre_enrolled]`, never
  `[voted, pre_enrolled]`: `voted` also counts voters who voted without
  pre-enrolling, so that ratio can pass 100%. The platform does not refuse
  such a ratio; choose a numerator the denominator contains.
- Time series are counted for: `voter_turnout` (`voted`),
  `enrollment_decisions` (`approved`, `disapproved`),
  `voting_enrollment_activity` (`approved`, `voted`), `voting_credentials`
  (`credentials_issued`) and `access_security` (every measure). A series of any other measure is refused when the
  widget is drawn, rather than shown as zero.
- **Poll status.** A Post is `not_initialized`, `initialized` (its
  initialization report was generated), `opened`, `paused` or `closed`.
  Milestones stay reached as a Post moves on (a closed Post was opened);
  `paused` counts only while it lasts.
- **Counting and transmission.** A Post is `not_tallied`, `tallied` (a tally
  session of it succeeded), `transmitted` (every package received by as many
  servers as its threshold asks) or `transmission_failed` (a package was
  refused and not yet received enough).
- **Final testing and lockdown.** A Post is `not_tested`, `tested` or
  `locked_down`.
- **Enrollment decisions.** Each voter's latest application counts once; the
  `reason` dimension is the recorded disapproval reason.
- **Access and security.** Keycloak `LOGIN` events count as `logins`.
  `LOGIN_ERROR` counts as `login_failures` and, by whether the username
  named an account, as `login_failures_valid_user` or
  `login_failures_unregistered`. `RESET_PASSWORD` and `UPDATE_PASSWORD`
  count as `password_resets`, and `SEND_RESET_PASSWORD` (a forgot-password
  request that sent a new password) as `password_reset_requests`. An attempt
  belongs to the Posts its voter's area votes in; one with an unknown
  username, or by a voter with no area, is counted for the whole event only.
- **Voting credentials.** Approved voters are those whose latest application
  was accepted, and those imported without one. Until the platform records
  when credentials are issued, a voter's credentials count as issued when
  their Keycloak password is set, the earliest time seen; every figure of the
  source carries the notice `CREDENTIALS_ISSUED_WHEN_PASSWORD_SET`.
- A source that is **not connected** is declared so a widget can be placed
  today. Its widget shows "Not connected" with the reason until the producer
  exists, never zero.

## The chart block

A widget's `chart` is a dbt Charts 0.8.0 board, restricted to drawing the
widget's own governed rows. It may set only:

| Field | Meaning |
|---|---|
| `charts` | The charts, by name. At most 50 per widget. |
| `rows`, `cols`, `grid` | The layout: which charts go where. `rows: [bars]`, or `rows: [{cols: [a, b]}, c]`. |
| `style` | Board style, as in a theme. |

Each chart names its data with `query:`, which must be one of the widget's
queries: `data` for a widget with a single `query`, or a name under
`queries`. Columns are those of the query's [template](#templates). For
example, a KPI row over a summary query:

```yaml
id: access-security
title: Access and security
source: access_security
query: {template: summary, measures: [logins, login_failures, password_resets]}
height: 180
chart:
  charts:
    logins:
      type: kpi
      query: data
      value: logins
      label: Successful sign-ins
      style: {value: {format: integer}}
    failures:
      type: kpi
      query: data
      value: login_failures
      label: Failed sign-ins
      style: {value: {format: integer}}
    resets:
      type: kpi
      query: data
      value: password_resets
      label: Password resets
      style: {value: {format: integer}}
  rows: [{cols: [logins, failures, resets]}]
```

A table of Posts, with colours by state:

```yaml
id: status-by-post
title: Status by Post
source: poll_status
query:
  template: by_post
  measures: [posts]
  sort: {by: label, order: asc}
height: 360
chart:
  charts:
    posts:
      type: table
      query: data
      style:
        columns:
          post: {label: Post}
          post_id: {visible: false}
          position: {visible: false}
          region: {label: Region}
          state: {visible: false}
          state_label: {label: Status}
          posts: {visible: false}
      conditional_formatting:
        state_label:
          when:
            - {eq: Opened, background: positive.bg}
            - {eq: Paused, background: warning.bg}
            - {eq: Closed, background: dbt-grays.border}
  rows: [posts]
```

Bars, areas, lines, donuts, KPIs, tables, stacking, layers, labels, colours,
fonts and number formats are all available, within the rules below.

A number format chooses how a figure is written, such as `integer` or
`percent`, not its separators. Every figure a chart, KPI or data table shows
uses the election event's
[Number Format Policy](./02-election-event/03-election_management_election-event_data.md#language--region):
an event set to `1.234.567,89` shows `53,2%` where the engine writes `53.2%`.
Only a text that is a whole figure is rewritten, so labels such as `18-24`
or `10:00` are shown as written. Exports keep plain numbers.


The shipped presets draw parts of a whole as progress bars: a `spark_bar`
over a `by_measure` query whose first measure is the whole, so every bar is
read against it. Name the rows with the query's `labels`:

```yaml
queries:
  progress:
    template: by_measure
    measures: [posts, initialized, opened, closed]
    labels: {posts: Posts in scope, opened: Online voting opened, closed: Voting closed}
chart:
  charts:
    progress: {type: spark_bar, query: progress, x: value, y: label}
  rows: [progress]
```

A `spark_bar` scales its bars to its largest row, so leave the whole out
only where the rows are comparable on their own, such as successful and
failed sign-ins. The presets' themes widen its label column
(`style.charts.spark_bar.label.width`) so that longer names are not cut.

### How the theme and the widget combine

For each widget the platform builds the board dbt Charts draws:

1. It starts from the widget's `chart`.
2. It merges the dashboard theme's `style` underneath the widget's `style`:
   mappings key by key, lists and single values replaced whole. **The widget
   wins**; a null in the widget keeps the theme's value.
3. The theme's `base` becomes the board's built-in theme. In dark mode the
   chart is drawn with `neon`, dbt Charts' only dark theme.
4. It turns off what a dashboard must not show: the engine's footer and
   "data as of" line (the dashboard shows its own time, in the event's time
   zone), a pie or donut's total (slices need not add up to the scope), and
   a table's pager (the frame scrolls instead).
5. It adds the widget's query results as the board's only queries, as
   inline values.

The Admin Portal's preview and the server build the same board for the same
widget.

## What the platform refuses

Every number on a dashboard must come from a governed data source, so a
save that tries to bring in anything else is refused, with the problem named
and its path in the YAML. The checks run in every document, and all problems
are reported at once.

**Anywhere, in any document**, text may not contain:

- a template: `{{`, `{%`, `{#` or `${`;
- markup: `<` followed by a letter, `/`, `!` or `?`;
- a link: `](`;
- a script URL (`javascript:`, `vbscript:`) or a CSS `url(`;
- a URL or address: `scheme://…`, `//host`, `www.…`, or a word starting with
  `http:`, `https:`, `ftp:`, `file:`, `data:`, `blob:`, `about:`, `mailto:`,
  `tel:`, `sms:`, `intent:`, `ws:` or `wss:`.

YAML tags are refused, and a mapping may not give the same key twice.

**In the chart block and theme style:**

- Only the dbt Charts properties the platform classifies are accepted; a key
  it does not list is refused, so a property added by a dbt Charts upgrade
  reaches no widget until someone decides what it may do.
- Keys that would bring in data, a destination, markup or an expression are
  refused wherever they appear. Among them: `sql`, `url`, `link`, `path`,
  `files`, `queries`, `params`, `template`, `expr`, `where`, `aggregate`,
  `extends`, `theme`, `variables`, `footer`, `timestamp`, `icon`, `headers`,
  `token`, `password`, and every connection setting (`host`, `port`,
  `database`, `schema`, `user` and the like).
- A chart may not define its own query or source: `query:` names one of the
  widget's queries.
- The `callout` and `image` chart types are refused: they show typed-in
  content, not governed data.
- `tabs` and `details` are refused: the chart frame runs no script and
  navigates nowhere.
- Markdown text is refused; only the empty-chart overlay takes plain text.
- `visible` and `enabled` are `true` or `false`, never an expression or a
  query probe. A pie or donut `total` cannot be turned on.
- Maps draw only the geometry bundled with the platform: `world-countries`
  and `world-50m`.
- Chart text may not contain a double quote or a backslash; use typographic
  quotes (“ ”).
- A style value is one value: no `;`, `{` or `}`. Colours are `#rrggbb` (or
  3, 4 or 8 hex digits), `rgb()`, `rgba()`, `hsl()`, `hsla()`, a colour name,
  or a theme token such as `dbt-grays.muted` or `category[2]`.
- A `prefix`, `suffix`, `value_suffix` or `glyph` may not contain digits:
  they would read as part of the figure.
- Counts the engine draws that many of (ticks, bins, bars, columns, legend
  entries) are at most 100; other sizes are at most 10,000. Thresholds,
  domains and other values compared with a figure are not bounded.

**Limits:**

| What | Limit |
|---|---|
| Document size | 64 KiB |
| Nesting depth | 32 levels |
| Charts per widget | 50 |
| Query row `limit` | 1 to 1000 |
| Widget `height` | 80 to 2000 pixels |
| Layout `width` | 1 to 12 columns |

**Against the data source and the rest of the configuration:** a template,
measure, dimension or grain the source does not offer; a parameter the
template does not read; a selector default, `maps` entry, `when` value or
dashboard `values` entry that is not a listed option; a `when` naming a
selector not declared before; a group or filter naming a dimension the
settings do not configure; a layout naming a widget, or a dashboard naming a
theme, that does not exist; and duplicate ids.

Problems carry a stable code (for example `forbidden_key`,
`unknown_option`, `unsupported_by_source`, `layout_width`,
`chart_schema`). A `chart_schema` problem comes from dbt Charts itself and
carries the engine's own code, such as `ERR-EXTRA-FIELD`.

## Presets

A preset is the settings, themes, widgets and dashboards shipped for a kind
of deployment, as versioned YAML. Resetting an event to a preset writes each
of its documents as a new revision, exactly as saving it in the editor would,
and removes the event's documents the preset does not have; from then on the
configuration is the event's own. Nothing in the platform reads a preset's id
to decide how to count or what to show.

| Preset | Contents |
|---|---|
| `comelec` | An overview and a dashboard for each monitoring record of an overseas voting package, grouped by section in the order an election runs: enrollment (decisions, disapproval reasons, voting credentials), test voting, final testing and lockdown, voting (Posts initialized, opened and closed, status by Post), voter turnout (three ratios, by group, by Post, by country), counting and transmission, enrollment and voting rates, access and security (sign-ins, login outcomes, attack detections) and helpdesk. Time zone `Asia/Manila`; dimensions sex, age band and status abroad. |
| `campus` | Participation and operations dashboards for a university election, with faculty and role dimensions, `Europe/Madrid` time, and its own names for the dashboard selectors. It runs on the same data sources with no code change. |

Presets live in `packages/sequent-core/src/monitoring/presets/<id>/`: a
`preset.yaml` manifest (`id`, `version`, `title`, `description`, and the
requirements met by the Export action or owned by other parts of the
platform), `settings.yaml`, and one file per document under `themes/`,
`widgets/` and `dashboards/`, named after the document's id. Every preset is
validated by the build's tests, and every widget in it is evaluated against
sample figures.
