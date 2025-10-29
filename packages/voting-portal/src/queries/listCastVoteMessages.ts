// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const LIST_CAST_VOTE_MESSAGES = gql`
    query listCastVoteMessages(
        $tenantId: String!
        $electionEventId: String!
        $electionId: String
        $ballotId: String!
        $limit: Int
        $offset: Int
        $orderBy: ElectoralLogOrderBy
    ) {
        list_cast_vote_messages(
            tenant_id: $tenantId
            election_event_id: $electionEventId
            election_id: $electionId
            ballot_id: $ballotId
            limit: $limit
            offset: $offset
            order_by: $orderBy
        ) {
            list {
                statement_timestamp
                statement_kind
                ballot_id
                username
                message
            }
            total
        }
    }
`
