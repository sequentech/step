// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The editor's Hasura actions, as Harvest declares them (actions.graphql):
 * ids are `uuid`, reads are queries and writes mutations, and each output is
 * a typed object whose fields are selected here; only definitions and
 * tables are `jsonb`.
 *
 * The operations are named `MonitoringEditor*`: the view's documents for
 * the same actions (`@/queries/Monitoring*`) select less, and codegen needs
 * every operation name to be unique.
 */

import {gql} from "@apollo/client"

export const MONITORING_EDITOR_VALIDATE_CONFIG = gql`
    query MonitoringEditorValidateConfig(
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
            problems {
                severity
                code
                path
                message
                engine_code
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
                    engine_code
                }
                ignored_selectors
                render_ms
                snapshot_revision
                as_of
            }
        }
    }
`

export const MONITORING_EDITOR_SAVE_CONFIG = gql`
    mutation MonitoringEditorSaveConfig(
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

export const MONITORING_EDITOR_RENDER_WIDGET = gql`
    query MonitoringEditorRenderWidget(
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
            diagnostics {
                severity
                code
                path
                message
                engine_code
            }
            ignored_selectors
            render_ms
            snapshot_revision
            as_of
        }
    }
`

export const MONITORING_EDITOR_GET_CONFIG = gql`
    query MonitoringEditorGetConfig(
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
            author {
                id
                name
            }
            created_at
        }
    }
`

export const MONITORING_EDITOR_LIST_CONFIG = gql`
    query MonitoringEditorListConfig($election_event_id: uuid!) {
        monitoringListConfig(election_event_id: $election_event_id) {
            documents {
                kind
                key
                revision
                origin
                author {
                    id
                    name
                }
                created_at
            }
        }
    }
`

export const MONITORING_EDITOR_LIST_PRESETS = gql`
    query MonitoringEditorListPresets($election_event_id: uuid!) {
        monitoringListPresets(election_event_id: $election_event_id) {
            presets {
                id
                version
                title
                description
            }
        }
    }
`

export const MONITORING_EDITOR_RESET_TO_PRESET = gql`
    mutation MonitoringEditorResetToPreset($election_event_id: uuid!, $preset_id: String!) {
        monitoringResetToPreset(election_event_id: $election_event_id, preset_id: $preset_id) {
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
