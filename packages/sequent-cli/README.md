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
> This is a mandatory first command to setup the credetials in order to use the CLI 

Run ```cargo run -- config --auth-token <AUTH_TOKEN> --tenant-id <TENANT_ID> --endpoint-url <ENDPOINT_URL>```

- auth_token is the keycloak auth token - required*
- endpoint_url is http://graphql-engine:8080/v1/graphql when in local dev codespace environment - required*
- You can grab tenant_id from the local storage in Admin Portal - required*

## Create Auth Token
Run ```cargo run -- generate-auth```


## Create Election Event
Run ```cargo run -- create-election-event --name <ELECTION_EVENT_NAME> --description <DESCRIPTION> --encryption-protocol "RSA256" --is-archived false```

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