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

### Get help:
For detailed usage instructions:
> To see all available CLI commands:
```bash
cli step --help
```
> To get detailed help for a specific command:
```bash
cli step <command> --help
```

---

## Configuration

Configure CLI with your environment credentials:
This is a mandatory first command to setup the credentials in order to use the CLI 

```bash
cli step step config \
  --tenant-id <TENANT_ID> \
  --endpoint-url <ENDPOINT_URL> \
  --keycloak-url <KEYCLOAK_URL> \
  --keycloak-user <KEYCLOAK_USER> \
  --keycloak-password <KEYCLOAK_PASSWORD> \
  --keycloak-client-id <KEYCLOAK_CLIENT_ID> \
  --keycloak-client-secret <KEYCLOAK_CLIENT_SECRET>
```

* `--tenant-id` - Tenant ID used to connect to the corresponding Keycloak realm **(required)**
* `--endpoint-url` -  GraphQL endpoint URL (HASURA_ENDPOINT) **(required)**
* `--keycloak-url` - Keycloak server URL (KEYCLOAK_URL) **(required)**
* `--keycloak-user` - Username of a user that exists in the realm associated with the tenant **(required)**
* `--keycloak-password` - Password of the specified user  **(required)**
* `--keycloak-client-id` - Keycloak realm client ID **(required)**
* `--keycloak-client-secret` - Keycloak realm client secret **(required)**

---

## Refresh Auth Token

> Call this command when the JWT has expired.

```bash
cli step refresh-token
```

---

## Create Election Event

```bash
cli step create-election-event \
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
cli step create-election \
  --name <ELECTION_NAME> \
  --external-id <EXTERNAL_ID> \
  --description <DESCRIPTION> \
  --election-event-id <ELECTION_EVENT_ID>
```

* `--name` – Election name **(required)**
* `--external-id` Unique Id for the election - required*
* `--description` – Election description *(optional)*
* `--election-event-id` – Associated election event ID **(required)**

---

## Create Contest

```bash
cli step create-contest \
  --name <CONTEST_NAME> \
  --description <DESCRIPTION> \
  --election-event-id <ELECTION_EVENT_ID> \
  --election-id <ELECTION_ID> \
  --counting-algorithm <COUNTING_ALGORITHM>
```

* `--name` – Contest name **(required)**
* `--description` – Contest description *(optional)*
* `--election-event-id` – Associated election event ID **(required)**
* `--election-id` – Associated election ID **(required)**
* `--counting-algorithm` - Counting algorithm *(optional, default: plurality-at-large)*. One of: `plurality-at-large`, `instant-runoff`

---

## Create Candidate

```bash
cli step create-candidate \
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
cli step create-area \
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
cli step create-area-contest \
  --election-event-id <ELECTION_EVENT_ID> \
  --contest-id <CONTEST_ID> \
  --area-id <AREA_ID>
```

* `--election-event-id` – Associated election event ID **(required)**
* `--contest-id` – Contest ID **(required)**
* `--area-id` – Area ID **(required)**

---

## Export Election event

```bash
cli step export-election-event \
  --election-event-id <ELECTION_EVENT_ID> \
  --include-voters \
  --activity-logs \
  --bulletin-board \
  --publications \
  --s3-files \
  --scheduled-events \
  --reports \
  --applications \
  --tally \
  --encrypted \
  --output-dir <OUTPUT_DIRECTORY_PATH>
```
* `--election-event-id` – Associated election event ID **(required)**
* `--include-voters` – Include voter data in the export *(optional)*
* `--activity-logs` – Include election event  activity logs *(optional)*
* `--bulletin-board` –  Include bulletin board contents *(optional)*
* `--publications` – Include publications *(optional)*
* `--s3-files` – Include associated files stored in S3 *(optional)*
* `--scheduled-events` – Include scheduled events *(optional)*
* `--reports` – Include election event reports *(optional)*
* `--applications `– Include applications data *(optional)*
* `--tally` – Include election event tally *(optional)*
* `--encrypted `– Encrypt the exported package *(optional)*
* `--output-dir` – Directory where the export will be saved *(optional, default: ./data)*
  
> Note: The export is forced to be encrypted if you include any sensitive data flags, specifically: `--bulletin-board`, `--reports`, `--applications`, or `--tally`.

---

## Update Election Event Voting Status

```bash
cli step update-event-voting-status \
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
cli step update-election-voting-status \
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
cli step import-election \
  --file-path <PATH> \
  --is-local
```

* `--file-path` – Path to JSON file **(required)**
* `--is-local` – Use local environment *(optional flag)*

---

## Create Voter

```bash
cli step create-voter \
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
cli step update-voter \
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
cli step publish \
  --election-event-id <ELECTION_EVENT_ID> \
  --election-id <ELECTION_ID>
```

* `--election-event-id` – Election event ID **(required)**
* `--election-id` – Election ID *(optional)*

---

## Start Key Ceremony

```bash
cli step start-key-ceremony \
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
cli step complete-key-ceremony \
  --election-event-id <ELECTION_EVENT_ID> \
  --key-ceremony-id <KEY_CEREMONY_ID>
```

* `--election-event-id` – Election event ID **(required)**
* `--key-ceremony-id` – Key ceremony ID **(required)**

---

## Start Tally Ceremony

```bash
cli step start-tally \
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
cli step confirm-key-tally \
  --election-event-id <ELECTION_EVENT_ID> \
  --tally-id <TALLY_ID>
```

* `--election-event-id` – Election event ID **(required)**
* `--tally-id` – Tally ceremony ID **(required)**

---

## Update Tally Ceremony Status

```bash
cli step update-tally \
  --election-event-id <ELECTION_EVENT_ID> \
  --tally-id <TALLY_ID> \
  --status <STATUS>
```

