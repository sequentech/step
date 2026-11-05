// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useEffect, useState} from "react"
import type {BallotStage} from "@vitrine/verify"
import {ballotIdChars} from "../services/vitrine/ballotId"
import {trackingDigestHex} from "../services/vitrine/trackingId"
import {IReceiptView, runTrackedReceipt} from "../services/vitrine/castReceipt"
import type {ITrackerReceipt} from "../services/vitrine/ballotTracker"
import type {ITransparency} from "../services/vitrine/presentation"
import {ETransparencyMode} from "../services/vitrine/types"

interface IFetchedReceipt {
    key: string
    view?: IReceiptView
    done: boolean
}

export interface ITrackedBallot {
    eventId: string
    electionId: string
    /** The ballot's Tracking ID. */
    ballotId: string
    /** The stage of the entry to check: `cast` unless the ballot was audited. */
    stage?: BallotStage
    /** The entry's `accepted_at`, in POSIX seconds, when the portal knows it. */
    acceptedAt?: number
}

/** The receipt of a ballot the tracker found: for public events, one attempt
 * to get it from the log, which the ballot's cast time lets the portal ask
 * for. This device keeps no receipts, and a private event's receipt comes only
 * to the page that cast the ballot, so there is nothing else to ask for. */
export const useTrackedReceipt = (
    ballot: ITrackedBallot | undefined,
    transparency: ITransparency
): ITrackerReceipt => {
    const [fetched, setFetched] = useState<IFetchedReceipt>()
    const eventId = ballot?.eventId
    const electionId = ballot?.electionId
    const ballotId = ballot ? ballotIdChars(ballot.ballotId) : undefined
    const stage = ballot?.stage ?? "cast"
    const acceptedAt = ballot?.acceptedAt
    const key = `${eventId}/${electionId}/${stage}/${ballotId}`
    const asking =
        eventId !== undefined &&
        electionId !== undefined &&
        ballotId !== undefined &&
        acceptedAt !== undefined &&
        transparency.mode === ETransparencyMode.PUBLIC

    useEffect(() => {
        if (!asking || !eventId || !electionId || !ballotId || acceptedAt === undefined) return
        const controller = new AbortController()
        setFetched({key, done: false})
        const run = async () => {
            const digest = await trackingDigestHex(ballotId)
            if (digest) {
                await runTrackedReceipt({
                    acceptedAt,
                    target: {eventId, electionId, stage, ballotId: digest},
                    transparency,
                    signal: controller.signal,
                    onView: (view) => setFetched({key, view, done: false}),
                })
            }
            if (controller.signal.aborted) return
            setFetched((current) => ({
                key,
                ...(current?.key === key && current.view ? {view: current.view} : {}),
                done: true,
            }))
        }
        void run()
        return () => controller.abort()
    }, [asking, key, eventId, electionId, ballotId, stage, acceptedAt, transparency])

    const mine = fetched?.key === key ? fetched : undefined
    return {view: mine?.view, settled: !asking || Boolean(mine?.done)}
}
