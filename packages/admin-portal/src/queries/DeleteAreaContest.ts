// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const DELETE_AREA_CONTESTS = gql`
    mutation delete_area_contests($tenantId: uuid!, $area: uuid!) {
        delete_sequent_backend_area_contest(
            where: {_and: {area_id: {_eq: $area}, tenant_id: {_eq: $tenantId}}}
        ) {
            returning {
                id
            }
        }
    }
`
