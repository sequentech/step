// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const MONITORING_LIST_DASHBOARDS = gql`
    query MonitoringListDashboards($electionEventId: uuid!, $electionId: uuid) {
        monitoringListDashboards(election_event_id: $electionEventId, election_id: $electionId) {
            mode
            preset {
                id
                title
            }
            dashboards {
                id
                title
                requirements
                widget_count
            }
            snapshot {
                revision
                as_of
                checked_at
            }
            refresh_seconds
        }
    }
`
