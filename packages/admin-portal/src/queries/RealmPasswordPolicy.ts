// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export interface RealmPasswordPolicy {
    configured: boolean
    minimum_length: number
    maximum_length: number
    include_uppercase: boolean
    include_lowercase: boolean
    include_digits: boolean
    include_special_characters: boolean
}

export interface GetRealmPasswordPolicyQuery {
    get_realm_password_policy: RealmPasswordPolicy
}

export interface UpdateRealmPasswordPolicyMutation {
    update_realm_password_policy: {
        updated: boolean
    }
}

export const GET_REALM_PASSWORD_POLICY = gql`
    query GetRealmPasswordPolicy($election_event_id: String!) {
        get_realm_password_policy(election_event_id: $election_event_id) {
            configured
            minimum_length
            maximum_length
            include_uppercase
            include_lowercase
            include_digits
            include_special_characters
        }
    }
`

export const UPDATE_REALM_PASSWORD_POLICY = gql`
    mutation UpdateRealmPasswordPolicy(
        $election_event_id: String!
        $minimum_length: Int!
        $maximum_length: Int!
        $include_uppercase: Boolean!
        $include_lowercase: Boolean!
        $include_digits: Boolean!
        $include_special_characters: Boolean!
    ) {
        update_realm_password_policy(
            election_event_id: $election_event_id
            minimum_length: $minimum_length
            maximum_length: $maximum_length
            include_uppercase: $include_uppercase
            include_lowercase: $include_lowercase
            include_digits: $include_digits
            include_special_characters: $include_special_characters
        ) {
            updated
        }
    }
`
