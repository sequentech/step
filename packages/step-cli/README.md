<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# step-cli

Command-line tool for automating and streamlining operations within the Sequent Voting Platform ecosystem.

## Features

- Election event and contest creation
- Voter management and import
- Key ceremony and tally management
- Load testing and data generation utilities
- Template rendering for email notifications
- ImmuDB bulletin board export

## Quick Start

### Installation

Build from source:

```bash
cd packages/step-cli
cargo build --release
```

Or use the pre-compiled binary from GitHub releases.

### Configuration

Configure CLI with your environment credentials:
This is a mandatory first command to setup the credentials in order to use the CLI 

```bash
cli step config \
  --tenant-id <TENANT_ID> \
  --endpoint-url <ENDPOINT_URL> \
  --keycloak-url <KEYCLOAK_URL> \
  --keycloak-user <KEYCLOAK_USER> \
  --keycloak-password <KEYCLOAK_PASSWORD> \
  --keycloak-client-id <KEYCLOAK_CLIENT_ID> \
  --keycloak-client-secret <KEYCLOAK_CLIENT_SECRET>
```

For local development:
- Tenant id - You can use 90505c8a-23a9-4cdf-a26b-4e19f6a097d5 when in local dev - required*
- Endpoint url - endpoint_url is http://graphql-engine:8080/v1/graphql when in local dev codespace environment - required*
- Keycloak url - this is http://keycloak:8090 when in local dev codespace environment - required*
- Keycloak user - use admin for local - required*
- Keycloak password - use admin for local - required*
- Keycloak client id - required*
- Keycloak client secret - required*
### Usage

Get help:

```bash
cli step --help
cli step <command> --help
```

Common commands:

## Refresh Auth Token
> This command should be called when the jwt has expired
```bash 
cli step refresh-token
```

# Create Election Event
```bash 
cli step create-election-event --name <ELECTION_NAME> --description <DESCRIPTION> --encryption-protocol "RSA256" --is-archived
```
- name - the election event name - required*
- description - the election event description - optional*
- encryption_protocol - optional*
- is_archived - boolean if should be archived - optional*

## Create Election
```bash 
cli step create-election --name <ELECTION_NAME> --external-id <EXTERNAL_ID> --description <DESCRIPTION> --election-event-id <ELECTION_EVENT_ID>
```
- name - the election name - required*
- external-id Unique Id for the election - required*
- description - the election description - optional*
- election_event_id - The associated election event id - required*

## Create Contest
```bash 
cli step create-contest --name <CONTEST_NAME> --description <DESCRIPTION> --election-event-id <ELECTION_EVENT_ID> --election-id <ELECTION_ID> --counting-algorithm <COUNTING_ALGORITHM>
```
- name - the contest name - required*
- description - the contest description - optional*
- election_event_id - The associated election event id - required*
- election_id - The associated election id - required*
- counting_algorithm - A valid counting algorithm(plurality-at-large, instant-runoff) - optional* (default: plurality-at-large )

## Create Candidate
```bash 
cli step create-candidate --name <CANDIDATE_NAME> --description <DESCRIPTION> 
--election-event-id <ELECTION_EVENT_ID>
```
- name - the candidate name - required*
- description - the candidate description - optional*
- election_event_id - The associated election event id - required*
- contest_id - The associated contest id - required*

## Create Area
```bash 
cli step create-area --name <AREA_NAME> --description <DESCRIPTION> 
--election-event-id <ELECTION_EVENT_ID>
```
- name - the area name - required*
- description - the area description - optional*
- election_event_id - The associated election event id - required*

## Create Area Contest
```bash 
cli step create-area-contest --election-event-id <ELECTION_EVENT_ID> --contest-id <CONTEST_ID> --area-id <AREA_ID>
```
- election_event_id - The associated election event id - required*
- contest_id - The associated contest id - required*
- area_id - The associated area id - required*

## Export Election Event
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
- Election event id – The associated election event id - required*
- Include voters – Include voter data in the export - optional*
- Activity logs – Include election event activity logs - optional*
- Bulletin board – Include bulletin board contents - optional*
- Publications – Include publications- optional*
- S3 files – Include associated files stored in S3 - optional*
- Scheduled events – Include scheduled events - optional*
- Reports – Include election event reports - optional*
- Applications – Include applications data - optional*
- Tally – Include election event tallyInclude election event tally - optional*
- Encrypted – Encrypt the exported package - optional*
- Output-dir – The directory where the export will be saved - optional* (defaults to the data folder within the step-cli package.)

> Note: The export is forced to be encrypted if you include any sensitive data flags, specifically: `--bulletin-board`, `--reports`, `--applications`, or `--tally`.

