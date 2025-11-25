// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const UPDATE_TEMPLATE = gql`
    mutation UpdateTemplate(
        $id: uuid!
        $tenantId: uuid!
        $set: sequent_backend_template_set_input!
    ) {
        update_sequent_backend_template_by_pk(
            pk_columns: {id: $id, tenant_id: $tenantId}
            _set: $set
        ) {
            id
        }
    }
`
