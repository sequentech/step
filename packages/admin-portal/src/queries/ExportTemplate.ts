// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const EXPORT_TEMPLATE = gql`
    mutation ExportTemplate($tenantId: String!) {
        export_template(tenant_id: $tenantId) {
            error_msg
            document_id
        }
    }
`
