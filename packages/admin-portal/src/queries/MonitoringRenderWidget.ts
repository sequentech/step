// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const MONITORING_RENDER_WIDGET = gql`
    query MonitoringRenderWidget(
        $electionEventId: uuid!
        $electionId: uuid
        $dashboardId: String!
        $widgetId: String!
        $scope: jsonb!
        $selectorValues: jsonb!
        $snapshotRevision: Int
        $width: Int!
        $colorScheme: String!
        $locale: String!
        $draft: jsonb
    ) {
        monitoringRenderWidget(
            election_event_id: $electionEventId
            election_id: $electionId
            dashboard_id: $dashboardId
            widget_id: $widgetId
            scope: $scope
            selector_values: $selectorValues
            snapshot_revision: $snapshotRevision
            width: $width
            color_scheme: $colorScheme
            locale: $locale
            draft: $draft
        ) {
            state
            reason
            svg
            table
            tables {
                query
                table
            }
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
`
