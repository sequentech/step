// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const MONITORING_EXPORT = gql`
    mutation MonitoringExport(
        $electionEventId: uuid!
        $electionId: uuid
        $dashboardId: String!
        $widgetId: String
        $scope: jsonb!
        $selectorValues: jsonb!
        $widgetSelectorValues: jsonb
        $snapshotRevision: Int!
        $format: String!
        $from: String
        $to: String
    ) {
        monitoringExport(
            election_event_id: $electionEventId
            election_id: $electionId
            dashboard_id: $dashboardId
            widget_id: $widgetId
            scope: $scope
            selector_values: $selectorValues
            widget_selector_values: $widgetSelectorValues
            snapshot_revision: $snapshotRevision
            format: $format
            from: $from
            to: $to
        ) {
            document_id
            task_execution {
                id
                name
                execution_status
            }
        }
    }
`
