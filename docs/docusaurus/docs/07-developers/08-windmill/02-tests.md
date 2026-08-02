---
id: windmill_tests
title: Testing
sidebar_position: 2
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Testing

:::note
All commands below assume you are already inside the dev container (VS Code
opened in the Dev Container or GitHub Codespaces). The dev environment,
including all required environment variables, is set up automatically.
:::

## Activity Log Tests

The tests described here live in
`packages/windmill/src/services/reports/activity_log.rs`.

### test_generate_export_csv_data_120k_memory

Verifies the large-scale CSV export path (`generate_export_csv_data`) under a
realistic load and measures its heap usage. The test:

1. Reads `tenant_id` and `election_event_id` from the external config file.
2. Creates a fresh ImmuDB database (using a time-stamped slug so each run gets a
   clean database — ImmuDB's delete is unreliable).
3. Seeds 120 000 log entries by calling the `step-cli` binary.
4. Wraps the call to `generate_export_csv_data` with a
   [dhat](https://docs.rs/dhat) heap profiler so peak memory usage is recorded.
5. Asserts that the produced CSV file contains exactly 120 000 data rows (plus
   the header), catching any silent truncation in the batching logic.

This test is marked `#[ignore]` (see [Running ignored tests](#running-ignored-tests)).

---

## Running Ignored Tests

This test is annotated with `#[ignore]` because it:

- Requires a live ImmuDB instance and Keycloak database.
- Seeds 120 000 entries — expensive in time and resources.
- Is therefore skipped in GitHub Actions CI.

To run it locally, pass `--ignored` together with `--nocapture` so that the
progress output is visible:

```bash
cd /workspaces/step/packages/windmill && \
  cargo test --release \
    test_generate_export_csv_data_120k_memory \
    -- --nocapture --ignored
```

---

## PostgreSQL-backed voter-channel test

`voters_by_channel_defaults_legacy_votes_and_uses_latest_valid_revote` is
ignored by the default `cargo test` command because it requires PostgreSQL and
the `HASURA_DB__*` connection variables. GitHub Actions runs it in the dedicated
Windmill PostgreSQL job. In a configured dev container, run it with:

```bash
cd /workspaces/step/packages/windmill && \
  cargo test \
    services::cast_votes::tests::voters_by_channel_defaults_legacy_votes_and_uses_latest_valid_revote \
    -- --ignored --exact
```

---

## Memory Tests

### What they measure

`test_generate_export_csv_data_120k_memory` uses `dhat::Profiler::new_heap()`
to record all heap allocations that occur **inside `generate_export_csv_data`**.
When the profiler is dropped at the end of the test, it writes a
`dhat-heap.json` file to the current working directory and also prints a
summary to stdout:

```
Peak live heap:   <N> bytes
Total allocated:  <N> bytes in <N> blocks
Current live:     <N> bytes in <N> blocks
CSV file size:    <N> bytes
CSV line count:   <N> lines (<N> data rows)
```

The goal is to confirm that the streaming-batch approach keeps the **peak live
heap** low even for very large electoral logs, rather than loading all 120 000
entries into memory at once.

### Viewing the dhat-heap.json file

After the test finishes, open the
[DHAT Viewer](https://nnethercote.github.io/dh_view/dh_view.html) in a browser
and upload the `dhat-heap.json` file. It provides a flame-graph-style breakdown
of allocation sites, showing which call paths consumed the most memory.

The file is written to whichever directory Cargo uses as the current working
directory when running tests (typically `packages/windmill/`).

### Why this test is ignored in CI

Running it in GitHub Actions would:

- Require a live ImmuDB + Keycloak environment in the CI runner.
- Seed 120 000 log entries per run, making the job slow and expensive.
- Produce a `dhat-heap.json` artefact that is only meaningful when reviewed
  manually.

It is therefore excluded from CI and intended to be run locally during
development when investigating memory regressions.

---

## Seeding Electoral Logs with step-cli

The test above internally calls the `step-cli` binary to seed log entries into
ImmuDB. You can also run that command manually — for example, to load log
entries into an existing election event's electoral log for manual QA or
performance testing.

### 1. Build step-cli in release mode

You can skip this step if the binary is already there.
```bash
cd /workspaces/step/packages/step-cli && \
  cargo build --release
```

The binary will be at:
`packages/step-cli/rust-local-target/release/step-cli`

### 2. Set the election_event_id in external_config.json

Open `packages/step-cli/data/external_config.json` and update the
`election_event_id` field to match the target election event. This is the
**only field that normally needs to be changed** — the other IDs (`tenant_id`,
`area_id`, `election_id`, `realm_name`) should already match the event in your
development environment.

```json
{
  "election_event_id": "<your-election-event-id>",
  ...
}
```

### 3. Run the command

```bash
/workspaces/step/packages/step-cli/ \
  step create-electoral-logs \
  --working-directory data/ \
  --num-logs 120000
```

The command reads `tenant_id`, `election_event_id`, `election_id`, `area_id`,
and `realm_name` from `external_config.json`. It queries Keycloak to obtain
real user IDs and usernames and inserts synthetic `SendCommunications` log
entries in batches of 1 000 rows.

Open election event's LOGS tab in the Admin Portal to confirm the new entries.
