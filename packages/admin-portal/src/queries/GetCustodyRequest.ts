// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_CUSTODY_REQUEST = gql`
    mutation GetCustodyRequest($electionEventId: String!, $keysCeremonyId: String!) {
        get_custody_request(
            object: {election_event_id: $electionEventId, keys_ceremony_id: $keysCeremonyId}
        ) {
            custody_request_json
        }
    }
`
