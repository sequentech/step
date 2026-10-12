---
id: election_management_election_event_logs
title: Logs
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


The Logs tab provides a holistic view of all ongoing activities, offering detailed insights into the system's operations.

These are application-level actions being recorded. Access the system log to monitor activity across the entire platform, including voter events, Keycloak events, system events, and user events.

- Select **Columns** in order to hide/show different points of data per log entry.

## Immutability

The Logs tab reads native wbraid audit storage in a private PostgreSQL database.
Deliveries append opaque batches; a recoverable SQL index supports queries.
Application electoral-log messages retain their sender and system signatures.
Ballot-box seals bind the stored ballots at close.

Integrity checks use those signatures and seals. Native storage does not provide
the retired database's Merkle verification protocol, and database access alone
does not establish that the complete history is unchanged. Preserve the native
blobs, application database and signing keys in backups. See
[storage and delivery behavior](../../../07-developers/03-development-environment/testing/electoral-log.md).
