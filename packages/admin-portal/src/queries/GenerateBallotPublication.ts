// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GENERATE_BALLOT_PUBLICATION = gql`
    mutation GenerateBallotPublication($electionEventId: uuid!, $electionId: uuid) {
        generate_ballot_publication(election_event_id: $electionEventId, election_id: $electionId) {
            ballot_publication_id
        }
    }
`
