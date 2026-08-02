// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const GENERATE_VOTER_INFORMATION_LETTER = gql`
    mutation GenerateVoterInformationLetter($electionEventId: String!, $voterId: String!) {
        generate_voter_information_letter(election_event_id: $electionEventId, voter_id: $voterId) {
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

export const GET_VOTER_INFORMATION_LETTER_PASSWORD = gql`
    query GetVoterInformationLetterPassword($taskId: String!) {
        get_voter_information_letter_password(task_id: $taskId) {
            pdf_password
        }
    }
`
