// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export interface GetRealmAttributesQuery {
    get_realm_attributes: {
        attributes: Record<string, string>
    }
}

export const GET_REALM_ATTRIBUTES = gql`
    query GetRealmAttributes($election_event_id: String!) {
        get_realm_attributes(election_event_id: $election_event_id) {
            attributes
        }
    }
`
