// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useEffect, useState} from "react"
import {useApolloClient} from "@apollo/client/react"
import {IReceiptView, runCastReceipt} from "../services/vitrine/castReceipt"
import {voterForward} from "../services/vitrine/forward"
import {trackingDigestHex} from "../services/vitrine/trackingId"
import {voterContextFor} from "../services/vitrine/voterContext"
import type {IConfirmationVitrine} from "../store/castVotes/confirmationScreenDataSlice"

/** The ballot an election's receipt is about, by the Tracking ID the voter reads. */
export interface IReceiptBallot {
    eventId: string
    electionId: string
    trackingId: string
}

export interface ICastReceipt {
    view?: IReceiptView
}

/** The receipt of the ballot just cast: checked, and upgraded while the page
 * is open. It lives in the page's memory only: this device keeps no receipts
 * and no Tracking IDs, and later checks start from the receipt the voter keeps.
 * Without `vitrine` (the event has no transparency, the ballot is a demo, or
 * the page does not know the ballot yet) there is nothing to check. */
export const useCastReceipt = (
    target: IReceiptBallot,
    vitrine: IConfirmationVitrine | undefined
): ICastReceipt => {
    const client = useApolloClient()
    const [view, setView] = useState<IReceiptView>()
    const {eventId, electionId, trackingId} = target

    useEffect(() => {
        if (!vitrine) return
        const controller = new AbortController()
        const run = async () => {
            const ballotId = await trackingDigestHex(trackingId)
            if (!ballotId || controller.signal.aborted) return
            await runCastReceipt({
                cast: vitrine.cast,
                target: {eventId, electionId, stage: "cast", ballotId},
                transparency: vitrine.transparency,
                context: () => voterContextFor(client, eventId, electionId),
                forward: voterForward(client, eventId),
                signal: controller.signal,
                onView: setView,
            })
        }
        void run()
        return () => controller.abort()
    }, [client, eventId, electionId, trackingId, vitrine])

    return {view}
}
