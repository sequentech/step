// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const IMPORT_TEMPLATES = gql`
    mutation ImportTemplates($tenantId: String!, $documentId: String!, $sha256: String) {
        import_templates(tenant_id: $tenantId, document_id: $documentId, sha256: $sha256) {
            error_msg
            document_id
        }
    }
`
