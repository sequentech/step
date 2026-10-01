---
id: monitoring_scale
title: Monitoring at Scale
sidebar_label: Scale
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Measured sizes

Monitoring is measured at the sizes an election has, up to tens of
thousands of voters, votes, applications and sign-in counters, at each
stage: the snapshot pass and its storage, the widgets' computation and
drawing, the export, and the Admin Portal. Each stage has a test that seeds
or builds data of that size and holds it to a budget, so a regression fails
a test rather than an election.

### The snapshot pipeline

`packages/windmill/tests/postgres_monitoring_scale.rs` seeds a synthetic
event in bulk into the test database (a private `windmill_schema_*`
database, as for the other windmill database tests; never the dev
database): 100 Posts in 10 regions and 150 countries, 105 areas, votes over
a week with 10% revotes and some discarded or unfinished ballots,
applications for a third of the voters (some applying twice), 15-minute
sign-in counters over the week, and a Keycloak-shaped realm with the voter
group, the comelec attributes and credentials, in tables carrying
Keycloak's indexes. It resets the event to the comelec preset and runs the
snapshot job's own pass:

1. a first pass, which reads every account;
2. the activity refresh on its own, which finds nothing to change;
3. a pass with nothing changed, which finds nothing it counts from moved
   and counts nothing;
4. a pass after 1% of the voters voted;
5. a full pass over the accounts with nothing changed, which counts
   nothing either;
6. pruning every run but the current one;
7. a CSV and a SQL export of each of the 21 comelec dashboards.

Every step has a budget of a base plus an amount per 10,000 voters, for a
release build; a debug build gets 16 times as much, and
`MONITORING_SCALE_BUDGET_FACTOR` stretches them further on a slow machine.
The test prints a `[scale]` line per step, the figures and payload bytes
written, the group sizes of the event-scope payloads, and the process's
peak memory.

It reaches the test database as the other windmill database tests do (the
`HASURA_DB__*` variables, set in the devenv shell; see the architecture
guide).

```sh
cd packages
# 1000 voters, debug: the size that runs with the other database tests.
cargo test -p windmill --test postgres_monitoring_scale -- --nocapture

# Several sizes, release.
MONITORING_SCALE_VOTERS=1000,10000,50000 \
  cargo test --release -p windmill --test postgres_monitoring_scale -- --nocapture
```

Measured on a development machine, release build:

| Step | 1,000 voters | 10,000 voters | 50,000 voters |
| --- | --- | --- | --- |
| First pass | 0.8 s | 4.5 s | 16.4 s |
| Activity refresh, nothing changed | < 0.1 s | < 0.1 s | 0.15 s |
| Pass, nothing changed | < 0.1 s | 0.1 s | 0.5 s |
| Pass after 1% voted | 0.3 s | 1.3 s | 4.3 s |
| Full pass, nothing changed | < 0.1 s | 0.5 s | 4.5 s |
| Prune | < 0.1 s | < 0.1 s | 0.1 s (see below) |
| Export, 21 dashboards, CSV and SQL each | 0.1 s | 0.1 s | 0.2 s |
| Figures stored by the first pass | 2,775 | 10,671 | 27,940 |
| Payload text stored by the first pass | 12 MB | 40 MB | 114 MB |
| Largest payload | 55 KB | 57 KB | 58 KB |
| Figures and payload text a 1% change writes | 86, 1 MB | 565, 10 MB | 1,278, 25 MB |
| Peak memory of the test process | 89 MiB | 208 MiB | 542 MiB |

A debug build takes 4 to 14 times as long: at 1,000 voters about 5 s for a
first pass and 4.5 s for the others.

What these show:

- **Payloads stay small.** A payload holds one figure's groups, Posts, cube
  cells and time series, never voter rows. At 50,000 voters the event-scope
  turnout payload has 100 Posts, 150 countries, 10 regions, 20 cube cells
  and 169 series buckets; the largest payload is under 60 KB. About 96% of
  payload bytes are time series. PostgreSQL compresses payload text about
  five times on disk.
- **A pass writes what changed.** An unchanged pass writes nothing; after
  1% of the voters vote, about 5% of the figures get a new payload.
- **An unchanged pass counts nothing.** It reads digests of everything it
  counts from in one statement, after the projection's refresh, and,
  finding the shown run's, only marks that run checked. Before, it computed
  every figure, compared them with the open ones and wrote nothing: 0.9 s
  at 10,000 voters and 3.2 s at 50,000, and a full pass over unchanged
  accounts 1.2 s and 8 s. What is left of the full pass is reading the
  accounts from Keycloak.
