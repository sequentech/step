<!-- // SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only -->

# Configure
Run cargo run -- config --auth_token "your_auth_token" --tenant_id "your_tenant_id" --endpoint_url "your_endpoint_url"

- auth_token is the keycloak auth token
- endpoint_url is "http://harvest:8400" when in local dev codespace environment
- You can grab tenant_id from the local storage in Admin Portal

# Create Election Event
Run cargo run -- create-election-event --name "Election_Event_Name" --description "Description" --presentation "{}" --encryption_protocol "RSA256" --is_archived false

- name - the election event name - required*
- description - the election event desciption - optional*
- presentation - Presentation object - optional*
- encryption_protocol - optional*
- is_archived - boolean if should be archived - optional*