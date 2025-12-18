---
id: cli_tutorials_running_a_keys_ceremony
title: Running a Keys Ceremony Using Step CLI
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
This guide walks you through configuring your environment and executing a **Keys Ceremony**.

---

## Prerequisites

* Ensure your environment is configured with the correct credentials.
  Use the [`step config`](../getting-started/cli_tutorials_getting_started.md#configuration) command to point to your specific tenant and Keycloak instance.
  Make sure you authenticate with an **admin user**.

* Ensure the trustees containers are running:

```bash
cd .devcontainer
docker compose up -d --no-deps beat trustee1 trustee2
```

---

## Start the Keys Ceremony

Run the following command to start a keys ceremony:

```bash
cargo run step start-key-ceremony \
  --election-event-id <ELECTION_EVENT_ID> \
  --threshold <THRESHOLD> \
  --election-id <ELECTION_ID> \
  --name <NAME>
```

* `--election-event-id` – Unique ID of the election event **(required)**
* `--threshold` – Minimum number of trustees required to complete the ceremony *(optional, default: 2)*
* `--election-id` – Start the ceremony for a specific election *(optional)*
* `--name` – Alias or name for the ceremony *(optional)*

Once successful, the command outputs a **Key Ceremony ID**.
Save this ID for use in the next step.

---

## Complete the Key Ceremony (Trustees)

After the ceremony has started, it must be completed **once by each trustee**.

> ⚠️ This command must be executed separately by **every trustee**.
> Before running it, re-run the [`step config`](../getting-started/cli_tutorials_getting_started.md#configuration) command to authenticate as the specific trustee.

```bash
cargo run step complete-key-ceremony \
  --election-event-id <ELECTION_EVENT_ID> \
  --key-ceremony-id <KEY_CEREMONY_ID>
```

* `--election-event-id` – Election event ID used when starting the ceremony **(required)**
* `--key-ceremony-id` – Key ceremony ID returned from the start step **(required)**
