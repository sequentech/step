// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {EVotingPortalAuditButtonCfg} from "@sequentech/ui-core"
import type {IReceivedCast} from "../../hooks/useCastReceivedBallot"

export interface SessionBallotData {
    ballotId: string
    // The voter's signed request to cast the ballot, when the ballot box
    // received it at review.
    receivedCast?: IReceivedCast
    electionId: string
    isDemo: boolean
    ballot: string
    timestamp?: number
    auditButtonCfg?: EVotingPortalAuditButtonCfg
}

export const BALLOT_DATA_KEY = "ballotData"
export const BALLOT_DATA_EXPIRATION_KEY = "ballotDataExpiration"
export const clearSessionStorageBallotData = () => {
    sessionStorage.removeItem(BALLOT_DATA_KEY)
    sessionStorage.removeItem(BALLOT_DATA_EXPIRATION_KEY)
}
