---
id: voting_lifecycle_overview
title: Voting lifecycle overview
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Prepare an event's zones, schedule and lifecycle requirements together. Review
what the published configuration authorizes before opening voting, and verify
the recorded result at each deadline.

| Task | Guide |
| --- | --- |
| Configure zones, local times and CSV schedules | [Timezones and schedules](./17-timezones-and-schedules.md) |
| Understand signed schedule coverage, policy changes and retained close deadlines | [Scheduled openings and closings with signatures](./18-signed-scheduled-transitions.md) |
| Conduct and recover an initialization ceremony | [Initialization ceremony runbook](../../01-tutorials/11-admin_portal_tutorials_initialization-ceremony.md) |
| Generate the dated initialization document | [Initialization Report](../../01-tutorials/11-admin_portal_tutorials_initialization-report.md) |
| Operate registration windows and migrate existing realms | [Per-Post enrollment windows](../../../07-developers/06-keycloak/enrollment_windows.md) |
| Deploy the event-timezone migration and review transmission changes | [Event timezones migration](../../../07-developers/11-updates/event-time-zones-migration.md) |
| Seal each ballot box at close, and run the tally gate | [Ballot box seal](./20-ballot-box-seal.md) and its [runbook](../../01-tutorials/22-admin_portal_tutorials_ballot-box-seal-runbook.md) |
| Inspect signing execution, persistence and audit delivery | [Signing architecture](../../../07-developers/14-signing/01-signing-architecture.md) and [API](../../../07-developers/14-signing/02-signing-api.md) |

Defaults preserve imported older data where fields are optional. Legacy reads
with unusable timezone settings can fall back to UTC; that fallback is a recovery
behavior, not a valid new event configuration. Review migrated events and repair
invalid or offset-less schedule rows before relying on them.

Timezones define presentation and conversion; they do not extend voting periods.
Initialization gates openings, and the common close remains effective when a
Post initializes late. Check per-Post overrides and signed snapshots when
explaining a deadline. Read the actual outcome and audit details instead of
assuming that a saved schedule already ran.
