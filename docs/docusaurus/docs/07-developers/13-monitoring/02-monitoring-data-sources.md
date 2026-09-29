---
id: monitoring_data_sources
title: Monitoring Data Sources and Presets
sidebar_label: Data sources and presets
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

A monitoring data source is a set of governed facts with a fixed counting
unit. Configuration can only pick a source, one of its templates and the
template's parameters, so changing what a source counts, or adding a source,
is a code change reviewed like one. The
[configuration reference](../../02-election_managers/02-reference/12-monitoring-configuration.md#data-sources)
lists the sources as administrators see them; the
[architecture](./01-monitoring-architecture.md) page describes the snapshot
job that runs the producers.

## How a source is declared

`DataSourceId::spec()` in `packages/sequent-core/src/monitoring/sources.rs`
is the only place a source's contract is stated:

| Field | Meaning |
|---|---|
| `counting_unit` | What one unit of its figures is (`DISTINCT_VOTERS`, `POSTS_IN_SCOPE`, `ATTEMPTS`, …). The portal translates it into the help under the data source. |
| `measures` | The numbers it reports. A query may ask only for these. |
| `templates` | The result shapes it offers (`summary`, `by_group`, `by_post`, `timeseries`, `by_measure`). |
| `builtin_dimensions` | Which of `region`, `post`, `country`, `reason`, `state`, `category` it can group by. `region`, `post` and `country` are also the dashboard selectors that can narrow it. |
| `voter_dimensions` | `CONFIGURED` when its rows carry the voter dimensions the event's settings declare, else `NOT_APPLICABLE`. |
| `states` | For a source that counts Posts, the states a Post moves through; `PostState::has_reached` defines each milestone. |
| `producer` | `Available`, or `Pending(PendingProducer)` naming what is missing. |

A `Pending` source is declared so presets can place its widgets today. The
snapshot job records it as `NOT_CONNECTED` with its reason, and its widgets
show "Not connected · reason" and never a zero. Five sources are pending:

| Source | `PendingProducer` | What it waits for |
|---|---|---|
| `test_voting` | `TEST_ELECTION_DESIGNATION` | A way to mark an election as a test election. |
| `voting_credentials` | `CREDENTIAL_ISSUED_EVENT` | A recorded event when credentials are issued. |
| `final_testing_lockdown` | `FINAL_TESTING_LOCKDOWN_STATE` | A recorded final-testing and lockdown state per Post. |
| `attack_detections` | `ATTACK_DETECTION_FEED` | A feed of detected attacks. |
| `helpdesk` | `HELPDESK_INTEGRATION` | A helpdesk system. |

## Turning a pending source on

The steps below connect a declared source. Write each test first; they
should fail until the step is done.

1. **Record the facts.** Make the owning feature store what the source
   counts, in the backend database. Where the fact is per voter, prefer a
   column of `monitoring_voter`: the projection already has
   `credentials_at` and `test_voted_at` for the credential and test-voting
   sources. Anything new in a table goes in a migration, as nullable or
   defaulted columns so the change stays backwards compatible.
2. **Read them in the pass.** Extend `load_facts` in
   `windmill/src/services/monitoring/snapshot.rs` (and the projection, for
   per-voter facts) so the pass reads them inside its REPEATABLE READ
   transaction, into `EventFacts` in `producers.rs`. The pass must read each
   source table once per event, never per viewer.
3. **Produce the payloads.** In `producers.rs`, compute the source's
   `ScopePayload` for every scope and every election set, and return it from
   `produce`:
   - count every scope from its own rows, never by adding up narrower
     scopes, so a voter in two Posts is one voter of their region;
   - count every measure the spec lists at every scope (a missing measure
     reads as "not counted", never as zero);
   - put missing dimension values under `__unknown__`;
   - for a source with a `timeseries` template, add the measures it puts in
     series to `series_measures`, bucketed by the hour in the settings' time
     zone;
   - for a Post source, give each Post exactly one state, so the states add
     up to the Posts in scope.
   Test it in `producers_tests.rs` and, against PostgreSQL, in
   `windmill/tests/postgres_monitoring_snapshot.rs`.
4. **Mark it available.** In `sources.rs`, change the source's `producer` to
   `Producer::Available`. Keep the `PendingProducer` variant: stored runs
   and older readers still name it. If the source needs a new measure,
   dimension or state, add it as a new variant; the migration's `CHECK`
   lists name every source, so a new source (not a newly connected one)
   needs a migration deployed before the code that writes it.
5. **Keep the samples true.** `sample.rs` builds the made-up payloads used by
   previews, save checks, the preset tests and the renderer tests; make its
   payload for the source match what the producer now writes.
6. **Check the presets.** The widgets for the source already exist in the
   presets; run the sequent-core preset tests, and check the widgets on a
   running stack (see below). Widgets need no change unless the source's
   contract did.

Rolling deploys are safe in both directions: an older Harvest drops a
measure, state or notice it cannot name rather than refusing the payload.

## Adding a preset

A preset is data, never code. To add one, create
`packages/sequent-core/src/monitoring/presets/<id>/` with:

- `preset.yaml`: `id`, `version` (start at 1), `title`, `description`, and
  optionally `export_requirements` and `owned_elsewhere`;
- `settings.yaml`: the time zone, where region and country are read, who is
  pre-enrolled, the voter dimensions and, if wanted, the selector words;
- `themes/default.yaml` and any other themes;
- one file per widget under `widgets/` and per dashboard under
  `dashboards/`, each named after its `id`.

The build script lists the directories and compiles the files in, so the
Admin Portal, Harvest and the tests read the same bytes. Record the preset
in `fixtures/preset_versions.txt` by running the sequent-core preset tests
once with `MONITORING_UPDATE_PRESET_VERSIONS=1`. Afterwards, raise `version`
whenever a document changes, and rewrite the file the same way; the tests
fail on a changed preset whose version was not raised. Nothing may
select code by preset or customer id.

## Checking a source end to end

1. Start the backend stack with the renderer (`monitoring-renderer` is in the
   `base` and `full` compose profiles) and an election event with voters.
2. Reset the event to a preset with the `monitoringResetToPreset` action,
   which also switches its Dashboard tab to configured mode.
3. Within about 30 seconds the beat's first pass completes and the
   Dashboard tab shows the figures. Cast votes or decide applications and
   check that the next pass updates them, that ratios and buckets reconcile
   with the totals, and that a label-restricted administrator sees only the
   permitted Posts.
4. Check that the source's widgets are no longer "Not connected".
