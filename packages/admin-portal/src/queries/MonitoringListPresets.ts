// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const MONITORING_LIST_PRESETS = gql`
    query MonitoringListPresets($electionEventId: String!) {
        monitoringListPresets(election_event_id: $electionEventId) {
            presets {
                id
                version
                title
                description
            }
        }
    }
`
