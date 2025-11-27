// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_TRUSTEE_ARTIFACT_UPLOAD_URL = gql`
    mutation GetTrusteeArtifactUploadUrl(
        $electionEventId: String!
        $artifactKind: String!
        $fileName: String!
        $mediaType: String!
        $size: Int!
    ) {
        get_trustee_artifact_upload_url(
            election_event_id: $electionEventId
            artifact_kind: $artifactKind
            file_name: $fileName
            media_type: $mediaType
            size: $size
        ) {
            url
            bucket
            key
        }
    }
`
