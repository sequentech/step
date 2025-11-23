// // SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const CALL_PLUGIN_ROUTE = gql`
    mutation CallPluginRoute($path: String!, $data: jsonb!) {
        call_plugin_route(path: $path, data: $data) {
            data
        }
    }
`
