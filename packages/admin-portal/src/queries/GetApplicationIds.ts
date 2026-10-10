// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

// One page of the ids of an event's applications, for finding one by the
// support reference the voter was given (derived from the id, one way).
export const GET_APPLICATION_IDS = gql`
    query GetApplicationIds($where: sequent_backend_applications_bool_exp!, $offset: Int!) {
        sequent_backend_applications(
            where: $where
            order_by: {id: asc}
            limit: 5000
            offset: $offset
        ) {
            id
        }
    }
`

export const APPLICATION_IDS_PAGE = 5000

export interface IGetApplicationIdsResult {
    sequent_backend_applications: Array<{id: string}>
}
