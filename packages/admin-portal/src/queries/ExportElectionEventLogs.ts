// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const EXPORT_ELECTION_EVENT_LOGS = gql`
    mutation ExportElectionEventLogs($electionEventId: String) {
        export_election_event_logs(election_event_id: $electionEventId) {
            document_id
            task_id
        }
    }
`
