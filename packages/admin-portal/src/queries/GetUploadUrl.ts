// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_UPLOAD_URL = gql`
    mutation GetUploadUrl($name: String!, $media_type: String!, $size: Int!) {
        get_upload_url(name: $name, media_type: $media_type, size: $size) {
            url
            document_id
        }
    }
`
