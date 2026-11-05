// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const MONITORING_VALIDATE_CONFIG = gql`
    query MonitoringValidateConfig(
        $electionEventId: uuid!
        $kind: String!
        $key: String!
        $yaml: String!
    ) {
        monitoringValidateConfig(
            election_event_id: $electionEventId
            kind: $kind
            key: $key
            yaml: $yaml
        ) {
            result
            problems {
                severity
                code
                path
                message
            }
            preview {
                state
                reason
                svg
                table
                notices
                diagnostics {
                    severity
                    code
                    path
                    message
                }
                ignored_selectors
                render_ms
                snapshot_revision
                as_of
            }
        }
    }
`
