// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const IMPORT_ELECTION_EVENT = gql`
    mutation ImportElectionEvents($tenantId: String!, $documentId: String!) {
        import_election_event(
            tenant_id: $tenantId
            document_id: $documentId
        ) {
            id
        }
    }
`
