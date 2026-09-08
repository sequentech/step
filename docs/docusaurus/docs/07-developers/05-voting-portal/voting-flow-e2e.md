---
id: voting-flow-e2e
title: Voting journey observability
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

The [voting load runner](./voter-status-performance.md) records client request counts in each run's `results.json`. Set `workload.trace_http: true` before preparation to record sanitized k6 fetch diagnostics. The summary report concentrates on outcomes and performance.

## What the journey exercises

1. Keycloak authorization, username/password authentication, and PKCE token exchange.
2. Minimal `GetVoterStatus` GraphQL read for eligibility, cast status and signed publication URLs.
3. S3 event, election, summary and ballot-style downloads.
4. `InsertCastVote` and verification of the returned ballot receipt.

Chromium also loads portal assets, renders the election list, selects candidates and encrypts through WASM. k6 uses native ciphertexts prepared before execution. A Chromium capture is never a prerequisite for a k6 run.

The internal capture modules can collect browser HAR and database statement-log intervals for development diagnostics. They share the browser flow and protocol definitions with the finite runner. SQL intervals may include background work; a browser request inventory is not a trace of every internal Hasura query.

For independent receipt persistence checks, use the [report audit command](./voting-load-results.md#regenerate-audit-or-capture-a-report). Keep raw logs, census data, signed URLs and ballots private.