- **Pruning right after bulk writes can be slow.** The first prune after
  the passes has, in some runs, taken up to 13 s at 50,000 voters: the
  foreign-key and payload-kept checks of each deleted row are planned from
  table statistics that do not yet count the rows the passes wrote. Once
  autovacuum analyzes the tables it takes a tenth of a second, as in the
  table. Pruning holds the event's lock, so it delays that event's next
  pass, and no other.
- **The export is small.** Every comelec dashboard at 50,000 voters exports
  to 238 KB of CSV (384 KB of SQL) in total; the largest dashboard is 39 KB.

Before the voter activity refresh was rewritten, it joined the voter rows
with the latest applications and first votes in one statement, which the
planner answered with nested loops while the statistics lagged. It took
11 s of a 15 s first pass at 10,000 voters, and later passes took 30 to
39 s until autovacuum caught up. It now reads the three sets once, compares
them in memory and writes the changed rows in batches.

### Widgets and the renderer

`packages/sequent-core/src/monitoring/presets_scale_tests.rs` builds
payloads of a large event (every Post, 150 countries, a week of hourly and
15-minute buckets) and computes and draws every widget of every preset with
them, checking each against the renderer's limits: at most 5,000 rows, a
request body of at most 256 KiB, and 4 s to draw. It keeps the largest
boards in `packages/sequent-core/src/monitoring/fixtures/scale/`, and
`packages/monitoring-renderer/tests/test_scale.py` has the renderer accept
and draw each of them in time. `MONITORING_UPDATE_BOARDS=1` rewrites the
files after a change to the presets.

```sh
cd packages
cargo test -p sequent-core --features monitoring --lib presets_scale -- --nocapture
# then, from packages/monitoring-renderer, as in the architecture guide:
python -m pytest -q tests/test_scale.py
```

No widget comes near a limit. Computing a widget takes at most about 11 ms
in a debug build; the largest request body is 70 KB; the slowest drawing
is about 1 s, for a week of 15-minute buckets (a 710 KB SVG), and a
151-value dimension draws a 384 KB SVG. The request body limit would bind
at about 2,700 hourly rows, well past a week.

### The Admin Portal

The View data table pages its rows: 100 by default, with 250 and 1,000 to
choose from, and no pager for 100 rows or fewer. At 10,000 rows it opened
in 3.5 s before and in 0.1 s after, in the tests' jsdom. Drawn charts are shown in a frame at
most twice the widget's height, whatever their size. Sanitizing an SVG
before showing it takes about 1 s for 2 MB and 2.5 s for 5 MB in Chromium,
which is larger than any widget the renderer draws for an election.

```sh
cd packages/admin-portal
yarn jest src/components/monitoring/MonitoringDataTable.test.tsx \
  src/components/monitoring/lib/chartDocument.scale.test.ts
```

The `ManyRows` story of `MonitoringDataTable` shows a table of more than a
page.

## Seeding a dev stack at scale

`step-dev monitoring_scale` fills an existing election event of the
checkout's running stack with synthetic voters carrying the attributes the
monitoring projection reads, and optionally with cast votes and enrollment
applications, so that the snapshot job and the dashboards can be measured
with thousands of voters. `--cleanup` removes exactly what a seed created.
Run it in the devcontainer (it reaches the Compose services by name), with
the backend mode up, against a throwaway event, such as one a scenario
creates (`step-dev scenario list`, then `scenario up <name>`; `scenario urls
<name>` prints its event id). `scenario reset <name>` deletes that event with
its realm, which also disposes of everything a seed put in it.

The event only needs areas linked to contests; the command reads its areas,
contests and elections from the database and assumes no fixture. The tenant
is the stack's `SUPER_ADMIN_TENANT_ID`, the only one its administrator can
import into; `--tenant` must match it when given.

### Commands

```sh
# The plan, without contacting the stack.
scripts/dev/step-dev monitoring_scale --event <event-id> --voters 10000 \
  --with-votes --with-applications --dry-run --offline

# The plan against the event: reads it, writes nothing.
scripts/dev/step-dev monitoring_scale --event <event-id> --voters 10000 \
  --with-votes --with-applications --dry-run

# Seed it.
scripts/dev/step-dev monitoring_scale --event <event-id> --voters 10000 \
  --with-votes --with-applications

# What a cleanup would delete, then delete it.
scripts/dev/step-dev monitoring_scale --event <event-id> --cleanup --dry-run
scripts/dev/step-dev monitoring_scale --event <event-id> --cleanup
```

