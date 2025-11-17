// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const CREATE_TRANSMISSION_PACKAGE = gql`
    mutation CreateTransmissionPackage(
        $electionEventId: uuid!
        $electionId: uuid!
        $tallySessionId: uuid!
        $areaId: uuid!
        $force: Boolean!
    ) {
        create_transmission_package(
            election_event_id: $electionEventId
            election_id: $electionId
            tally_session_id: $tallySessionId
            area_id: $areaId
            force: $force
        ) {
            error_msg
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
