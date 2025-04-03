// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_SIGNED_URLS = gql`
    mutation GetSignedUrls($eventId: uuid!) {
        get_signed_urls(election_event_id: $eventId) {
            urls
        }
    }
`
