// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const MONITORING_LIST_CONFIG = gql`
    query MonitoringListConfig($electionEventId: uuid!) {
        monitoringListConfig(election_event_id: $electionEventId) {
            documents {
                kind
                key
                revision
                origin
                author {
                    id
                    name
                }
                created_at
                sha256
            }
        }
    }
`
