---
id: cli_reference
title: CLI Reference
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# CLI – Command Reference

---

## Refresh Auth Token

> Call this command when the JWT has expired.

```bash
step refresh-token
```

---

## Create Election Event

```bash
cargo run step create-election-event \
  --name <ELECTION_NAME> \
  --description <DESCRIPTION> \
  --encryption-protocol RSA256 \
  --is-archived
```

* `--name` – Election event name **(required)**
* `--description` – Election event description *(optional)*
* `--encryption-protocol` – Encryption protocol (e.g. `RSA256`) *(optional)*
* `--is-archived` – Archive the election event *(optional flag)*

---

## Create Election

```bash
cargo run step create-election \
  --name <ELECTION_NAME> \
  --description <DESCRIPTION> \
  --election-event-id <ELECTION_EVENT_ID>
```

* `--name` – Election name **(required)**
* `--description` – Election description *(optional)*
* `--election-event-id` – Associated election event ID **(required)**

---

## Create Contest

```bash
cargo run step create-contest \
  --name <CONTEST_NAME> \
  --description <DESCRIPTION> \
  --election-event-id <ELECTION_EVENT_ID> \
  --election-id <ELECTION_ID>
```

* `--name` – Contest name **(required)**
* `--description` – Contest description *(optional)*
* `--election-event-id` – Associated election event ID **(required)**
* `--election-id` – Associated election ID **(required)**

---

## Create Candidate

```bash
cargo run step create-candidate \
  --name <CANDIDATE_NAME> \
  --description <DESCRIPTION> \
  --election-event-id <ELECTION_EVENT_ID> \
  --contest-id <CONTEST_ID>
```

* `--name` – Candidate name **(required)**
* `--description` – Candidate description *(optional)*
* `--election-event-id` – Associated election event ID **(required)**
* `--contest-id` – Associated contest ID **(required)**

---

## Create Area

```bash
cargo run step create-area \
  --name <AREA_NAME> \
  --description <DESCRIPTION> \
  --election-event-id <ELECTION_EVENT_ID>
```

* `--name` – Area name **(required)**
* `--description` – Area description *(optional)*
* `--election-event-id` – Associated election event ID **(required)**

---

## Create Area Contest

```bash
cargo run step create-area-contest \
  --election-event-id <ELECTION_EVENT_ID> \
  --contest-id <CONTEST_ID> \
  --area-id <AREA_ID>
```

* `--election-event-id` – Associated election event ID **(required)**
* `--contest-id` – Contest ID **(required)**
* `--area-id` – Area ID **(required)**

---

## Update Election Event Voting Status

```bash
cargo run step update-event-voting-status \
  --election-event-id <ELECTION_EVENT_ID> \
  --voting-status <VOTING_STATUS> \
  --voting-channel <VOTING_CHANNEL>
```

* `--election-event-id` – Election event ID **(required)**
* `--voting-status` – Voting status **(required)**. One of: `OPEN`, `CLOSE`, `PAUSE`
* `--voting-channel` – Voting channel *(optional)*. One of: `ONLINE`, `KIOSK`, `EARLY_VOTING`

---

## Update Election Voting Status

```bash
cargo run step update-election-voting-status \
  --election-event-id <ELECTION_EVENT_ID> \
  --election-id <ELECTION_ID> \
  --voting-status <VOTING_STATUS> \
  --voting-channel <VOTING_CHANNEL>
```

* `--election-event-id` – Election event ID **(required)**
* `--election-id` – Election ID **(required)**
* `--voting-status` – Voting status **(required)**. One of: `OPEN`, `CLOSE`, `PAUSE`
* `--voting-channel` – Voting channel *(optional)*. One of: `ONLINE`, `KIOSK`, `EARLY_VOTING`

---

## Import Election Event

```bash
cargo run step import-election \
  --file-path <PATH> \
  --is-local
```

* `--file-path` – Path to JSON file **(required)**
* `--is-local` – Use local environment *(optional flag)*

---

## Create Voter

```bash
cargo run step create-voter \
  --election-event-id <ELECTION_EVENT_ID> \
  --first-name <FIRST_NAME> \
  --last-name <LAST_NAME> \
  --username <USERNAME> \
  --email <EMAIL>
```

* `--election-event-id` – Election event ID **(required)**
* `--email` – Voter email **(required)**
* `--first-name` – First name *(optional)*
* `--last-name` – Last name *(optional)*
* `--username` – Username *(optional)*

---

## Update Voter

```bash
cargo run step update-voter \
  --election-event-id <ELECTION_EVENT_ID> \
  --user-id <USER_ID> \
  --first-name <FIRST_NAME> \
  --last-name <LAST_NAME> \
  --username <USERNAME> \
  --email <EMAIL> \
  --password <PASSWORD> \
  --area-id <AREA_ID>
```

* `--election-event-id` – Election event ID **(required)**
* `--user-id` – User ID **(required)**
* `--email` – Email *(optional)*
* `--first-name` – First name *(optional)*
* `--last-name` – Last name *(optional)*
* `--username` – Username *(optional)*
* `--password` – Password *(optional)*
* `--area-id` – Area ID *(optional)*

