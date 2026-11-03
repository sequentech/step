// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const BALLOT_PRIVACY_SETTINGS = gql`
    query BallotPrivacySettings($electionEventId: uuid!) {
        ballot_privacy_settings(election_event_id: $electionEventId) {
            payload
        }
    }
`

export interface IBallotPrivacySettings {
    system_public_key: string | null
    roles: {forensic_read: string[]; pseudonym_resolve: string[]}
}
