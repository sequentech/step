// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const MONITORING_RESET_TO_PRESET = gql`
    mutation MonitoringResetToPreset($electionEventId: uuid!, $presetId: String!) {
        monitoringResetToPreset(election_event_id: $electionEventId, preset_id: $presetId) {
            generation
        }
    }
`
