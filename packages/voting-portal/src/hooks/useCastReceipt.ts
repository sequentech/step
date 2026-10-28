// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useCallback, useContext, useEffect, useState} from "react"
import type {ApolloClient} from "@apollo/client"
import {useApolloClient} from "@apollo/client/react"
import {AuthContext} from "../providers/AuthContextProvider"
import {
    VITRINE_VOTER_FORWARD,
    VitrineVoterForwardMutation,
    VitrineVoterForwardMutationVariables,
} from "../queries/VitrineVoterForward"
import {
    IReceiptView,
    runCastReceipt,
    runKeptReceipt,
    TForward,
} from "../services/vitrine/castReceipt"
import type {ITransparency} from "../services/vitrine/presentation"
import type {IReceiptTarget} from "../services/vitrine/receipt"
import {IVitrineSdk, loadVitrineSdk} from "../services/vitrine/sdk"
import {
    EReceiptKeeping,
    forgetReceipt,
    forgetReceipts,
    IKeptReceipt,
    keepReceipt,
    keptReceipts,
    receiptKeeping,
    setReceiptKeeping,
} from "../services/vitrine/storage"
import {ETransparencyMode} from "../services/vitrine/types"
import {voterContextFor} from "../services/vitrine/voterContext"
import type {IConfirmationVitrine} from "../store/castVotes/confirmationScreenDataSlice"

/** Harvest's `vitrine_voter_forward` for the event (private events). An
 * answer that didn't come reads as status 0. */
export const voterForward =
    (client: ApolloClient, eventId: string): TForward =>
    async (route, body, hub) => {
        try {
            const {data} = await client.mutate<
                VitrineVoterForwardMutation,
                VitrineVoterForwardMutationVariables
            >({
                mutation: VITRINE_VOTER_FORWARD,
                variables: {electionEventId: eventId, route, body, ...(hub ? {hub} : {})},
            })
            const answer = data?.vitrine_voter_forward
            return answer
                ? {status: answer.status, ...(answer.body ? {body: answer.body} : {})}
                : {status: 0}
        } catch {
            return {status: 0}
        }
    }

/** Keeps what a view holds: the chain, or the entry that waits for one. */
const keepView = async (
    sdk: IVitrineSdk,
    userId: string,
    target: IReceiptTarget,
    view: IReceiptView,
    isAllowed?: () => boolean
): Promise<boolean> => {
    const {chain, own, policy} = view
    if (!policy || (!chain && !own)) return false
    return keepReceipt(
        sdk,
        userId,
        {
            ...target,
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
    keeping: EReceiptKeeping
    /** Whether this device keeps the receipt. */
    kept: boolean
    setKeeping: (keeping: EReceiptKeeping) => void
}

/** The receipt of the ballot just cast: checked, upgraded while the page is
 * open, and kept on this device unless it is a kiosk or the voter said no. */
export const useCastReceipt = (
    target: IReceiptTarget,
    vitrine: IConfirmationVitrine
): ICastReceipt => {
    const client = useApolloClient()
    const {isKiosk, userId} = useContext(AuthContext)
    const kiosk = isKiosk()
    const [view, setView] = useState<IReceiptView>()
    const [keeping, setKeepingState] = useState<EReceiptKeeping>(receiptKeeping)
    const [kept, setKept] = useState(false)
    const {eventId, electionId, ballotId} = target

    useEffect(() => {
        const controller = new AbortController()
        void runCastReceipt({
            cast: vitrine.cast,
            target: {eventId, electionId, ballotId},
            transparency: vitrine.transparency,
            context: () => voterContextFor(client, eventId, electionId),
            forward: voterForward(client, eventId),
            signal: controller.signal,
            onView: setView,
        })
        return () => controller.abort()
    }, [client, eventId, electionId, ballotId, vitrine])

    useEffect(() => {
        if (!view || kiosk || !userId || keeping !== EReceiptKeeping.KEEP) return
        let active = true
        const keep = async () => {
            let ok = false
            try {
                const sdk = await loadVitrineSdk()
                if (!active) return
                ok = await keepView(
                    sdk,
                    userId,
                    {eventId, electionId, ballotId},
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
    }, [view, kiosk, keeping, userId, eventId, electionId, ballotId])

    const setKeeping = useCallback(
        (next: EReceiptKeeping) => {
            setReceiptKeeping(next)
            setKeepingState(next)
            if (next !== EReceiptKeeping.FORGET) return
            setKept(false)
            const forget = async () => {
                try {
                    await forgetReceipt(await loadVitrineSdk(), eventId, electionId)
                } catch {
                    // Nothing was kept.
                }
            }
            void forget()
        },
        [eventId, electionId]
    )

    return {view, keeping, kept, setKeeping}
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
                const target = {eventId, electionId: k.electionId, ballotId: k.ballotId}
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
                                target,
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
