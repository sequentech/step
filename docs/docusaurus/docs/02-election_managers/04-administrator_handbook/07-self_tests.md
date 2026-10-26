---
id: self_tests
title: Background Integrity Checks and Self-Test Requirements
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## What is available

The implemented integrity check is **Audit electoral log**. It checks the
election event's stored log records, Merkle tree, checkpoint signatures and
configured write-once checkpoint copies. It also checks sealed logs continued
when an election event was imported.

The audit is a **background task**. Starting it returns a task identifier;
closing the dialog or navigating to another page does not cancel the work.
The audit does not hold a browser request open while it scans the log.

:::warning Required self-test coverage
The six-check self-test suite required by the Security Target (FPT_TST.1) is
not implemented as a complete suite in this source version. There is no
"Self-tests" tab, "Run self-test" Dashboard action, or `self-test-*` permission.
An electoral-log audit is a narrower integrity check and does not establish
that all six checks passed. Do not approve the evaluated configuration on the
basis of this audit alone.
:::

## Ask the election board to run an audit

Only the election board may read or audit electoral records. Administrators
must not grant themselves those permissions to perform this check.

1. Ask a board member to open the election event and select **Logs**.
2. The board member selects **Audit** and confirms **Audit electoral log**.
   The permission is `electoral-log-audit`.
3. Open the election event's **Tasks** tab and find **Audit Electoral Log**.
   The task initially shows **In progress**. Other work may continue while
   the audit runs.
4. Wait for a terminal result and inspect its details. **Success** means the
   implemented log checks found no inconsistency. **Failed** means the check
   found an integrity problem or could not finish; neither is a clean result.

If the task fails, the board records its identifier and details and asks the
administrator and server operator to investigate. Do not delete records,
regenerate checkpoints or repeatedly press Audit to make the error disappear.
The worker refuses a concurrent scan of the same event. A queued request can
run after the earlier scan finishes; wait for the existing task's result before
requesting another.

## Configure regular background audits

The operator configures the following environment settings for **both Beat
and the Windmill workers** before deployment. These settings control the
implemented log audit, not the missing complete self-test suite.

| Setting | Default | Accepted values | Meaning |
| --- | --- | --- | --- |
| `ELECTORAL_LOG_AUDIT_INTERVAL_SECS` | `900` (15 minutes) | `60` to `3600` seconds | How often Beat queues a sweep of election events with an open voting channel. |
| `ELECTORAL_LOG_AUDIT_TIMEOUT_SECS` | `60` seconds | `10` seconds to the configured interval | Maximum time allowed for one event's scan. A timeout is a failure, never a clean result. |

The scan time limit includes waiting for a database connection, beginning the
transaction and acquiring the audit lock. Sweep setup and task/result database
operations have separate 10-second limits, so a stalled connection cannot
leave a worker waiting without a deadline.

The worker records a failed task when it can reach result storage. If that
storage is unavailable, the worker fails but the Tasks tab may still show
**In progress**. Investigate a task that exceeds its expected duration; a
pending status does not mean that the scan is still running or has passed.

The Compose definitions pass these values from the operator's environment.
For the evaluated cluster, set the same values in the platform configuration
used by the `beat` and `windmill` Deployments. Invalid values prevent Beat from
starting; a worker with invalid values fails the requested audit. Confirm both
Deployments are ready and inspect the first scheduled task after changing them.

Each periodic sweep scans events **one at a time**; manual audits of different
events may run concurrently. A database lock prevents
overlapping sweeps, and an old queued sweep expires after one minute. These
controls limit duplicate work; they do not guarantee that a long queue or a
large event finishes within the configured interval. Check the actual task
times. A missing, delayed or timed-out audit needs investigation.

Choose the interval with the election organizer. For a one-day election or
high load, start with 5–15 minutes; for a multi-day election, 15–30 minutes.
Never exceed 60 minutes. Measure the duration on representative data before
voting opens. If audits cannot finish within the required interval, provide
resources or reduce competing background work; do not treat a longer interval
as equivalent protection.

## Required checks still to be completed

The ST requires the complete suite at startup, at the election period start,
before starting or resuming voting, after leaving maintenance mode, on an
authorized request, and regularly. The current log audit does not implement
these lifecycle gates or the required automatic maintenance/failure state.

| ST check | Implemented evidence and remaining requirement |
| --- | --- |
| Ballot boxes | Log records and checkpoint history are audited. Ballot-box sealing and verification exist separately. A complete check of both ballot boxes with the required unrecoverable-failure response is still required. |
| Imported election data | Signed-package validation exists at import. Periodic comparison of all live election data with the approved package is still required. |
| Audit generation | The audit verifies existing records. A test record's write/read-back and integration with the complete suite are still required. |
| Trusted-channel interfaces | AGD-04 checks the deployed TLS endpoint. An automatic runtime test of the required TLS profile is still required. |
| Access-control mechanisms | Route and data permissions exist. A runtime integrity comparison against the approved access-control baseline is still required. |
| Clock synchronization | AGD-04 configures and checks NTS/chrony. A TSF runtime synchronization check and its failure response are still required. |

These requirements remain unchanged. Separate deployment checks, a green
background task and a healthy server are not substitutes for the complete
FPT_TST.1 suite or its failure-handling requirements.
