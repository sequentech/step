---
id: cli-creating-an-election-event
title: Creating an Election Event with the CLI
position: 2
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->



This guide walks you through manually creating an election event using the CLI.
> You can also [import](#import-election-event) an election event from JSON configuration file.

----

## Prerequisites

1. Navigate to the CLI:
```bash
cd packages/step-cli
```

2. Configure the CLI to point to the correct tenant and Keycloak instance, and authenticate as an ***admin user***:
```bash
cli step config --tenant-id 90505c8a-23a9-4cdf-a26b-4e19f6a097d5 \
--endpoint-url http://graphql-engine:8080/v1/graphql \
--keycloak-url http://keycloak:8090 \
--keycloak-user admin \
--keycloak-password admin \
--keycloak-client-id api-key-client \
--keycloak-client-secret 4lzmxNgZHjfzS5BwDVlyrRUDqwvFLUvL
```

---

## Step 1: Create an Election Event

Create the election event that will contain all elections and related entities.

```bash
cli step create-election-event \
  --name "National Election 2025" \
  --description "National general election" \
  --encryption-protocol RSA256
```
### Output
If successful, you will see output similar to:

```bash
Success! Election event created successfully! ID: 79aba105-f250-4375-a211-43dcaeadba64
```

📌 Save the Election Event ID (79aba105-f250-4375-a211-43dcaeadba64 in this example).
You will use it in all subsequent steps.

---

## Step 2: Create an Election

Create an election within the previously created election event.
```bash
cli step create-election \
  --name "Presidential Election" \
  --external-id "presidential-election"
  --description "Presidential race" \
  --election-event-id 79aba105-f250-4375-a211-43dcaeadba64
```
### Output
```bash
Success! Election created successfully! ID: 79aba105-f250-4375-a211-43dcaeadba64
```

📌 Save the Election ID (33eae221-9376-4356-a2d7-f62997ec4eaa).

---

## Step 3: Create a Contest

Create a contest within the election.

> ℹ️ **Note**: You can optionally specify a `--counting-algorithm` parameter (defaults to `plurality-at-large`). Use `cli step create-contest --help` to see available options.

```bash
cli step create-contest \
  --name "President" \
  --description "President of the Republic" \
  --election-event-id 79aba105-f250-4375-a211-43dcaeadba64 \
  --election-id 33eae221-9376-4356-a2d7-f62997ec4eaa
```
### Output
```bash
Success! Contest created successfully! ID: 79aba105-f250-4375-a211-43dcaeadba64
```

📌 Save the Contest ID (17d7244b-fe8b-4c3e-897c-6602285cce8c).

---

## Step 4: Create Candidates

Create one or more candidates for the contest.
```bash
cli step create-candidate \
  --name "Alice Johnson" \
  --description "Candidate A" \
  --election-event-id 79aba105-f250-4375-a211-43dcaeadba64 \
  --contest-id 17d7244b-fe8b-4c3e-897c-6602285cce8c
```
### Output
```bash
Success! Candidate created successfully! ID: 79aba105-f250-4375-a211-43dcaeadba64
```

🔁 Repeat this step to add additional candidates to the same contest.

---

## Step 5: Create an Area

Create a geographical or logical area for the election.
```bash
cli step create-area \
  --name "National District" \
  --election-event-id 79aba105-f250-4375-a211-43dcaeadba64
```
### Output
```bash
Success! Area created successfully! ID: 79aba105-f250-4375-a211-43dcaeadba64
```

📌 Save the Area ID (9ebb6361-df2a-4779-a095-0da5888d9fe3).

---

## Step 6: Assign Contest to Area

Link the contest to the area using the IDs you received in previous steps.
```bash
cli step create-area-contest \
  --election-event-id 79aba105-f250-4375-a211-43dcaeadba64 \
  --contest-id 17d7244b-fe8b-4c3e-897c-6602285cce8c \
  --area-id 9ebb6361-df2a-4779-a095-0da5888d9fe3
```
### Output
```bash
Success! Area contest created successfully! ID: 79aba105-f250-4375-a211-43dcaeadba64
```
---

After completing all steps, you will have a basic election event configured with default settings.

---

## Export Election event
To back up your election event configuration, use the following command:

```bash
cli step export-election-event \
--election-event-id 3f53ab49-79b2-4703-8b26-8a25d49d48b9 \
--include-voters \
--bulletin-board \
--encrypted
```
> This will export the election event configuration, including voters and the bulletin board, to the `data` folder inside the `step-cli` package.

---

## Import Election event
```bash
cli step import-election \
  --file-path ./data/export_election_event-bc0da9ad-acae-4574-a4bf-4c549b79c34e.json \
  --is-local
```

This command creates an election event in the system based on the configuration file provided via `--file-path`.
All IDs from the imported file will be replaced with newly generated IDs.
> ℹ️ In this example, the data folder is located in the `packages/step-cli` directory.

### Output

If successful, you will see output similar to:

```bash
Success! Election event created successfully! ID: 79aba105-f250-4375-a211-43dcaeadba64
```

📌 Save the Election Event ID (79aba105-f250-4375-a211-43dcaeadba64 in this example).
You will need it to execute subsequent commands.