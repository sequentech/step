// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const INSERT_AREA_CONTESTS = gql`
    mutation insert_area_contests($areas: [sequent_backend_area_contest_insert_input!]!) {
        insert_sequent_backend_area_contest(objects: $areas) {
            returning {
                id
            }
        }
    }
`
