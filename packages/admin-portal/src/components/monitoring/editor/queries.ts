// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The editor's Hasura actions, written by hand against the monitoring
 * contract until Harvest's actions are in the schema and codegen types them.
 * Nested structures (problems, previews, author) are `jsonb` in the action
 * output types, so the editor reads them through {@link normalizeProblems}
 * rather than trusting a shape.
 */

import {gql} from "@apollo/client"

export const MONITORING_VALIDATE_CONFIG = gql`
    mutation MonitoringValidateConfig(
        $election_event_id: uuid!
        $kind: String!
        $key: String!
        $yaml: String!
    ) {
        monitoringValidateConfig(
            election_event_id: $election_event_id
            kind: $kind
            key: $key
            yaml: $yaml
        ) {
            result
            problems
            preview
        }
    }
`

export const MONITORING_SAVE_CONFIG = gql`
    mutation MonitoringSaveConfig(
        $election_event_id: uuid!
        $kind: String!
        $key: String!
        $yaml: String
        $expected_revision: Int
        $change: String!
    ) {
        monitoringSaveConfig(
            election_event_id: $election_event_id
            kind: $kind
            key: $key
            yaml: $yaml
            expected_revision: $expected_revision
            change: $change
        ) {
            revision
            generation
        }
    }
`

export const MONITORING_RENDER_WIDGET_DRAFT = gql`
    mutation MonitoringRenderWidgetDraft(
        $election_event_id: uuid!
        $election_id: uuid
        $dashboard_id: String!
        $widget_id: String!
        $scope: jsonb!
        $selector_values: jsonb!
        $width: Int!
        $color_scheme: String!
        $locale: String!
        $draft: jsonb
    ) {
        monitoringRenderWidget(
            election_event_id: $election_event_id
            election_id: $election_id
            dashboard_id: $dashboard_id
            widget_id: $widget_id
            scope: $scope
            selector_values: $selector_values
            width: $width
            color_scheme: $color_scheme
            locale: $locale
            draft: $draft
        ) {
            state
            reason
            svg
            table
            notices
            diagnostics
            ignored_selectors
            render_ms
            snapshot_revision
            as_of
        }
    }
`

export const MONITORING_GET_CONFIG = gql`
    query MonitoringGetConfig(
        $election_event_id: uuid!
        $kind: String!
        $key: String!
        $revision: Int
    ) {
        monitoringGetConfig(
            election_event_id: $election_event_id
            kind: $kind
            key: $key
            revision: $revision
        ) {
            yaml
            revision
            origin
            author
            created_at
        }
    }
`

export const MONITORING_LIST_CONFIG = gql`
    query MonitoringListConfig($election_event_id: uuid!) {
        monitoringListConfig(election_event_id: $election_event_id) {
            documents
        }
    }
`

export const MONITORING_LIST_PRESETS = gql`
    query MonitoringListPresets($election_event_id: uuid!) {
        monitoringListPresets(election_event_id: $election_event_id) {
            presets
        }
    }
`

export const MONITORING_RESET_TO_PRESET = gql`
    mutation MonitoringResetToPreset($election_event_id: uuid!, $preset_id: String!) {
        monitoringResetToPreset(election_event_id: $election_event_id, preset_id: $preset_id) {
            generation
        }
    }
`