## Update Election Event Voting Status
```bash 
cli step update-event-voting-status --election-event-id <ELECTION_EVENT_ID> --voting-status <VOTING_STATUS>
```
- election-event-id - The associated election event id - required*
- voting-status - A valid voting status (OPEN, CLOSE, PAUSE) - required*

## Update Election Voting Status
```bash 
cli step update-election-voting-status --election-event-id <ELECTION_EVENT_ID> --election-id <ELECTION_ID> --voting-status <VOTING_STATUS>
```
- election-event-id - The associated election event id - required*
- election-id - The associated election id - required*
- voting-status - A valid voting status (OPEN, CLOSE, PAUSE) - required*

## Import election event from .json file
```bash 
cli step import-election --file-path <PATH> --is-local <ADD THIS FOR LOCAL ONLY>
```
- file-path - Path to file - required* (Example - /workspaces/step/packages/step-cli/data/mock.json)
- is-local - If run locally add this flag

## Create Voter
> This can be used to create a new voter for an election event
```bash 
cli step create-voter --election-event-id <ELECTION_EVENT_ID> --first-name <FIRST_NAME> --last-name <LAST_NAME> --username <USERNAME> --email <EMAIL>
```
- Election event id - the election event to be associated with - required*
- Email - voter email - required*
- First name - voter name
- Last name - voter name
- username - voter username

## Update Voter
> This can be used to update voter details, set a password and area for a voter
```bash 
cli step update-voter --election-event-id <ELECTION_EVENT_ID> --user-id <USER_ID> --first-name <FIRST_NAME> --last-name <LAST_NAME> --username <USERNAME> --email <EMAIL>  --password <PASSWORD> --area-id <AREA_ID>
```
- Election event id - the election event to be associated with - required*
- User Id - user identifier - required*
- Email - voter email - required*
- First name - voter name
- Last name - voter name
- username - voter username
- Password - user password
- Area Id - area to be associated to user

## Publish Ballot
> This generates a new publication and publishes it
```bash 
cli step publish --election-event-id <ELECTION_EVENT_ID> --election-id <ELECTION_ID>```
```
- Election event id - the election event to be associated with - required*
- Election id - the election to be associated with. if set publish for spesific election- optional*

## Start Key Ceremony
> This only starts a key ceremony - make sure to first run in .devcontainer:
```bash 
docker compose up -d --no-deps beat trustee1 trustee2
cli step start-key-ceremony --election-event-id <ELECTION_EVENT_ID> --threshold <THRESHOLD> --election-id <ELECTION_ID> --name <NAME>
```
- Election event id - the election event to be associated with - required*
- Threshold - the minimum number of trustees required to tally - optional* (default: 2)
- Election id - optional specific election to start the key ceremony for - optional*
- Name - optional name or alias of the election - optional*

## Complete Key Ceremony
> This needs to be done by a trustee - authenticate with a trustee using the config command
```bash 
cli step complete-key-ceremony --election-event-id <ELECTION_EVENT_ID> --key-ceremony-id <KEY_CEREMONY_ID>
```
- Election event id - the election event to be associated with - required*
- Key ceremony id - the key ceremony to complete - required*

## Start Tally Ceremony
```bash 
cli step start-tally --election-event-id <ELECTION_EVENT_ID> --election-ids <ELECTION_IDS> --tally-type <TALLY_TYPE>
```
- Election event id - the election event to be associated with - required*
- Election ids - optional specific elections to start the tally for - optional*
- Tally type - the type of tally to perform (ELECTORAL_RESULTS or INITIALIZATION_REPORT) - required*

## Confirm Trustee Key For Tally Ceremony
> This needs to be done by a trustee - authenticate with a trustee using the config command
```bash 
cli step confirm-key-tally --election-event-id <ELECTION_EVENT_ID> --tally-id <TALLY_ID>
```
- Election event id - the election event to be associated with - required*
- Tally id - the tally ceremony id to confirm the key for - required*

## Update Tally Ceremony Status
> This can be used to complete the tally ceremony after the trustee keys have been confirmed
```bash 
cli step update-tally --election-event-id <ELECTION_EVENT_ID> --tally-id <TALLY_ID> --status <STATUS>
```
- Election event id - the election event to be associated with - required*
- Tally id - the tally ceremony id to confirm the key for - required*
- Status - the status of the tally - enter <b>IN_PROGRESS</b> for completing the tally ceremony

## Download Tally Results
```bash
cli step download-tally-results \
  --election-event-id <ELECTION_EVENT_ID> \
  --tally-id <TALLY_ID> \
  --output-dir <OUTPUT_DIR>
```
- Election event id - the election event to be associated with - required*
- Tally id - the tally ceremony id to confirm the key for - required*
- Output-dir - the directory where results will be saved. optional* (defaults to the output folder within the step-cli package.)

## Tally Sheets
Create and review single tally sheets:

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

Preview, create, and review tally sheet imports:

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

Convert an ES&S Enhanced XML file to the canonical tally sheet CSV format for inspection or upload:

```bash
cli step tally-sheet convert-ess-xml \
  --input <SOURCE_XML> \
  --output <CANONICAL_CSV> \
  --selected-channel PAPER
