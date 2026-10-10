// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export enum ELocateBallotStatus {
    FOUND = "found",
    NOT_FOUND = "not-found",
    AMBIGUOUS = "ambiguous",
    CHECKS_ENDED = "checks-ended",
}

export const LOCATE_BALLOT = gql`
    query LocateBallot($electionEventId: uuid!, $electionId: uuid!, $ballotId: String!) {
        locate_ballot(
            election_event_id: $electionEventId
            election_id: $electionId
            ballot_id: $ballotId
        ) {
            status
            ballot_id
            content
            cast_at
            checks_available_until
        }
    }
`
