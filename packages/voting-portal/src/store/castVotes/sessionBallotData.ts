// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {EVotingPortalAuditButtonCfg} from "@sequentech/ui-core"
import type {IReceivedCast} from "../../hooks/useCastReceivedBallot"

export interface SessionBallotOwner {
    tenantId: string
    eventId: string
    userId: string
}

export interface SessionBallotLocator {
    owner: SessionBallotOwner
    electionId: string
    ballotId: string
}

export interface SessionBallotData {
    ballotId: string
    owner?: SessionBallotOwner
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
export const BALLOT_LOCATOR_KEY = "ballotLocatorReference"

export const discardForeignSessionBallotLocator = (owner: SessionBallotOwner): void => {
    const raw = sessionStorage.getItem(BALLOT_LOCATOR_KEY)
    if (!raw) return
    try {
        const saved = JSON.parse(raw) as SessionBallotLocator | null
        if (
            saved?.owner?.tenantId === owner.tenantId &&
            saved.owner.eventId === owner.eventId &&
            saved.owner.userId === owner.userId
        )
            return
    } catch {
        /* Malformed references cannot belong to an authenticated owner. */
    }
    sessionStorage.removeItem(BALLOT_LOCATOR_KEY)
}

export const readSessionBallotLocator = (
    owner: SessionBallotOwner,
    electionId: string
): SessionBallotLocator | undefined => {
    try {
        const saved = JSON.parse(
            sessionStorage.getItem(BALLOT_LOCATOR_KEY) ?? "null"
        ) as SessionBallotLocator | null
        if (
            saved?.owner?.tenantId === owner.tenantId &&
            saved.owner.eventId === owner.eventId &&
            saved.owner.userId === owner.userId &&
            !!owner.userId &&
            saved.electionId === electionId &&
            typeof saved.ballotId === "string" &&
            /^(?:[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}|[a-f0-9]{64}|[a-f0-9]{128})$/.test(
                saved.ballotId
            )
        )
            return {owner, electionId: saved.electionId, ballotId: saved.ballotId}
    } catch {
        /* Invalid references grant no ballot access. */
    }
    return undefined
}
export interface SessionBallotSnapshot {
    serialized: string | null
    expiry: string | null
}

export const snapshotSessionBallotData = (): SessionBallotSnapshot => ({
    serialized: sessionStorage.getItem(BALLOT_DATA_KEY),
    expiry: sessionStorage.getItem(BALLOT_DATA_EXPIRATION_KEY),
})

export const isCurrentSessionBallotData = (expected: SessionBallotSnapshot): boolean =>
    sessionStorage.getItem(BALLOT_DATA_KEY) === expected.serialized &&
    sessionStorage.getItem(BALLOT_DATA_EXPIRATION_KEY) === expected.expiry

export const clearSessionStorageBallotData = (
    expected?: SessionBallotSnapshot,
    locator?: SessionBallotLocator
): boolean => {
    if (expected && !isCurrentSessionBallotData(expected)) return false
    // Only expiry retains this non-disclosing reference. Logout, successful
    // casting and explicit abandonment clear both pieces of recovery state.
    sessionStorage.removeItem(BALLOT_DATA_KEY)
    sessionStorage.removeItem(BALLOT_DATA_EXPIRATION_KEY)
    if (locator) sessionStorage.setItem(BALLOT_LOCATOR_KEY, JSON.stringify(locator))
    else sessionStorage.removeItem(BALLOT_LOCATOR_KEY)
    return true
}
