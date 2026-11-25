// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"
import type {TallyVerificationBoard} from "@/services/TallyPublicVerification"

export interface GetTallyPublicVerificationBoardsQuery {
    get_tally_public_verification_boards: {boards: TallyVerificationBoard[]}
}

export const GET_TALLY_PUBLIC_VERIFICATION_BOARDS = gql`
    query GetTallyPublicVerificationBoards($electionEventId: String!, $tallySessionId: String!) {
        get_tally_public_verification_boards(
            election_event_id: $electionEventId
            tally_session_id: $tallySessionId
        ) {
            boards {
                dkg_board
                tally_board
                batch
                election_presentation
                contest_presentation
                area_name
                configuration_message
                ballots_message
            }
        }
    }
`
