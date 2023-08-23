// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const CREATE_REPORT = gql`
    mutation CreateReport($template: String!, $tenantId: String!, $format: String!) {
        renderTemplate(template: $template, tenant_id: $tenantId, format: $format) {
            url
        }
    }
`
