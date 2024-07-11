<!-- // SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only -->


# Help
Run cargo run -- help to see list of commands
For any given command you can run -h to see list of arguments you can pass


# Configure
Run cargo run -- config --auth-token "your_auth_token" --tenant-id "your_tenant_id" --endpoint-url "your_endpoint_url"

- auth_token is the keycloak auth token
- endpoint_url is "http://graphql-engine:8080/v1/graphql" when in local dev codespace environment
- You can grab tenant_id from the local storage in Admin Portal

# Create Auth Token
Run cargo run -- generate-auth


# Create Election Event
Run cargo run -- create-election-event --name "Election_Event_Name" --description "Description" --encryption-protocol "RSA256" --is-archived false

- name - the election event name - required*
- description - the election event desciption - optional*
- encryption_protocol - optional*
- is_archived - boolean if should be archived - optional*


# Create Election
Run cargo run -- create-election --name "Election_Name" --description "Description" --election-event-id "election event id"

- name - the election event name - required*
- description - the election event desciption - optional*
- election_event_id - The associated election event id - required*

# Create Contest
Run cargo run -- create-contest --name "Election_Name" --description "Description" --election-event-id "election event id"

- name - the election event name - required*
- description - the election event desciption - optional*
- election_event_id - The associated election event id - required*
- election_id - The associated election id - required*

# Create Candidate
Run cargo run -- create-candidate --name "Election_Name" --description "Description" --election-event-id "election event id"

- name - the election event name - required*
- description - the election event desciption - optional*
- election_event_id - The associated election event id - required*
- contest_id - The associated contest id - required*