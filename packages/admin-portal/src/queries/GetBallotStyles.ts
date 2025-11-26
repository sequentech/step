// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_BALLOT_STYLES = gql`
    query GetBallotStyles {
        sequent_backend_ballot_style {
            id
            election_id
            election_event_id
            status
            tenant_id
            ballot_eml
            ballot_signature
            created_at
            area_id
            annotations
            labels
            last_updated_at
        }
    }
`
