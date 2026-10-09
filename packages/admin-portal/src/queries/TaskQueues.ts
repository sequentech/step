// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const TASK_QUEUES_OVERVIEW = gql`
    query TaskQueuesOverview {
        task_queues_overview
    }
`

export const TASK_QUEUES_THROUGHPUT = gql`
    query TaskQueuesThroughput($queue: String!, $hours: Int, $bucketMinutes: Int) {
        task_queues_throughput(queue: $queue, hours: $hours, bucket_minutes: $bucketMinutes)
    }
`

export const TASK_QUEUES_MESSAGES = gql`
    query TaskQueuesMessages($queue: String!, $state: String!, $before: String, $limit: Int) {
        task_queues_messages(queue: $queue, state: $state, before: $before, limit: $limit)
    }
`

export const TASK_QUEUES_DEAD_LETTERS = gql`
    mutation TaskQueuesDeadLetters($operation: String!, $messageIds: [String!]!) {
        task_queues_dead_letters(operation: $operation, message_ids: $messageIds)
    }
`

export const TASK_QUEUES_QUERY = gql`
    query TaskQueuesQuery($sql: String!) {
        task_queues_query(sql: $sql)
    }
`
