<!-- // SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only -->

# Sequent CLI
A tool created to automate and streamline actions within the sequent ecosystem

## Help
Run 
``` cargo run -- help``` to see list of commands
For any given command you can run -h to see list of arguments you can pass


## Configure
> This is a mandatory first command to setup the credentials in order to use the CLI 

Run ```cargo run -- config --tenant-id <TENANT_ID> --endpoint-url <ENDPOINT_URL> --keycloak-url <KEYCLOAK_URL> --keycloak-user <KEYCLOAK_USER> --keycloak-password <KEYCLOAK_PASSWORD> --keycloak-client-id <KEYCLOAK_CLIENT_ID> --keycloak-client-secret <KEYCLOAK_CLIENT_SECRET>```

- Tenant id - You can use 90505c8a-23a9-4cdf-a26b-4e19f6a097d5 when in local dev - required*
- Endpoint url - endpoint_url is http://graphql-engine:8080/v1/graphql when in local dev codespace environment - required*
- Keycloak url - this is http://keycloak:8090 when in local dev codespace environment - required*
- Keycloak user - use admin for local - required*
- Keycloak password - use admin for local - required*
- Keycloak client id - use admin for local - required*
- Keycloak client secret - use admin for local - required*

## Refresh Auth Token
> This command should be called when the jwt has expired
Run ```cargo run -- refresh-token```

## Create Election Event
Run ```cargo run -- create-election-event --name <ELECTION_EVENT_NAME> --description <DESCRIPTION> --encryption-protocol "RSA256" --is-archived```

- name - the election event name - required*
- description - the election event desciption - optional*
- encryption_protocol - optional*
- is_archived - boolean if should be archived - optional*


## Create Election
Run ```cargo run -- create-election --name <ELECTION_NAME> --description <DESCRIPTION> --election-event-id <ELECTION_EVENT_ID>```

- name - the election name - required*
- description - the election desciption - optional*
- election_event_id - The associated election event id - required*

## Create Contest
Run ```cargo run -- create-contest --name <CONTEST_NAME> --description <DESCRIPTION> --election-event-id <ELECTION_EVENT_ID>```

- name - the contest name - required*
- description - the contest desciption - optional*
- election_event_id - The associated election event id - required*
- election_id - The associated election id - required*

## Create Candidate
Run ```cargo run -- create-candidate --name <CANDIDATE_NAME> --description <DESCRIPTION> --election-event-id <ELECTION_EVENT_ID>```

- name - the candidate name - required*
- description - the candidate desciption - optional*
- election_event_id - The associated election event id - required*
- contest_id - The associated contest id - required*

## Create Area
Run ```cargo run -- create-area --name <AREA_NAME> --description <DESCRIPTION> --election-event-id <ELECTION_EVENT_ID>```

- name - the area name - required*
- description - the area desciption - optional*
- election_event_id - The associated election event id - required*

## Create Area
Run ```cargo run -- create-area-contest --election-event-id <ELECTION_EVENT_ID> --contest-id <CONTEST_ID> --area-id <AREA_ID>```

- election_event_id - The associated election event id - required*
- contest_id - The associated contest id - required*
- area_id - The associated area id - required*

## Update Election Event Voting Status
Run ```cargo run -- update-election-event-status --election-event-id <ELECTION_EVENT_ID> --status <STATUS>```

- election_event_id - The associated election event id - required*
- status - A valid voting status (OPEN, CLOSED,...)- required*

## Update Event Voting Status
Run ```cargo run -- update-election-status --election-event-id <ELECTION_EVENT_ID> --election-id <ELECTION_ID> --status <STATUS>```

- election_event_id - The associated election event id - required*
- election_id - The associated election id - required*
- status - A valid voting status (OPEN, CLOSED,...)- required*

## Import election event from .json file
Run ```cargo run -- import-election --file-path <PATH> --is-local <ADD THIS FOR LOCAL ONLY>```

- file-path - Path to file - required* (Example - /workspaces/step/packages/sequent-cli/data/mock.json)
- is-local - If run locally add this flag

## Create Voter
> This can be used to create a new voter for an election event

Run ```cargo run -- create-voter --election-event-id <ELECTION_EVENT_ID> --first-name <FIRST_NAME> --last-name <LAST_NAME> --username <USERNAME> --email <EMAIL>```

- Election event id - the election event to be associated with - required*
- Email - voter email - required*
- First name - voter name
- Last name - voter name
- username - voter username

## Update Voter

> This can be used to update voter details, set a password and area for a voter

Run ```cargo run -- update-voter --election-event-id <ELECTION_EVENT_ID> --user-id <USER_ID> --first-name <FIRST_NAME> --last-name <LAST_NAME> --username <USERNAME> --email <EMAIL>  --password <PASSWORD> --area-id <AREA_ID>```

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

Run ```cargo run -- publish --election-event-id <ELECTION_EVENT_ID>```

- Election event id - the election event to be associated with - required*

## Start Key Ceremony

> This <b>only</b> starts a key ceremony

Run ```cargo run -- start-key-ceremony --election-event-id <ELECTION_EVENT_ID>```

- Election event id - the election event to be associated with - required*

## Complete Key Ceremony

> This needs to be done by a trustee - authenticate with a trustee using the config command

Run ```cargo run -- complete-key-ceremony --election-event-id <ELECTION_EVENT_ID> --key-ceremony-id <KEY_CEREMONY_ID>```

- Election event id - the election event to be associated with - required*
- Key ceremony id - the key ceremony to complete - required*