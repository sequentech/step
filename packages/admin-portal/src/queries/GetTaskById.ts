// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_TASK_BY_ID = gql`
    query GetTaskById($task_id: uuid) {
        sequent_backend_tasks_execution(where: {id: {_eq: $task_id}}) {
            id
            election_event_id
            tenant_id
            execution_status
            type
            start_at
            end_at
            logs
            annotations
            executed_by_user
        }
    }
`
