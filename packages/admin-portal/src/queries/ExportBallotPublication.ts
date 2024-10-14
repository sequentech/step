// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const EXPORT_BALLOT_PUBLICATION = gql`
    mutation ExportBallotPublication(
        $tenantId: String!
        $electionEventId: String!
        $electionId: String
        $ballotPublicationId: String!
    ) {
        export_ballot_publication(
            tenant_id: $tenantId
            election_event_id: $electionEventId
            election_id: $electionId
            ballot_publication_id: $ballotPublicationId
        ) {
            document_id
            task_execution {
                id
                name
                execution_status
                created_at
                start_at
                end_at
                logs
                annotations
                labels
                executed_by_user
                tenant_id
                election_event_id
                type
            }
        }
    }
`
