// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const MONITORING_GET_CONFIG = gql`
    query MonitoringGetConfig(
        $electionEventId: uuid!
        $kind: String!
        $key: String!
        $revision: Int
        $beforeRevision: Int
        $limit: Int
    ) {
        monitoringGetConfig(
            election_event_id: $electionEventId
            kind: $kind
            key: $key
            revision: $revision
            before_revision: $beforeRevision
            limit: $limit
        ) {
            yaml
            revision
            origin
            author {
                id
                name
            }
            created_at
            history {
                revision
                origin
                author {
                    id
                    name
                }
                created_at
            }
        }
    }
`
