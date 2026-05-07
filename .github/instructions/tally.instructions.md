---
applyTo: "packages/windmill/src/services/cast_votes.rs,packages/windmill/src/services/insert_cast_vote.rs,packages/windmill/src/services/join.rs,packages/windmill/src/services/ceremonies/**/*.rs,packages/windmill/src/postgres/cast_vote.rs,beyond/packages/ballot-audit/**/*.rs,packages/step-cli/src/commands/duplicate_votes.rs"
---
<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Tally And Ballot Selection Review

- Treat any change that influences which ballot is selected, exported, counted, duplicated, or audited as critical.
- For logic over sequent_backend.cast_vote, verify scoping by tenant_id, election_event_id, election_id, and area_id when the operation is area-specific.
- When PostgreSQL DISTINCT ON is used to choose one ballot per voter or area, ORDER BY must begin with the same grouping keys and then use deterministic recency. Flag stale-selection patterns such as ordering only by voter_id_string or using recency without a stable tie-breaker.
- If created_at can tie, require a stable tie-breaker such as id DESC. Otherwise the selected revote can change across runs.
- Latest valid ballot must win consistently across tally export, statistics, duplicate-vote tooling, receipts, and ballot-audit. Flag semantic drift between these paths.
- COUNT(DISTINCT voter_id_string) is not interchangeable with latest-per-voter or latest-per-voter-per-area semantics unless the election rules prove it.
- In revote checks, verify same-area and other-area handling, unlimited-revote behavior, and retry paths. Review whether failures can permit double voting or reject valid revotes.
- For CSV joins and exports, preserve sort and dedup preconditions. merge_join_csv assumes sorted input and currently skips malformed rows; flag changes that can silently drop or misalign ballots and voters.
- Expect tests for stale-ordering regressions, equal-timestamp tie handling, multi-area voters, revote limits, and consistency between tally and audit outputs.
