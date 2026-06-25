// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export interface UpdateRealmAttributesMutation {
    update_realm_attributes: {
        updated: boolean
    } | null
}

export const UPDATE_REALM_ATTRIBUTES = gql`
    mutation UpdateRealmAttributes($election_event_id: String!, $attributes: jsonb!) {
        update_realm_attributes(election_event_id: $election_event_id, attributes: $attributes) {
            updated
        }
    }
`
