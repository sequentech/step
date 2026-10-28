// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Receipts kept on this device (step-integration.md, section 2.6): one per
// election, shown again after login, and only to the voter who cast it. The
// chain goes to Vitrine's receipt store; the rest, which the chain doesn't
// say for a promise or an entry still waiting for its receipt, goes to
// localStorage. Callers keep nothing in kiosk mode.

import type {ReceiptStore} from "@vitrine/verify"
import type {IVitrineSdk} from "./sdk"

/** Whether this device keeps the voter's receipts. */
export enum EReceiptKeeping {
    KEEP = "keep",
    FORGET = "forget",
}

export interface IKeptReceipt {
    eventId: string
    electionId: string
    ballotId: string
    /** The entry's `accepted_at`, once the log answered for it. */
    acceptedAt?: number
    policyId: string
    /** The receipt chain; none while the entry waits for its receipt. */
    chain?: string
    /** Milliseconds since the epoch. */
    savedAt: number
}

interface IKeptRecord {
    /** SHA-256 of the voter's user ID, hex. */
    voter: string
    ballotId: string
    acceptedAt?: number
    policyId: string
    /** SHA-256 of the chain in the receipt store, hex. */
    chain?: string
    savedAt: number
}

type TStorage = typeof globalThis.localStorage

const KEEPING_KEY = "vitrine-receipt-keeping"
const RECORD_PREFIX = "vitrine-receipt:"

let store: ReceiptStore | undefined
let keepingHere: EReceiptKeeping | undefined
let askedToPersist = false

const local = (): TStorage | undefined => {
    try {
        return globalThis.localStorage ?? undefined
    } catch {
        return undefined
    }
}

const electionKey = (eventId: string, electionId: string): string => `${eventId}/${electionId}`

const recordKey = (eventId: string, electionId: string): string =>
    `${RECORD_PREFIX}${electionKey(eventId, electionId)}`

const digest = (sdk: IVitrineSdk, text: string): string =>
    sdk.pure.encoding.hexEncode(sdk.pure.hashes.sha256(sdk.pure.bytes.utf8(text)))

const receiptStore = (sdk: IVitrineSdk): ReceiptStore => {
    store ??= new sdk.verify.ReceiptStore({persist: false})
    return store
}

/** Asks the browser once to keep this site's data (`navigator.storage.persist()`). */
const askToPersist = async (): Promise<void> => {
    if (askedToPersist) return
    askedToPersist = true
    try {
        await globalThis.navigator?.storage?.persist?.()
    } catch {
        // The browser may keep the data anyway; it decides.
    }
}

const isRecord = (value: unknown): value is IKeptRecord => {
    if (typeof value !== "object" || value === null) return false
    const r = value as Record<string, unknown>
    return (
        typeof r.voter === "string" &&
        typeof r.ballotId === "string" &&
        typeof r.policyId === "string" &&
        typeof r.savedAt === "number" &&
        (r.acceptedAt === undefined || Number.isSafeInteger(r.acceptedAt)) &&
        (r.chain === undefined || typeof r.chain === "string")
    )
}

const readRecord = (storage: TStorage, key: string): IKeptRecord | undefined => {
    try {
        const value: unknown = JSON.parse(storage.getItem(key) ?? "null")
        return isRecord(value) ? value : undefined
    } catch {
        return undefined
    }
}

export const receiptKeeping = (): EReceiptKeeping => {
    if (keepingHere) return keepingHere
    try {
        return local()?.getItem(KEEPING_KEY) === EReceiptKeeping.FORGET
            ? EReceiptKeeping.FORGET
            : EReceiptKeeping.KEEP
    } catch {
        return EReceiptKeeping.KEEP
    }
}

export const setReceiptKeeping = (keeping: EReceiptKeeping): void => {
    keepingHere = keeping
    try {
        local()?.setItem(KEEPING_KEY, keeping)
    } catch {
        // Without storage the choice lasts as long as the page.
    }
}

/** Keeps the voter's receipt for its election, replacing the one there was.
 * False when this device can't keep it. */
export const keepReceipt = async (
    sdk: IVitrineSdk,
    userId: string,
    r: IKeptReceipt,
    options: {isAllowed?: () => boolean} = {}
): Promise<boolean> => {
    const storage = local()
    const allowed = () =>
        options.isAllowed?.() !== false && receiptKeeping() !== EReceiptKeeping.FORGET
    if (!storage || !userId || !allowed()) return false
    const key = electionKey(r.eventId, r.electionId)
    let chain: string | undefined
    try {
        if (r.chain) {
            await receiptStore(sdk).put(r.chain, {election: key})
            chain = digest(sdk, r.chain)
        } else {
            await receiptStore(sdk).delete(key)
        }
    } catch {
        chain = undefined
    }
    if (!allowed()) {
        try {
            await receiptStore(sdk).delete(key)
        } catch {
            /* No receipt kept. */
        }
        return false
    }
    if (!chain && r.acceptedAt === undefined) return false
    const record: IKeptRecord = {
        voter: digest(sdk, userId),
        ballotId: r.ballotId,
        ...(r.acceptedAt === undefined ? {} : {acceptedAt: r.acceptedAt}),
        policyId: r.policyId,
        ...(chain ? {chain} : {}),
        savedAt: r.savedAt,
    }
    try {
        storage.setItem(recordKey(r.eventId, r.electionId), JSON.stringify(record))
    } catch {
        return false
    }
    await askToPersist()
    return true
}

/** The voter's receipts for the event kept on this device. */
export const keptReceipts = async (
    sdk: IVitrineSdk,
    userId: string,
    eventId: string
): Promise<IKeptReceipt[]> => {
    const storage = local()
    if (!storage) return []
    const voter = digest(sdk, userId)
    const prefix = `${RECORD_PREFIX}${eventId}/`
    const keys: string[] = []
    try {
        for (let i = 0; i < storage.length; i++) {
            const key = storage.key(i)
            if (key?.startsWith(prefix)) keys.push(key)
        }
    } catch {
        return []
    }
    const kept: IKeptReceipt[] = []
    for (const key of keys.sort()) {
        const record = readRecord(storage, key)
        if (!record || record.voter !== voter) continue
        const electionId = key.slice(prefix.length)
        let chain: string | undefined
        try {
            chain = (await receiptStore(sdk).get(electionKey(eventId, electionId)))?.chain
        } catch {
            chain = undefined
        }
        const theirs = chain !== undefined && record.chain === digest(sdk, chain)
        if (!theirs && record.acceptedAt === undefined) continue
        kept.push({
            eventId,
            electionId,
            ballotId: record.ballotId,
            ...(record.acceptedAt === undefined ? {} : {acceptedAt: record.acceptedAt}),
            policyId: record.policyId,
            ...(theirs ? {chain} : {}),
            savedAt: record.savedAt,
        })
    }
    return kept
}

/** Removes the receipt kept for the election. */
export const forgetReceipt = async (
    sdk: IVitrineSdk,
    eventId: string,
    electionId: string
): Promise<void> => {
    try {
        local()?.removeItem(recordKey(eventId, electionId))
    } catch {
        // Nothing kept, then.
    }
    try {
        await receiptStore(sdk).delete(electionKey(eventId, electionId))
    } catch {
        // As above.
    }
}

/** Removes the voter's receipts for the event. */
export const forgetReceipts = async (
    sdk: IVitrineSdk,
    userId: string,
    eventId: string
): Promise<void> => {
    const kept = await keptReceipts(sdk, userId, eventId)
    for (const r of kept) await forgetReceipt(sdk, r.eventId, r.electionId)
}
