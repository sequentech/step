---
id: monitoring_scale
title: Monitoring at Scale
sidebar_label: Scale
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

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
