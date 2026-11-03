// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const FORENSIC_NETWORK_SUMMARY = gql`
    query ForensicNetworkSummary(
        $electionEventId: uuid!
        $electionId: uuid
        $ip: String
        $country: String
        $limit: Int
        $offset: Int
        $operation: String
    ) {
        forensic_network_summary(
            election_event_id: $electionEventId
            election_id: $electionId
            ip: $ip
            country: $country
            limit: $limit
            offset: $offset
            operation: $operation
        ) {
            payload
        }
    }
`

export const FORENSIC_NETWORK_DETAIL = gql`
    query ForensicNetworkDetail(
        $electionEventId: uuid!
        $electionId: uuid
        $ip: String!
        $cursor: String
        $limit: Int
    ) {
        forensic_network_detail(
            election_event_id: $electionEventId
            election_id: $electionId
            ip: $ip
            cursor: $cursor
            limit: $limit
        ) {
            payload
        }
    }
`
