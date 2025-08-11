---
id: election_management_election_event_logs
title: Logs
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


The Logs tab provides a holistic view of all ongoing activities, offering detailed insights into the system's operations.

These are application-level actions being recorded. Access the system log to monitor activity across the entire platform, including voter events, Keycloak events, system events, and user events.

- Select **Columns** in order to hide/show different points of data per log entry.

## Immutability

The Logs tab is implemented using Immudb, a tamper-evident database. Each entry is inserted into a SQL table of actions. The immutability allows to verify that no change has been performed, for anyone with credentials to access to the immutable log.