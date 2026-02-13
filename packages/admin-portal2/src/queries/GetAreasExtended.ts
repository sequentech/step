// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const GET_AREAS_EXTENDED = gql`
    query sequent_backend_area_extended($electionEventId: uuid!, $areaId: uuid!) {
        sequent_backend_area_contest(
            where: {_and: {election_event_id: {_eq: $electionEventId}, area_id: {_eq: $areaId}}}
        ) {
            contest {
                id
                name
            }
        }
    }
`
