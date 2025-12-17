---
id: cli_tutorials_running_a_tally_ceremony
title: Running a Tally Ceremony Using Step CLI
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
This guide walks you through configuring your environment and executing a **Tally Ceremony**.

---

## Prerequisites

* Ensure your environment is configured with the correct credentials.
  Use the `step config` command to point to your specific tenant and Keycloak instance.
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
> Before running it, re-run the `step config` command to authenticate as the specific trustee.

```bash
cargo run step confirm-key-tally \
  --election-event-id <ELECTION_EVENT_ID> \
  --tally-id <TALLY_ID>
```

* `--election-event-id` – Election event ID used when starting the tally **(required)**
* `--tally-id` – Tally ceremony ID returned from the start step **(required)**
