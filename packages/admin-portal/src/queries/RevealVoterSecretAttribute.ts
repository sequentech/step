// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const REVEAL_VOTER_SECRET_ATTRIBUTE = gql`
    query RevealVoterSecretAttribute(
        $tenantId: String!
        $electionEventId: String!
        $userId: String!
        $attributeName: String!
    ) {
        reveal_voter_secret_attribute(
            tenant_id: $tenantId
            election_event_id: $electionEventId
            user_id: $userId
            attribute_name: $attributeName
        ) {
            attribute_name
            values
        }
    }
`
