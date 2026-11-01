// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useContext, useEffect, useState} from "react"
import type {Policy} from "@vitrine/verify"
import {AuthContext} from "../providers/AuthContextProvider"
import type {ITransparency} from "../services/vitrine/presentation"
import {loadVitrineSdk} from "../services/vitrine/sdk"
import {
    bucketServers,
    EShortIdKind,
    EShortIdStatus,
    IShortIdResult,
    lookupShortId,
    shortIdToLook,
} from "../services/vitrine/shortId"
import {EReceiptKeeping, keptReceipts, receiptKeeping} from "../services/vitrine/storage"
import {ETransparencyMode} from "../services/vitrine/types"
import {useVitrinePolicyState} from "./useVitrinePolicy"

export interface IShortIdMatches {
    status: EShortIdStatus
    result?: IShortIdResult
}

/** The tracking IDs of the receipts this device keeps for the event. */
const keptIds = async (userId: string, eventId: string): Promise<string[]> => {
    try {
        const sdk = await loadVitrineSdk()
        return (await keptReceipts(sdk, userId, eventId)).map((kept) => kept.ballotId)
    } catch {
        return []
    }
}

interface IBucketLookup {
    /** The short ID's characters, when the input is one to look up. */
    look: string | undefined
    eventId: string | undefined
    /** Whether the event has a public policy to name the servers. */
    applicable: boolean
    policy: Policy | undefined
    /** The wait for the policy is over. */
    settled: boolean
}

/** Looks the short ID up in the buckets of the policy's servers. */
export const useBucketMatches = ({
    look,
    eventId,
    applicable: wanted,
    policy,
    settled,
}: IBucketLookup): IShortIdMatches => {
    const {userId, isKiosk} = useContext(AuthContext)
    const kiosk = isKiosk()
    const applicable = look !== undefined && !!eventId && wanted
    const [answer, setAnswer] = useState<{look: string; result?: IShortIdResult}>()

    useEffect(() => {
        if (!applicable || !settled || !look || !eventId) return
        if (!policy || bucketServers(policy).length === 0) {
            setAnswer({look, result: undefined})
            return
        }
        const controller = new AbortController()
        const run = async () => {
            const kept =
                userId && !kiosk && receiptKeeping() !== EReceiptKeeping.FORGET
                    ? await keptIds(userId, eventId)
                    : []
            try {
                const result = await lookupShortId(policy, look, kept, {signal: controller.signal})
                if (!controller.signal.aborted) setAnswer({look, result})
            } catch {
                if (!controller.signal.aborted) {
                    setAnswer({
                        look,
                        result: {
                            kind: EShortIdKind.UNAVAILABLE,
                            typed: look,
                            matches: [],
                            total: 0,
                            truncated: false,
                        },
                    })
                }
            }
        }
        void run()
        return () => controller.abort()
    }, [applicable, settled, policy, look, eventId, userId, kiosk])

    if (!applicable) return {status: EShortIdStatus.OFF}
    if (!settled || answer?.look !== look) return {status: EShortIdStatus.LOADING}
    return {status: EShortIdStatus.DONE, ...(answer.result ? {result: answer.result} : {})}
}

/** The ballots that share the typed short ID, from a bucket of the policy's
 * log server or a monitor, so that no server is told which ballot is looked
 * for. The receipt this device keeps decides which one is the voter's. When
 * no server of the policy lists buckets, the result says so, and the routes
 * that send the ID to the election's server take over. */
export const useShortIdMatches = (
    typed: string | undefined,
    eventId: string | undefined,
    transparency: ITransparency
): IShortIdMatches => {
    const {policy, settled} = useVitrinePolicyState(transparency)
    return useBucketMatches({
        look: typed === undefined ? undefined : shortIdToLook(typed),
        eventId,
        applicable:
            transparency.mode === ETransparencyMode.PUBLIC && !!transparency.vitrine?.policy,
        policy,
        settled,
    })
}
