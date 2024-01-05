// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const UPDATE_COMMUNICATION_TEMPLATE = gql`
    mutation UpdateCommunicationTemplate(
        $id: uuid!
        $tenantId: uuid!
        $set: sequent_backend_communication_template_set_input!
    ) {
        update_sequent_backend_communication_template_by_pk(
            pk_columns: {id: $id, tenant_id: $tenantId}
            _set: $set
        ) {
            id
        }
    }
`
