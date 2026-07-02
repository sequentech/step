---
id: cli_tutorials_running_a_keys_ceremony
title: Running a Keys Ceremony with the CLI
position: 3
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This guide walks you through configuring your environment, starting a Keys Ceremony, and completing it as trustees.

---

## Prerequisites

1. Start the Required Containers
```bash
cd .devcontainer
docker compose up -d --no-deps beat trustee1 trustee2
```

2. Navigate to the CLI:
```bash
cd packages/step-cli  # or ../packages/step-cli if already in .devcontainer
```

3. Ensure a Valid Election Event Exists  
>Make sure a valid election event is already set up in the system.
* You can [create or import](./02-cli-tutorials-creating-an-election-event.md) an election event using the CLI if necessary.
* The election event ID will be required when starting the Keys Ceremony.

---

## Step 1: Start the Keys Ceremony (Admin Step)
### 1. Configure the CLI as an Admin

First, configure the CLI to point to the correct tenant and Keycloak instance, and authenticate as an ***admin user***:
```bash
cd packages/step-cli
cli step config --tenant-id 90505c8a-23a9-4cdf-a26b-4e19f6a097d5 \
--endpoint-url http://graphql-engine:8080/v1/graphql \
--keycloak-url http://keycloak:8090 \
--keycloak-user admin \
--keycloak-password admin \
--keycloak-client-id api-key-client \
--keycloak-client-secret 4lzmxNgZHjfzS5BwDVlyrRUDqwvFLUvL
```
### 2. Start the Keys Ceremony
After successfully authenticating as the admin user, start a Keys Ceremony for the given election event:

```bash
cli step start-key-ceremony \
  --election-event-id ac037831-66bd-451b-bdf7-e0a30eb2bfa0
```
> This command starts a Keys Ceremony for all elections within the specified election event. 

### Output:

If the command succeeds, you will see output similar to:


```bash 
Success! Successfully started key ceremony. ID: d9792af0-71b8-4952-8aac-94bc0fead5f7
```
📌 Important:
Save the Key Ceremony ID (d9792af0-71b8-4952-8aac-94bc0fead5f7 in this example).
You will need it in Step 2.

---

## Step 2: Complete the Key Ceremony (Trustees)

After the ceremony has started, each trustee must complete it ***individually***.

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

### 2. Run the Completion Command
Use the Key Ceremony ID returned in Step 1 to complete this trustee’s part of the ceremony:
```bash
cli step complete-key-ceremony \
  --election-event-id ac037831-66bd-451b-bdf7-e0a30eb2bfa0 \
  --key-ceremony-id d9792af0-71b8-4952-8aac-94bc0fead5f7
```

### Repeat for the Next Trustee
After completing the ceremony as trustee1:
> Re-run Step 2 again, starting with authentication,
> this time logging in as trustee2, in order to fully complete the Keys Ceremony.

-----------

## Create Publication and Enable Voting
After completing the key ceremony, you must publish the election event to make it official and then 
start the election event in order to enable voting.

### 1. Authenticate as Admin
This process requires Gold-level admin permissions. If you haven't configured your CLI session yet, run the following command to authenticate:

```bash
cli step config --tenant-id 90505c8a-23a9-4cdf-a26b-4e19f6a097d5 \
--endpoint-url http://graphql-engine:8080/v1/graphql \
--keycloak-url http://keycloak:8090 \
--keycloak-user admin \
--keycloak-password admin \
--keycloak-client-id api-key-client \
--keycloak-client-secret 4lzmxNgZHjfzS5BwDVlyrRUDqwvFLUvL
```
### 2. Publish the Election Event
Once authenticated, publish the election event and create publication:

```bash
cli step publish \
--election-event-id ac037831-66bd-451b-bdf7-e0a30eb2bfa0
```

### 3. Enable Online Voting
Finally, transition the voting status to OPEN for the online channel. This action allows eligible voters to begin casting their votes

```bash
cli step update-event-voting-status \
  --election-event-id ac037831-66bd-451b-bdf7-e0a30eb2bfa0 \
  --voting-status OPEN \
  --voting-channel ONLINE
```
--------


ℹ️ Reminder:
Values like ac037831-66bd-451b-bdf7-e0a30eb2bfa0 and d9792af0-71b8-4952-8aac-94bc0fead5f7 are examples only.
Your actual IDs will differ depending on your system configuration.