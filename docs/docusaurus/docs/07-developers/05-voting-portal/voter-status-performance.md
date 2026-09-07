---
id: voter-status-performance
title: Voter status performance
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

`GetVoterStatus` is the authenticated live read used by the voting portal. It returns authorized publication references and existing cast status; election descriptions and ballot content come from private S3.

## Measure the status API

Set `mode: "status"` in a Chromium capture target. Configure `performance: {"samples": 100, "warmup": 10, "concurrency": 4, "max_p95_ms": 100}`, then run:

```sh
python3 scripts/voting_e2e/capture.py PRIVATE_STATUS_TARGET PRIVATE_RESULT
```

The runner logs in through the actual portal, observes its `GetVoterStatus` request and uses that authenticated query for repeated read-only calls. It excludes login and warm-up requests from measured API latency. No votes are cast. HTTP errors, GraphQL errors, unexpected response shape and budget breaches fail the test.

Inspect the private `capture.json` and generated `report.md` for the sample count, latency and request inventory. Browser captures observe portal-facing endpoints; an optional action proxy observes Hasura's call to Harvest. SQL summaries cover the capture interval, so background statements may be included.

Use [Chromium journey tests](./voting-flow-e2e.md) for login, election selection, encryption and confirmation. Use the [derived k6 profile](./prepared-vote-load.md) to measure the whole HTTP workload with multiple voters and workers.
