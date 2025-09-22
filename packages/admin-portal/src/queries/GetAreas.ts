// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const GET_AREAS = gql`
    query sequent_backend_area($electionEventId: uuid!) {
        sequent_backend_area(where: {election_event_id: {_eq: $electionEventId}}) {
            id
            name
        }
    }
`
