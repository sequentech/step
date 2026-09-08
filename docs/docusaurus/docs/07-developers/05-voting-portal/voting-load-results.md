---
title: Read load-test results
sidebar_position: 9
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Open `report.html` from the run directory. It is a self-contained document suitable for sharing or printing to PDF: successful journeys, accepted casts per second, response distributions and configured goals.

![Example voting-load report captured from an actual local run](/img/voting-load-report.png)

The image illustrates the report layout, not deployment capacity.

## Interpret the measurements

| Measure | Meaning |
| --- | --- |
| Successful journeys | Distinct voters that completed the configured journey |
| Accepted casts/s | Unique API receipts divided by the interval from the first journey start to the last completion |
| p50 / p99 | Global percentiles of individual samples across all workers |
| Duration | Measured journey interval, excluding census generation and encryption preparation |
| Goals | Explicit thresholds from the workload configuration |

k6 includes authentication, voter status, publication downloads and cast acceptance. Chromium additionally includes rendering and browser encryption. Compare runs using the same engine and workload. Status-only reports show successful journeys per second because they cast no votes.

Every planned journey must succeed; voting runs also require unique receipts. A missing worker, failed journey or missed threshold makes the run fail. Reports are still produced for partial runs. An API receipt confirms acceptance; it does not independently prove database persistence or successful tallying.

## Regenerate, audit or capture a report

```bash
step-cli load report runs/smoke \
  --open
step-cli load report runs/smoke \
  --screenshot report.png
```

Screenshot capture needs Playwright and Chromium configured in `runtime`; ordinary HTML reporting does not. Screenshot dimensions are in `reporting`.

For an optional read-only receipt audit:

```bash
read -rs -p 'Read-only backend PostgreSQL DSN: ' LOAD_AUDIT_DSN
export LOAD_AUDIT_DSN
step-cli load report runs/smoke \
  --dsn-env LOAD_AUDIT_DSN
unset LOAD_AUDIT_DSN
```

## Investigate a failure

The run contains `settings.yaml` (effective configuration), `setup/` (private provisioning logs), `inputs/` (publication and encrypted shards), and `inputs/results/` (worker logs, samples and attempt markers). Census CSV batches and import checkpoints live in `setup/census/`, including runs that reuse an existing event.

`results.json` retains the request inventory for diagnostics. Enable `workload.trace_http` before preparation for sanitized per-fetch protocol logs. These details stay out of the summary report. Keep raw logs and inputs private: they may contain voter credentials, signed URLs and ballots.

If preparation fails, inspect `setup/setup.log` and `setup/setup-state.json` before creating another election. If execution fails, inspect the affected worker log and reconcile accepted receipts. Preserve the run and prepare a new range; deleting attempt markers risks duplicate casts.

## Remove a synthetic election

After retaining the report and reconciling failures, delete the event created by that run. Its ID is `election_event_id` in `inputs/config.json`, or in `setup/setup-state.json` if preparation stopped early. Authenticate the CLI against the same tenant first.

```bash
read -r -p 'Synthetic election event ID from this run: ' LOAD_EVENT_ID
step-cli step delete-election-event \
  --election-event-id "$LOAD_EVENT_ID"
```

A run configured with `preparation.existing_event` does not own that event; retain it for its other users. Deleting an election does not remove the local report or Kubernetes resources.
