// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const LIST_CAST_VOTE_MESSAGES = gql`
    query listCastVoteMessages(
        $tenant_id: String!
        $election_event_id: String!
        $election_id: String
        $ballot_id: String!
        $limit: Int
        $offset: Int
        $order_by: ElectoralLogOrderBy
    ) {
        listCastVoteMessages(
            tenant_id: $tenant_id
            election_event_id: $election_event_id
            election_id: $election_id
            ballot_id: $ballot_id
            limit: $limit
            offset: $offset
            order_by: $order_by
        ) {
            list {
                statement_timestamp
                statement_kind
                ballot_id
                username
            }
            total
        }
    }
`