* `--election-event-id` – Election event ID **(required)**
* `--tally-id` – Tally ceremony ID **(required)**
* `--status` – Tally status **(required)**. One of:   `STARTED`, `CONNECTED`,
 `IN_PROGRESS`, `SUCCESS`, `CANCELLED`,

---

## Download Tally Results
```bash
cli step download-tally-results \
  --election-event-id <ELECTION_EVENT_ID> \
  --tally-id <TALLY_ID> \
  --output-dir <OUTPUT_DIR>
```
* `--election-event-id` – Election event ID **(required)**
* `--tally-id` – Tally ceremony ID **(required)**
* `--output-dir` - The directory where results will be saved. *(optional; defaults to the output folder within the step-cli package.)*

---

## Tally Sheets

Create and review single tally-sheet versions:

```bash
cli step tally-sheet create \
  --election-event-id <ELECTION_EVENT_ID> \
  --area-id <AREA_ID> \
  --contest-id <CONTEST_ID> \
  --channel PAPER \
  --content-file <AREA_CONTEST_RESULTS_JSON>

cli step tally-sheet review \
  --election-event-id <ELECTION_EVENT_ID> \
  --tally-sheet-id <TALLY_SHEET_ID> \
  --status APPROVED
```

Preview, create, review, inspect, and download tally-sheet imports:

```bash
cli step tally-sheet import-preview \
  --election-event-id <ELECTION_EVENT_ID> \
  --file-path <SOURCE_XML_OR_CSV> \
  --source-format ESS_ENHANCED_XML \
  --selected-channel PAPER

cli step tally-sheet import-create \
  --election-event-id <ELECTION_EVENT_ID> \
  --file-path <SOURCE_XML_OR_CSV> \
  --source-format ESS_ENHANCED_XML \
  --selected-channel PAPER

cli step tally-sheet import-review \
  --election-event-id <ELECTION_EVENT_ID> \
  --import-id <IMPORT_ID> \
  --decision APPROVE

cli step tally-sheet import-list \
  --election-event-id <ELECTION_EVENT_ID> \
  --limit 50

cli step tally-sheet import-show \
  --election-event-id <ELECTION_EVENT_ID> \
  --import-id <IMPORT_ID>

cli step tally-sheet import-download-source \
  --election-event-id <ELECTION_EVENT_ID> \
  --import-id <IMPORT_ID> \
  --output-dir <OUTPUT_DIR>
```

Use `--document-id <DOCUMENT_ID>` instead of `--file-path` when the source document is already uploaded. Use `--sha256 <HEX_SHA256>` with an existing document to enable source verification. For local development uploads, add `--is-local`.

Convert an ES&S Enhanced XML file to canonical tally-sheet CSV without uploading it:

```bash
cli step tally-sheet convert-ess-xml \
  --input <SOURCE_XML> \
  --output <CANONICAL_CSV> \
  --selected-channel PAPER
```

Recount a completed tally session with fresh result IDs:

```bash
cli step tally-sheet recount \
  --election-event-id <ELECTION_EVENT_ID> \
  --tally-id <TALLY_ID>
```

---

## Render Template

```bash
cli step render-template \
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
This command create csv file with voters within the `working-directory` path. 
> Required additional confituration at `external_config.json` in the `working-directory` (election_event_json_file and generate_voters fields)

```bash
cli step generate-voters \
  --working-directory <PATH> \
  --num-users <NUMBER>
```

* `--working-directory` – Working directory path **(required)**
* `--num-users` – Number of voters to generate **(required)**

---

## Duplicate Votes
This command duplicate ***existing*** cast_vote row.
> Required additional confituration at `external_config.json` in the `working-directory` (realm_name, duplicate_votes  fields)

```bash
cli step duplicate-votes \
  --working-directory <PATH> \
  --num-votes <NUMBER>
```

* `--working-directory` – Working directory path **(required)**
* `--num-votes` – Number of votes to duplicate **(required)**

---

## Create Applications
> Required additional confituration at `external_config.json` in the `working-directory` (realm_name, tenant_id, election_event_id, generate_applications  fields)

```bash
cli step create-applications \
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
Create electoral logs in immudb.
> Required additional confituration at `external_config.json` in the `working-directory` (tenant_id, election_event_id, election_id, area_id and realm_name fields)

```bash
cli step create-electoral-logs \
  --working-directory <PATH> \
  --num-logs <NUMBER>
```

* `--working-directory` – Working directory path **(required)**
* `--num-logs` – Number of logs **(required)**

---

## Hash Passwords CSV
This command takes a voter_list.csv as input where the input has password column and outputs a voter_list.csv with hashed passwords and salts for the passwords to make it faster to import.
```bash
cli step hash-passwords \
  --input-file <INPUT_CSV_PATH> \
  --output-file <OUTPUT_CSV_PATH> \
  --iterations <NUMBER>
```

* `--input-file` – Input CSV file **(required)**
* `--output-file` – Output CSV file **(required)**
* `--iterations` – Hashing iterations *(default: 600000)*

---

## Export Cast Votes CSV
This command accesses immudb bulletin board and exports in a csv file the casted ballots ballot_id.
```bash
cli step export-cast-votes \
  --server-url <IMMUDB_URL> \
  --username <USERNAME> \
  --password <PASSWORD> \
  --board-db <BOARD_DB_NAME>
```

* `--server-url` – ImmuDB server URL (`IMMUDB_SERVER_URL`) **(required)**
* `--username` – ImmuDB username (`IMMUDB_USER`) **(required)**
* `--password` – ImmuDB password (`IMMUDB_PASSWORD`) **(required)**
* `--board-db` – Bulletin board database name **(required)**
