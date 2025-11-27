// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_TRUSTEE_ARTIFACT_DOWNLOAD_URL = gql`
    mutation GetTrusteeArtifactDownloadUrl($bucket: String!, $key: String!) {
        get_trustee_artifact_download_url(bucket: $bucket, key: $key) {
            url
        }
    }
`
