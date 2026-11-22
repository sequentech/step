---
id: election_management_election_event_logs
title: Logs
description: "The Logs tab provides a holistic view of all ongoing activities, offering detailed insights into the system's operations."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


The Logs tab provides a holistic view of all ongoing activities, offering detailed insights into the system's operations.

These are application-level actions being recorded. Access the system log to monitor activity across the entire platform, including voter events, Keycloak events, system events, and user events.

- Select **Columns** in order to hide/show different points of data per log entry.

## Integrity

Log entries are stored in PostgreSQL. Each entry is also committed to a Merkle log for its election event. Through the platform's API and command-line tools, operators can obtain inclusion proofs for individual entries and consistency proofs showing that the log has only been extended since an earlier checkpoint.

These proofs show tampering only relative to a checkpoint saved outside the log. The platform publishes a signed checkpoint when voting closes and when each results tally completes, stored separately from the log and recorded in it as an "Electoral log checkpoint published" entry. You can also save checkpoints independently, for example at the start and end of voting, and verify later proofs against them. Deleting an election event also deletes its log.

## Audit

An audit checks that every log entry matches its Merkle log, that the stored Merkle tree is complete and consistent, and that every published checkpoint is correctly signed and part of the log's history. An audit runs automatically when a results tally completes; its result appears in the tally logs.

- Select **Audit** to start an audit manually. This requires the logs-read and electoral-log-audit permissions.
- The audit runs as an "Audit Electoral Log" task. It succeeds when the log is consistent and fails when it finds a problem, listing each finding in the task logs. Audits only report problems; they never change the log.