// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useCallback, useContext, useEffect, useState} from "react"
import {useApolloClient} from "@apollo/client/react"
import {AuthContext} from "../providers/AuthContextProvider"
import {IReceiptView, runCastReceipt, runKeptReceipt} from "../services/vitrine/castReceipt"
import {voterForward} from "../services/vitrine/forward"
import type {ITransparency} from "../services/vitrine/presentation"
import {trackingDigestHex} from "../services/vitrine/trackingId"
import {IVitrineSdk, loadVitrineSdk} from "../services/vitrine/sdk"
import {
    EReceiptKeeping,
    forgetReceipts,
    IKeptReceipt,
    keepReceipt,
    keptReceipts,
    receiptKeeping,
} from "../services/vitrine/storage"
import {ETransparencyMode} from "../services/vitrine/types"
import {voterContextFor} from "../services/vitrine/voterContext"
import type {IConfirmationVitrine} from "../store/castVotes/confirmationScreenDataSlice"

/** The ballot an election's receipt is about, by the Tracking ID the voter reads. */
export interface IReceiptBallot {
    eventId: string
    electionId: string
    trackingId: string
}

/** Keeps what a view holds: the chain, or the entry that waits for one. */
const keepView = async (
    sdk: IVitrineSdk,
    userId: string,
    target: IReceiptBallot,
    view: IReceiptView,
    isAllowed?: () => boolean
): Promise<boolean> => {
    const {chain, own, policy} = view
    if (!policy || (!chain && !own)) return false
    return keepReceipt(
        sdk,
        userId,
        {
            eventId: target.eventId,
            electionId: target.electionId,
            ballotId: target.trackingId,
            ...(own ? {acceptedAt: own.acceptedAt} : {}),
            policyId: policy.id,
            ...(chain ? {chain} : {}),
            savedAt: Date.now(),
        },
        {isAllowed}
    )
}

export interface ICastReceipt {
    view?: IReceiptView
    /** Whether this device keeps the receipt. */
    kept: boolean
}

/** The receipt of the ballot just cast: checked, upgraded while the page is
 * open, and kept on this device unless it is a kiosk or the voter said no.
 * Without `vitrine` (the event has no transparency, the ballot is a demo, or
 * the page does not know the ballot yet) there is nothing to check. */
export const useCastReceipt = (
    target: IReceiptBallot,
    vitrine: IConfirmationVitrine | undefined
): ICastReceipt => {
    const client = useApolloClient()
    const {isKiosk, userId} = useContext(AuthContext)
    const kiosk = isKiosk()
    const [view, setView] = useState<IReceiptView>()
    const [kept, setKept] = useState(false)
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

    useEffect(() => {
        if (!view || kiosk || !userId || receiptKeeping() !== EReceiptKeeping.KEEP) return
        let active = true
        const keep = async () => {
            let ok = false
            try {
                const sdk = await loadVitrineSdk()
                if (!active) return
                ok = await keepView(
                    sdk,
                    userId,
                    {eventId, electionId, trackingId},
                    view,
                    () => active
                )
            } catch {
                ok = false
            }
            if (active) setKept(ok)
        }
        void keep()
        return () => {
            active = false
        }
    }, [view, kiosk, userId, eventId, electionId, trackingId])

    return {view, kept}
}

export interface IKeptReceiptView {
    kept: IKeptReceipt
    view?: IReceiptView
}

export interface IKeptReceipts {
    receipts: IKeptReceiptView[]
    forgetAll: () => Promise<void>
}

/** The voter's receipts for the event kept on this device, each checked
 * after one attempt to upgrade it, or to get it for an entry that waits.
 * Nothing in kiosk mode, for events without transparency, or when the voter
 * said not to keep receipts. */
export const useKeptReceipts = (
    eventId: string | undefined,
    transparency: ITransparency
): IKeptReceipts => {
    const client = useApolloClient()
    const {isKiosk, userId} = useContext(AuthContext)
    const kiosk = isKiosk()
    const [receipts, setReceipts] = useState<IKeptReceiptView[]>([])
    const [receiptOwner, setReceiptOwner] = useState<string>()
    const off =
        !eventId ||
        !userId ||
        kiosk ||
        transparency.mode === ETransparencyMode.DISABLED ||
        receiptKeeping() === EReceiptKeeping.FORGET

    useEffect(() => {
        if (off || !eventId) {
            setReceipts([])
            return
        }
        const controller = new AbortController()
        const load = async () => {
            let sdk: IVitrineSdk
            let kept: IKeptReceipt[]
            try {
                sdk = await loadVitrineSdk()
                kept = await keptReceipts(sdk, userId, eventId)
            } catch {
                return
            }
            if (controller.signal.aborted) return
            setReceiptOwner(userId)
            setReceipts(kept.map((k) => ({kept: k})))
            kept.forEach((k) => {
                const ballot = {eventId, electionId: k.electionId, trackingId: k.ballotId}
                // Only a Tracking ID names a kept ballot: older receipts are left out.
                let digest: string
                try {
                    digest = sdk.pure.encoding.hexEncode(sdk.verify.decodeTrackingId(k.ballotId))
                } catch {
                    return
                }
                const target = {
                    eventId,
                    electionId: k.electionId,
                    stage: "cast" as const,
                    ballotId: digest,
                }
                void runKeptReceipt({
                    ...(k.chain ? {chain: k.chain} : {}),
                    ...(k.acceptedAt === undefined ? {} : {acceptedAt: k.acceptedAt}),
                    target,
                    transparency,
                    context: () => voterContextFor(client, eventId, k.electionId),
                    forward: voterForward(client, eventId),
                    signal: controller.signal,
                    onView: (view) => {
                        setReceipts((current) =>
                            current.map((r) =>
                                r.kept.electionId === k.electionId ? {...r, view} : r
                            )
                        )
                        if (view.chain && view.chain !== k.chain) {
                            void keepView(
                                sdk,
                                userId,
                                ballot,
                                view,
                                () => !controller.signal.aborted
                            )
                        }
                    },
                })
            })
        }
        void load()
        return () => controller.abort()
    }, [client, eventId, off, userId, transparency])

    const forgetAll = useCallback(async () => {
        if (!eventId) return
        try {
            await forgetReceipts(await loadVitrineSdk(), userId, eventId)
        } catch {
            // Nothing was kept.
        }
        setReceipts([])
    }, [eventId, userId])

    return {receipts: !off && receiptOwner === userId ? receipts : [], forgetAll}
}
