---
id: cli_tutorials_running_a_tally_ceremony
title: Running a Tally Ceremony with the CLI
position: 4
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This guide walks you through configuring your environment, starting a Tally Ceremony, confirming trustee keys, and completing the tally process.

---

## Prerequisites

1. Start the required containers:
```bash
cd .devcontainer
docker compose up -d --no-deps beat trustee1 trustee2
```

2. Navigate to the CLI:
```bash
cd packages/step-cli  # or ../packages/step-cli if already in .devcontainer
```

3. Ensure a Keys Ceremony Has Been Completed

>  A Keys Ceremony must be completed successfully for the election event before starting a Tally Ceremony. To learn about how to run a keys ceremony click [here](./03-cli-tutorials-running-a-keys-ceremony.md)

4. Close Voting (Optional)
   Depending on your election configuration, you may need to transition the voting status to CLOSE before the Tally Ceremony can begin.
   To close the online voting channel, run:
```bash
cli step update-event-voting-status \
  --election-event-id ac037831-66bd-451b-bdf7-e0a30eb2bfa0 \
  --voting-status CLOSE \
  --voting-channel ONLINE
```

---

## Step 1: Start the Tally Ceremony (Admin Step)
Start a Tally Ceremony for the given election event: 
> ⚠️ You must be authenticated as an admin user before running this command.
If you have not already done so, re-run cli step config with admin credentials.

```bash
cli step start-tally \
  --election-event-id ac037831-66bd-451b-bdf7-e0a30eb2bfa0 \
  --election-ids 6f5c2c7b-8b3f-4c2b-9e36-1b87c4c2e9c1 \
  --election-ids 314723b2-c8a5-4fb7-9d90-e6ffba0b0f6d \
  --tally-type ELECTORAL_RESULTS
```
> This command starts a Tally Ceremony for the specified elections within the election event.

> If you want to start a Tally Ceremony for all elections, dont pass `--election-ids` flag

### Output:

If the command succeeds, you will see output similar to:

```bash 
Success! Successfully started Tally ceremony. ID: bc7fcae1-f9e4-4714-a6c2-4c43a5cf07d9
```
📌 Important:
Save the Tally Ceremony ID (bc7fcae1-f9e4-4714-a6c2-4c43a5cf07d9 in this example).
You will need it in the next steps.

---

## Step 2: Confirm Trustee Keys (Trustees)

After the Tally Ceremony has started, each trustee must confirm their key ***individually***.

### 1. Authenticate as a Trustee
Reconfigure the CLI and log in as the specific trustee (for example, trustee1):
```bash
cli step config --tenant-id 90505c8a-23a9-4cdf-a26b-4e19f6a097d5 \
--endpoint-url http://graphql-engine:8080/v1/graphql \
--keycloak-url http://keycloak:8090 \
--keycloak-user trustee1 \
--keycloak-password trustee1 \
--keycloak-client-id api-key-client \
--keycloak-client-secret 4lzmxNgZHjfzS5BwDVlyrRUDqwvFLUvL
```
### 2. Confirm the Trustee Key
Use the Tally Ceremony ID returned in Step 1 to confirm this trustee’s key:
```bash
cli step confirm-key-tally \
  --election-event-id ac037831-66bd-451b-bdf7-e0a30eb2bfa0 \
  --tally-id bc7fcae1-f9e4-4714-a6c2-4c43a5cf07d9
```

### Repeat for the Next Trustee
After confirming the key as trustee1:
> 🔁 Re-run Step 2 from the beginning,
> authenticate as trustee2, and confirm the key again.

---

## Step 3: Update Tally Ceremony Status (Admin Step)
Once all trustee keys have been confirmed, the admin must update the Tally Ceremony status.


### 1. Re-authenticate as Admin

```bash
cli step config \
  --tenant-id 90505c8a-23a9-4cdf-a26b-4e19f6a097d5 \
  --endpoint-url http://graphql-engine:8080/v1/graphql \
  --keycloak-url http://keycloak:8090 \
  --keycloak-user admin \
  --keycloak-password admin \
  --keycloak-client-id api-key-client \
  --keycloak-client-secret 4lzmxNgZHjfzS5BwDVlyrRUDqwvFLUvL
```

### 2. Update the Tally Ceremony Status
Update the Tally Ceremony status to IN_PROGRESS:
```bash
cli step update-tally \
  --election-event-id ac037831-66bd-451b-bdf7-e0a30eb2bfa0 \
  --tally-id bc7fcae1-f9e4-4714-a6c2-4c43a5cf07d9 \
  --status IN_PROGRESS
```

-------

## Step 4: Download Tally Reults
To download tally results, run the following command:

```bash
cli step download-tally-results \
  --election-event-id ac037831-66bd-451b-bdf7-e0a30eb2bfa0 \
  --tally-id bc7fcae1-f9e4-4714-a6c2-4c43a5cf07d9
```
> This will download the results into `output` folder under step-cli package.

ℹ️ Reminder: 
Values like ac037831-66bd-451b-bdf7-e0a30eb2bfa0 and bc7fcae1-f9e4-4714-a6c2-4c43a5cf07d9 are examples only.
Your actual IDs will differ depending on your system configuration.