```

Recount a completed tally session:

```bash
cli step tally-sheet recount \
  --election-event-id <ELECTION_EVENT_ID> \
  --tally-id <TALLY_ID>
```

## Render Template
> This can be used to renders a handlerbars file into html
```bash 
cli step render-template [--base-template <PATH_TO_TEMPLATE_FILE>] --template <PATH_TO_TEMPLATE_FILE> --vars <PATH_TO_VARIABLES_FILE> --output  <PATH_TO_OUTPUT_FILE>
```
- template = path to the handlebars-rs template file can be example.hbs
- vars - path to variables file needs to be a json file containing the vars needed for the handlebars-rs file
- output - where should the html file be written to


## Generate voters
> This can be used to create csv file with voters. 
> This action require to have export_election_event-<id>.json file in working-directory.
```bash 
cli step generate-voters --working-directory <PATH_FOR_INPUT_OUTPUT> --num-users <NUMBER_VOTERS_TO_GENERATE>
```
- working-directory = path to the config.json files and output directoty (workspaces/step/packages/step-cli/data)
- num-users - how much voters to generate


## Duplicate votes
> This can be used to duplicate existing cast_vote row.
> This required additional confituration at config.json in working-directory
```bash 
cli step duplicate-votes --working-directory <PATH_FOR_INPUT_OUTPUT> --num-votes <NUMBER_VOTES_TO_DUPLICATE>
```
- working-directory = path to the config.json files and output directoty (workspaces/step/packages/step-cli/data)
- num-votes - how much votes to duplicate

## Create Applications
> This can be used to create applicaiton.
> This required additional confituration at config.json in working-directory
```bash 
cli step create-applications --working-directory <PATH_FOR_INPUT_OUTPUT> --num-applications <NUMBER_APPLICATIONS_TO_CREATE> --status <STATUS> --type <TYPE>
```
- working-directory = path to the config.json files and output directoty (workspaces/step/packages/step-cli/data)
- num-applications - how much applications to create
- status: OPTIONAL, should be PENDING, ACCEPTED or REJECTED
- type:  OPTIONAL, should be AUTOMATIC or MANUAL

## Create Electoral Logs
> This can be used to create electoral logs in immudb.
> This required additional confituration at config.json in working-directory (like area_id and election_id)
```bash 
cli step create-electoral-logs --working-directory <PATH_FOR_INPUT_OUTPUT> --num-logs <NUMBER_LOGS_TO_CREATE>
```
- working-directory = path to the config.json files and output directoty (workspaces/step/packages/step-cli/data)
- num-logs - how much logs to create

## Hash password csv
> This takes a voter_list.csv as input where the input has password column and outputs a voter_list.csv with hashed passwords and salts for the passwords to make it faster to import. 
```bash 
cli step hash-passwords --input-file <PATH_FOR_INPUT_OUTPUT> --output-file <PATH_TO_OUTPUT_FILE> --iterations <NUMBER_OF_HASHING_ITERATIONS>
```
## Export cast votes csv
> This accesses immudb bulletin board and exports in a csv file the casted ballots ballot_id.
```bash 
cli step export-cast-votes --server-url http://immudb:3322 --username immudb --password immudb --board-db tenant90505c8a23a94cdfaevent3a9fcf6515c4478db32105e02b509899
```
- iterations = number of iterations for the hashing where the default if 600000

## Compiling and Using the CLI 

> The CLI will be compilied using github action into a binray on every push/pull request or realease 
> The CLI will be compiled to both MAC use or L
>  In order to run on Linux: 
> go Open a terminal, navigate to the folder where the binary is located, and run:
  ```bash 
  chmod +x seq
  ```
> Execute a commend by running by running: 
  ```bash 
  ./seq <COMMAND> [OPTIONS]
  ```
   for Example: 
   ./seq step hash-passwords --input-file <PATH_FOR_INPUT_OUTPUT> --output-file <PATH_TO_OUTPUT_FILE> --iterations <NUMBER_OF_HASHING_ITERATIONS>

```
## Documentation

For comprehensive CLI reference and tutorials, see:
- [CLI Documentation](https://docs.sequentech.io/docusaurus/main/docs/developers/CLI/cli_cli)
- [CLI Reference](https://docs.sequentech.io/docusaurus/main/docs/developers/CLI/Reference/cli_reference)
- [Creating an Election Tutorial](https://docs.sequentech.io/docusaurus/main/docs/developers/CLI/Tutorials/Creating-an-Election-with-the-CLI/cli_tutorials_creating-an-election-with-the-cli)
