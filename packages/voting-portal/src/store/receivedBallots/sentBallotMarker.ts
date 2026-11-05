// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {SessionBallotOwner} from "../castVotes/sessionBallotData"

// The one-time voter key that casts a ballot lives only in memory. After the
// ballot box has received the ballot, a reload of the review page drops the key
// and the ballot with it. This marker is all that outlives the reload: that a
// ballot was sent from this tab, and its Tracking ID. It holds no key and no
// choices, and it ends with the tab.
const SENT_BALLOT_KEY = "sentBallotMarker"

export interface ISentBallotMarker {
    owner: SessionBallotOwner
    electionId: string
    trackingId: string
}

const sameOwner = (a?: SessionBallotOwner, b?: SessionBallotOwner): boolean =>
    !!a && !!b && a.tenantId === b.tenantId && a.eventId === b.eventId && a.userId === b.userId

export const markBallotSent = (marker: ISentBallotMarker): void => {
    try {
        sessionStorage.setItem(SENT_BALLOT_KEY, JSON.stringify(marker))
    } catch {
        /* Without the marker a reload shows the usual loading screen. */
    }
}

export const readBallotSent = (
    owner: SessionBallotOwner,
    electionId: string
): ISentBallotMarker | undefined => {
    try {
        const saved = JSON.parse(
            sessionStorage.getItem(SENT_BALLOT_KEY) ?? "null"
        ) as ISentBallotMarker | null
        if (
            saved &&
            saved.electionId === electionId &&
            typeof saved.trackingId === "string" &&
            sameOwner(saved.owner, owner)
        )
            return saved
    } catch {
        /* A malformed marker says nothing. */
    }
    return undefined
}

export const clearBallotSent = (electionId?: string): void => {
    try {
        if (electionId) {
            const saved = JSON.parse(
                sessionStorage.getItem(SENT_BALLOT_KEY) ?? "null"
            ) as ISentBallotMarker | null
            if (saved && saved.electionId !== electionId) return
        }
        sessionStorage.removeItem(SENT_BALLOT_KEY)
    } catch {
        /* Nothing to clear. */
    }
}
