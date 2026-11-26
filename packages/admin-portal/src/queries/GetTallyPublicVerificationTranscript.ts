// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"
import type {BoardMessages} from "@/services/TallyPublicVerification"

export interface GetTallyPublicVerificationTranscriptQuery {
    get_tally_public_verification_transcript: BoardMessages
}

export const GET_TALLY_PUBLIC_VERIFICATION_TRANSCRIPT = gql`
    query GetTallyPublicVerificationTranscript(
        $electionEventId: String!
        $tallySessionId: String!
        $tallyBoard: String!
    ) {
        get_tally_public_verification_transcript(
            election_event_id: $electionEventId
            tally_session_id: $tallySessionId
            tally_board: $tallyBoard
        ) {
            dkg_messages
            tally_messages
        }
    }
`
