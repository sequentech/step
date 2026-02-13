// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const CHANGE_APPLICATION_STATUS = gql`
    mutation ChangeApplicationStatus(
        $election_event_id: String!
        $user_id: String!
        $id: String!
        $tenant_id: String
        $area_id: String
        $rejection_reason: String
        $rejection_message: String
    ) {
        ApplicationChangeStatus(
            body: {
                election_event_id: $election_event_id
                id: $id
                user_id: $user_id
                tenant_id: $tenant_id
                area_id: $area_id
                rejection_reason: $rejection_reason
                rejection_message: $rejection_message
            }
        ) {
            message
            error
        }
    }
`
