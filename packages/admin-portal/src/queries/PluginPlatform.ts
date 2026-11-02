// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"
export const TENANT_PLUGINS = gql`
    query TenantPlugins {
        tenant_plugins {
            data
        }
    }
`
export const SET_PLUGIN_STATE = gql`
    mutation SetTenantPluginState($plugin_id: String!, $state: String!, $expected_version: Int!) {
        set_tenant_plugin_state(
            plugin_id: $plugin_id
            state: $state
            expected_version: $expected_version
        ) {
            data
        }
    }
`
export const EXECUTE_PLUGIN = gql`
    mutation ExecutePluginOperation($plugin_id: String!, $operation: String!, $data: jsonb!) {
        execute_plugin_operation(plugin_id: $plugin_id, operation: $operation, data: $data) {
            data
        }
    }
`
