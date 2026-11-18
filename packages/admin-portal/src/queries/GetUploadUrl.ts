// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_UPLOAD_URL = gql`
    mutation GetUploadUrl(
        $name: String!
        $media_type: String!
        $size: Int!
        $is_public: Boolean! = true
        $election_event_id: String
        $voter_secret_attributes: Boolean
    ) {
        get_upload_url(
            name: $name
            media_type: $media_type
            size: $size
            is_public: $is_public
            election_event_id: $election_event_id
            voter_secret_attributes: $voter_secret_attributes
        ) {
            url
            document_id
        }
    }
`