---

## Publish Ballot

```bash
cargo run step publish \
  --election-event-id <ELECTION_EVENT_ID> \
  --election-id <ELECTION_ID>
```

* `--election-event-id` – Election event ID **(required)**
* `--election-id` – Election ID *(optional)*

---

## Start Key Ceremony

```bash
docker compose up -d --no-deps beat trustee1 trustee2
```

```bash
cargo run step start-key-ceremony \
  --election-event-id <ELECTION_EVENT_ID> \
  --threshold <THRESHOLD> \
  --election-id <ELECTION_ID> \
  --name <NAME>
```

* `--election-event-id` – Election event ID **(required)**
* `--threshold` – Minimum trustees required *(optional, default: 2)*
* `--election-id` – Election ID *(optional)*
* `--name` – Ceremony name *(optional)*

---

## Complete Key Ceremony

```bash
cargo run step complete-key-ceremony \
  --election-event-id <ELECTION_EVENT_ID> \
  --key-ceremony-id <KEY_CEREMONY_ID>
```

* `--election-event-id` – Election event ID **(required)**
* `--key-ceremony-id` – Key ceremony ID **(required)**

---

## Start Tally Ceremony

```bash
cargo run step start-tally \
  --election-event-id <ELECTION_EVENT_ID> \
  --election-ids <ELECTION_ID> \
  --election-ids <ELECTION_ID> \
  --tally-type <TALLY_TYPE>
```

* `--election-event-id` – Election event ID **(required)**
* `--election-ids` – Election IDs *(repeatable, optional)*
* `--tally-type` – Tally type **(required)**. One of: `ELECTORAL_RESULTS`, `INITIALIZATION_REPORT`

---

## Confirm Trustee Key for Tally Ceremony

```bash
cargo run step confirm-key-tally \
  --election-event-id <ELECTION_EVENT_ID> \
  --tally-id <TALLY_ID>
```

* `--election-event-id` – Election event ID **(required)**
* `--tally-id` – Tally ceremony ID **(required)**

---

## Update Tally Ceremony Status

```bash
cargo run step update-tally \
  --election-event-id <ELECTION_EVENT_ID> \
  --tally-id <TALLY_ID> \
  --status <IN_PROGRESS>
```

* `--election-event-id` – Election event ID **(required)**
* `--tally-id` – Tally ceremony ID **(required)**
* `--status` – Tally status **(required)**

---

## Render Template

```bash
cargo run step render-template \
  --base-template <BASE_TEMPLATE_PATH> \
  --template <TEMPLATE_PATH> \
  --vars <VARS_JSON_PATH> \
  --output <OUTPUT_HTML_PATH>
```

* `--base-template` – Base handlebars template *(optional)*
* `--template` – Template file **(required)**
* `--vars` – Variables JSON file **(required)**
* `--output` – Output HTML file **(required)**

---

## Generate Voters

```bash
cargo run step generate-voters \
  --working-directory <PATH> \
  --num-users <NUMBER>
```

* `--working-directory` – Working directory path **(required)**
* `--num-users` – Number of voters to generate **(required)**

---

## Duplicate Votes

```bash
cargo run step duplicate-votes \
  --working-directory <PATH> \
  --num-votes <NUMBER>
```

* `--working-directory` – Working directory path **(required)**
* `--num-votes` – Number of votes to duplicate **(required)**

---

## Create Applications

```bash
cargo run step create-applications \
  --working-directory <PATH> \
  --num-applications <NUMBER> \
  --status <STATUS> \
  --type <TYPE>
```

* `--working-directory` – Working directory path **(required)**
* `--num-applications` – Number of applications **(required)**
* `--status` – Application status *(optional)*. One of: `PENDING`, `ACCEPTED`, `REJECTED`
* `--type` – Application type *(optional)*. One of: `AUTOMATIC`, `MANUAL`

---

## Create Electoral Logs

```bash
cargo run step create-electoral-logs \
  --working-directory <PATH> \
  --num-logs <NUMBER>
```

* `--working-directory` – Working directory path **(required)**
* `--num-logs` – Number of logs **(required)**

---

## Hash Passwords CSV

```bash
cargo run step hash-passwords \
  --input-file <INPUT_CSV_PATH> \
  --output-file <OUTPUT_CSV_PATH> \
  --iterations <NUMBER>
```

* `--input-file` – Input CSV file **(required)**
* `--output-file` – Output CSV file **(required)**
* `--iterations` – Hashing iterations *(default: 600000)*

---

## Export Cast Votes CSV

```bash
cargo run step export-cast-votes \
  --server-url <IMMUDB_URL> \
  --username <USERNAME> \
  --password <PASSWORD> \
  --board-db <BOARD_DB_NAME>
```

* `--server-url` – ImmuDB server URL **(required)**
* `--username` – ImmuDB username **(required)**
* `--password` – ImmuDB password **(required)**
* `--board-db` – Bulletin board database name **(required)**
