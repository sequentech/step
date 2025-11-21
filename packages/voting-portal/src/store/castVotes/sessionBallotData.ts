// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export interface SessionBallotData {
    ballotId: string
    electionId: string
    isDemo: boolean
    ballot: string
    timestamp?: number
}

export const BALLOT_DATA_KEY = "ballotData"
export const BALLOT_DATA_EXPIRATION_KEY = "ballotDataExpiration"
export const clearSessionStorageBallotData = () => {
    sessionStorage.removeItem(BALLOT_DATA_KEY)
    sessionStorage.removeItem(BALLOT_DATA_EXPIRATION_KEY)
}
