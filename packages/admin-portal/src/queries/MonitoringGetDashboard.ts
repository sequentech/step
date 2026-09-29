// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const MONITORING_GET_DASHBOARD = gql`
    query MonitoringGetDashboard(
        $electionEventId: uuid!
        $electionId: uuid
        $dashboardId: String!
    ) {
        monitoringGetDashboard(
            election_event_id: $electionEventId
            election_id: $electionId
            dashboard_id: $dashboardId
        ) {
            dashboard
            dashboard_revision
            widgets
            theme {
                id
                revision
            }
            settings
            settings_revision
            scope_options {
                regions {
                    key
                    label
                }
                posts {
                    key
                    label
                    region
                }
                countries {
                    key
                    label
                }
            }
            restricted
            pinned_post
            sources
            snapshot {
                revision
                as_of
                checked_at
            }
        }
    }
`
