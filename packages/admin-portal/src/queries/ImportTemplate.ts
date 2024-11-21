// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const IMPORT_TEMPLATES = gql`
    mutation ImportTemplates($tenantId: String!, $documentId: String!) {
        import_templates(tenant_id: $tenantId, document_id: $documentId) {
            error_msg
            document_id
        }
    }
`
