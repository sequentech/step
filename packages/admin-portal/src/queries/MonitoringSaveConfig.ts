// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const MONITORING_SAVE_CONFIG = gql`
    mutation MonitoringSaveConfig(
        $electionEventId: String!
        $kind: String!
        $key: String!
        $yaml: String
        $expectedRevision: Int
        $change: String!
    ) {
        monitoringSaveConfig(
            election_event_id: $electionEventId
            kind: $kind
            key: $key
            yaml: $yaml
            expected_revision: $expectedRevision
            change: $change
        ) {
            revision
            generation
            warnings {
                severity
                code
                path
                message
                engine_code
            }
        }
    }
`
