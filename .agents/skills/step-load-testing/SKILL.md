---
name: step-load-testing
description: Prepare, run or diagnose bounded Step voting load tests using native step-cli with k6 or Chromium, including multiple GitHub runners and live metrics. Use for capacity/workload measurements, not ordinary browser regression tests.
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Read [the operating guide](../../../docs/docusaurus/docs/07-developers/e2e.md), `packages/voting-load/README.md` and the native `step-cli load reference`. Preserve Rust preparation/encryption/reporting and its immutable shard claims. Do not implement a second protocol/encryption stack in Python or JavaScript.

Use k6 for authenticated HTTP throughput and headless Chromium for full rendering, WASM encryption and UI interaction. They measure different scopes. Keep coverage instrumentation out of capacity measurements. Start with the finite smoke preset, one worker and one concurrent voter; expand within the explicitly registered target limits and current task authorization.

`scripts/e2e load --target NAME --engine k6` uses the enabled synthetic registry. The manual E2E load workflow can split the same frozen cohort across one to four independent GitHub runners. Inputs and worker results are temporary private artifacts; administrator sessions never go to workers. Each voter/shard may be attempted once. Never automatically retry an ambiguous cast.

The target must permit load, have a designated synthetic tenant, and provide explicit HTTPS service/storage origins. Runtime limits, error thresholds, receipt reconciliation, owned election cleanup and monitoring delivery are part of success. No live target is configured by default. Do not manufacture a target or substitute an existing development tenant.

Use the Grafana dashboard for live aggregate metrics from both engines. Sum histogram buckets before computing quantiles; never average worker p95/p99. k6 also exports a per-worker HTML dashboard. An Actions runner's localhost port is not a shared dashboard URL. Prometheus must scrape the authenticated Pushgateway every five seconds; transient run groups are deleted after final scrapes. Stable status groups preserve the last successful probe and latest failed result.

Keep application availability alerts active during load. Missing workers, telemetry failure, exceeded deadlines and failed cleanup must fail the run or alert. Confirm all planned receipts against the read-only audit connection. Publish native aggregate reports and actual total wall time. Report limitations in geographic placement, network shaping or browser behavior before claiming Loadero parity.
