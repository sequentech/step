// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const EXPORT_ELECTION_EVENT_TASKS = gql`
    mutation ExportElectionEventTasks($electionEventId: String) {
        export_election_event_tasks(election_event_id: $electionEventId) {
            document_id
            task_id
        }
    }
`
