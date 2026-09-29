// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const MONITORING_SET_MODE = gql`
    mutation MonitoringSetMode($electionEventId: String!, $mode: String!) {
        monitoringSetMode(election_event_id: $electionEventId, mode: $mode) {
            mode
            generation
        }
    }
`
