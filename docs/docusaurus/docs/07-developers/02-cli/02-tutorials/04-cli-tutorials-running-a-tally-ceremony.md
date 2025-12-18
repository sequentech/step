---
id: cli_tutorials_running_a_tally_ceremony
title: Running a Tally Ceremony with the CLI
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
This guide walks you through configuring your environment and executing a **Tally Ceremony**.

---

## Prerequisites

* Ensure your environment is configured with the correct credentials.
  Use the [`step config`](../getting-started/cli_tutorials_getting_started.md#configuration)  command to point to your specific tenant and Keycloak instance.
  Make sure you authenticate with an **admin user**.

* Ensure the trustees containers are running:

```bash
cd .devcontainer
docker compose up -d --no-deps beat trustee1 trustee2
```

---

## Start the Tally Ceremony

Run the following command to start a tally ceremony:

```bash
cargo run step start-tally \
  --election-event-id <ELECTION_EVENT_ID> \
  --election-ids <ELECTION_ID> \
  --tally-type <ELECTORAL_RESULTS|INITIALIZATION_REPORT>
```

* `--election-event-id` – Election event ID associated with the tally **(required)**
* `--election-ids` – Election IDs to tally *(repeatable, optional)*
* `--tally-type` – Type of tally to perform **(required)**

Once successful, the command outputs a **Tally Ceremony ID**.
Save this ID for use in the next step.

---

## Confirm Trustee Key for Tally Ceremony

After the ceremony has started, each trustee must confirm their key **once**.

> ⚠️ This command must be executed separately by **every trustee**.
> Before running it, re-run the [`step config`](../getting-started/cli_tutorials_getting_started.md#configuration)  command to authenticate as the specific trustee.

```bash
cargo run step confirm-key-tally \
  --election-event-id <ELECTION_EVENT_ID> \
  --tally-id <TALLY_ID>
```

* `--election-event-id` – Election event ID **(required)**
* `--tally-id` – Tally ceremony ID (returned from the start step) **(required)**

---

## Update Tally Ceremony Status
Once the trustee keys have been confirmed, use this command to complete the ceremony by passing the `IN_PROGRESS` value to the `--status` flag.
> ⚠️ Before running, re-run the [`step config`](../getting-started/cli_tutorials_getting_started.md#configuration) command to authenticate as **admin user**.
```bash
cargo run step update-tally \
  --election-event-id <ELECTION_EVENT_ID> \
  --tally-id <TALLY_ID> \
  --status <STATUS>
```

* `--election-event-id` – Election event ID **(required)**
* `--tally-id` – Tally ceremony ID (returned from the start step) **(required)**
* `--status` – Tally status **(required)**.