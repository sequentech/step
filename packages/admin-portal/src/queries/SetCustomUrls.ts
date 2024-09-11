// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const SET_CUSTOM_URLS = gql`
    mutation SetCustomUrls(
        $origin: String!
        $redirect_to: String!
        $dns_prefix: String!
        $election_id: String!
        $key: String!
    ) {
        set_custom_urls(
            origin: $origin
            redirect_to: $redirect_to
            dns_prefix: $dns_prefix
            election_id: $election_id
            key: $key
        ) {
            success
            message
        }
    }
`
