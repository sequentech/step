// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const MONITORING_LIST_CONFIG = gql`
    query MonitoringListConfig($electionEventId: String!) {
        monitoringListConfig(election_event_id: $electionEventId) {
            mode
            generation
            preset {
                id
                version
            }
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
