// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const PREPARE_BALLOT_PUBLICATION_PREVIEW = gql`
    mutation PrepareBallotPublicationPreview(
        $electionEventId: String!
        $ballotPublicationId: String!
    ) {
        prepare_ballot_publication_preview(
            election_event_id: $electionEventId
            ballot_publication_id: $ballotPublicationId
        ) {
            error_msg
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