| Flag | Default | Meaning |
| --- | --- | --- |
| `--voters N` | 1000 | Voters to create. Above 50000 needs `--force`. |
| `--with-votes` | off | Also insert valid cast votes (see the caveat below). |
| `--with-applications` | off | Also insert enrollment applications. |
| `--turnout` | 0.6 | Share of voters who vote. |
| `--revote-share` | 0.1 | Share of voting voters who vote again, where the election allows it. |
| `--vote-days` | 5 | Days before now the votes spread over. |
| `--application-share` | 0.3 | Share of voters with an application. |
| `--import-batch` | 10000 | Voters per imported file. |
| `--db-batch` | 1000 | Rows per Hasura insert. |
| `--workers` | 8 | Parallel Keycloak deletions in `--cleanup`. |
| `--seed` | 13624 | Random seed; the same seed and areas give the same voters. |
| `--password` | `Mon-scale-2026!` | The synthetic voters' password. |
| `--force` | off | Allow more than 50000 voters, or votes in an event that has a tally session. |
| `--dry-run` | off | Print the plan and what is already seeded; write nothing. |
| `--offline` | off | With `--dry-run`: plan for `--areas` made-up areas without contacting the stack. |

A real run prints how long each phase took (checks, each imported file,
reading the voters back, the vote and application inserts), so the importer
is measured along the way.

### What it creates

- **Voters**, through the platform's own importer: the command writes CSV
  files and runs `step-cli step import-voters --is-local` on each, as the
  scenarios do. The importer COPYs the rows straight into the event realm's
  Keycloak tables in one transaction per file, which is far faster than the
  admin REST API. The CSV carries a password hash computed once by the
  command, so the importer does not hash every row. Each voter is:
  - named `mon-scale-000000`, `mon-scale-000001`, ... with the email
    `<username>@mon-scale.invalid`, in the `KEYCLOAK_VOTER_GROUP_NAME` group;
  - in one of the event's areas (`area-id`), spread unevenly; only areas that
    vote in an election are used, and areas sharing a name are left out
    because the importer finds areas by name;
  - given the comelec preset's attributes: `country` as `Country/Post`
    across 150 countries with a long tail (1% empty), `sex` (`M`, `F`, 2%
    empty), `dateOfBirth` spread over the four age bands,
    `landBasedOrSeafarer`, and `sequent.read-only.id-card-number-validated =
    VERIFIED` for about 40% (pre-enrolled).
  Every account gets a credential at import time, so they share one
  `credentials_at`.
- **Votes** (`--with-votes`): one `valid` `sequent_backend.cast_vote` row per
  voting voter and election of their area, at a time within the last
  `--vote-days` days (busier later and in the afternoon), plus a later second
  vote for `--revote-share` of them in elections whose `num_allowed_revotes`
  is 0 or at least 2, so the revote trigger accepts it. That is what the
  projection reads: the earliest `valid` vote per voter and election.
- **Applications** (`--with-applications`): one `sequent_backend.applications`
  row per applicant, `ACCEPTED` (60%), `REJECTED` with a
  `rejection_reason` annotation (25%) or `PENDING` (15%), dated before and
  during the votes.

Votes and applications are inserted through Hasura with the admin secret and
carry the `mon-scale-seed` annotation. Sign-in attempts are not seeded: the
login counters are aggregates without a per-row mark to clean up by.

The command refuses to run when the event or its realm is missing, when the
realm lacks the voter group, when the realm already holds a different number
of `mon-scale-` voters, or when seeded votes or applications already exist;
run `--cleanup` first. Rerunning with the same `--voters` skips the import,
so votes can be added to an earlier seed.

### Cleanup

`--cleanup` deletes the event's votes and applications annotated
`mon-scale-seed`, then every account of the event realm whose username starts
with `mon-scale-` and whose email ends with `@mon-scale.invalid`, through the
Keycloak admin API. Nothing without those marks is touched. The next full
monitoring pass removes the deleted voters from `monitoring_voter`.

### Caveats

- **Seeded votes break a tally of the event.** They have no ballot content or
  signature and are not on the bulletin board, which is why `--with-votes` is
  opt-in and refused for an event that already has a tally session unless
  `--force` is given. Use a throwaway event, or clean up before tallying.
- The seeded voters can sign in with `--password`, but a seeded vote is not
  a real ballot: do not use the seeded voters for voting journeys.

### Seeing the result in monitoring

The snapshot job only refreshes events whose dashboards are configured
(`monitoring_event.dashboard_mode = CONFIGURED`, which resetting the event to
a preset, such as comelec, sets). Beat sends `refresh_monitoring_snapshots`
every `monitoring_snapshot_interval` seconds (30 by default), which queues a
`refresh_monitoring_event_snapshot` pass per configured event. A pass reads
the realm's voters only on a full pass: when there is no projection yet, when
the event's monitoring settings changed, and otherwise every
`MONITORING_VOTER_FULL_PASS_SECONDS` (300 by default; the dev Compose file
does not set it). So allow up to about five and a half minutes after the
import before the voters appear; votes and applications are then picked up on
every pass. The windmill log shows `Refreshed the monitoring voters` with the
rows written, and `Monitoring snapshot pass` when a pass ends.
