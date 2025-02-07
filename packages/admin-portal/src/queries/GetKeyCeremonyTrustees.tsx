// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_KEY_CEREMONY_TRUSTEES = gql`
    query get_key_ceremony_trustees($trusteeIds: [uuid!], $tenantId: uuid!) {
        sequent_backend_trustee(
            where: {_and: {tenant_id: {_eq: $tenantId}, id: {_in: $trusteeIds}}}
        ) {
            name
        }
    }
`
