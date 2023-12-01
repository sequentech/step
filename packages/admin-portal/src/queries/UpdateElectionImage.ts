// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const UPDATE_ELECTION_IMAGE = gql`
    mutation update_election_image($id: uuid, $image: String) {
        update_sequent_backend_election(
            where: {id: {_eq: $id}}
            _set: {image_document_id: $image}
        ) {
            returning {
                id
            }
        }
    }
`
