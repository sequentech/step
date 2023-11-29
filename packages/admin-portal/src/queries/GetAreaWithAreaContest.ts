// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const GET_AREA_WITH_AREA_CONTESTS = gql`
    query get_area_with_area_contests($electionEventId: uuid!) {
        sequent_backend_area_contest(where: {election_event_id: {_eq: $electionEventId}}) {
            contest {
                name
            }
            id
        }
    }
`
