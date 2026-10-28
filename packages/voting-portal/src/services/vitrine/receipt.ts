// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The voter's own entry and receipt chain (Vitrine's specs/entry.md, section
// 3, and specs/receipt.md, sections 1 to 3). The portal knows every field of
// the entry but `accepted_at`, the ballot box's time, which it takes from the
// entry the log answered for: the receipt's, or the one whose leaf hash a
// promise names.

import type {ChainResult, Expect, TierName, TierStatus} from "@vitrine/verify"
import type {blocks} from "@vitrine/verify/pure"
import type {IVitrineSdk} from "./sdk"
import {EVitrineLog} from "./types"

/** The ballot the voter cast, as the portal itself knows it. */
export interface IReceiptTarget {
    eventId: string
    electionId: string
    /** The portal's own ballot ID: the hex of the ballot hash. */
    ballotId: string
}

/** The voter's entry, and the `accepted_at` it was built with. */
export interface IOwnEntry {
    entry: Uint8Array
    acceptedAt: number
}

// The ballot box accepts a ballot before the log promises it, and their
// clocks may differ: a promise's entry is looked for this far around the
// batch's time, in seconds.
const ACCEPTED_BEFORE_BATCH = 600
const ACCEPTED_AFTER_BATCH = 60

const CHAIN_KEYS = ["policy", "log"]
const HUB_BLOCKS = new Set([
    "anchor",
    "opening",
    "previous-anchor",
    "previous-opening",
    "opening-consistency",
    "attestation",
    "anchor-consistency",
    "browser-link",
    "browser-link-consistency",
    "browser",
    "bootstrap",
    "browser-consistency",
])
const BROKEN: readonly TierStatus[] = ["invalid", "failed"]

/** What the entry must be, from the portal's own IDs and, once known, the
 * entry's `accepted_at` (specs/receipt.md, section 1.1). */
export const receiptExpect = (target: IReceiptTarget, acceptedAt?: number): Expect => ({
    fields: {
        event_id: target.eventId,
        election_id: target.electionId,
        ballot_id: target.ballotId.toLowerCase(),
        ...(acceptedAt === undefined ? {} : {accepted_at: String(acceptedAt)}),
    },
})

/** The voter's entry as the ballot box accepted it at `acceptedAt`. */
export const ownEntry = (
    sdk: IVitrineSdk,
    target: IReceiptTarget,
    acceptedAt: number
): Uint8Array | undefined => {
    try {
        return sdk.verify.buildElectionEntry({
            eventId: target.eventId,
            electionId: target.electionId,
            ballotId: target.ballotId.toLowerCase(),
            acceptedAt,
        })
    } catch {
        return undefined
    }
}

/** The voter's entry as the ballot box accepted it at `acceptedAt`, with it. */
export const entryAt = (
    sdk: IVitrineSdk,
    target: IReceiptTarget,
    acceptedAt: number
): IOwnEntry | undefined => {
    const entry = ownEntry(sdk, target, acceptedAt)
    return entry && {entry, acceptedAt}
}

const acceptedAtOf = (sdk: IVitrineSdk, entry: Uint8Array): number | undefined => {
    try {
        const at = sdk.pure.entries.fieldU64(sdk.pure.entries.parseEntry(entry), "accepted_at")
        return at === undefined || at > BigInt(Number.MAX_SAFE_INTEGER) ? undefined : Number(at)
    } catch {
        return undefined
    }
}

/** The voter's entry, if `entry` is it. */
const asOwn = (
    sdk: IVitrineSdk,
    target: IReceiptTarget,
    entry: Uint8Array
): IOwnEntry | undefined => {
    const acceptedAt = acceptedAtOf(sdk, entry)
    if (acceptedAt === undefined) return undefined
    const own = ownEntry(sdk, target, acceptedAt)
    return own && sdk.pure.bytes.equalBytes(own, entry) ? {entry: own, acceptedAt} : undefined
}

/** The voter's entry in a receipt (its tlog-proof's extra), if it is theirs. */
export const receiptEntry = (
    sdk: IVitrineSdk,
    target: IReceiptTarget,
    receipt: string
): IOwnEntry | undefined => {
    try {
        const {extra} = sdk.pure.proofs.parseTlogProof(receipt)
        return extra && asOwn(sdk, target, extra)
    } catch {
        return undefined
    }
}

/** The entry Harvest sent with `queued` or `unavailable` (base64), if it is
 * the voter's own. */
export const castEntry = (
    sdk: IVitrineSdk,
    target: IReceiptTarget,
    text: string
): IOwnEntry | undefined => {
    try {
        return asOwn(sdk, target, sdk.pure.encoding.b64decode(text.trim()))
    } catch {
        return undefined
    }
}

/** The voter's entry whose leaf hash a promise names, found near the
 * promise batch's time. */
export const promisedEntry = (
    sdk: IVitrineSdk,
    target: IReceiptTarget,
    leaf: Uint8Array,
    batchTime: number
): IOwnEntry | undefined => {
    const matches = (t: number): IOwnEntry | undefined => {
        const own = entryAt(sdk, target, t)
        return own && sdk.pure.bytes.equalBytes(sdk.pure.hashes.leafHash(own.entry), leaf)
            ? own
            : undefined
    }
    if (!ownEntry(sdk, target, batchTime)) return undefined
    for (let d = 0; d <= ACCEPTED_BEFORE_BATCH; d++) {
        const before = matches(batchTime - d)
        if (before) return before
        const after = d > 0 && d <= ACCEPTED_AFTER_BATCH ? matches(batchTime + d) : undefined
        if (after) return after
    }
    return undefined
}

export interface IChainBlocks {
    policyAck?: string
    promise?: string
    issued?: string
}

/** A receipt chain for the ballots log with these blocks. */
export const chainDocument = (sdk: IVitrineSdk, policyId: string, b: IChainBlocks): string => {
    const {utf8, fromUtf8} = sdk.pure.bytes
    return fromUtf8(
        sdk.pure.formatChain({
            policyId,
            log: EVitrineLog.BALLOTS,
            hubs: new Map(),
            unknown: [],
            ...(b.policyAck ? {policyAck: utf8(b.policyAck)} : {}),
            ...(b.promise ? {promise: utf8(b.promise)} : {}),
            ...(b.issued ? {issued: utf8(b.issued)} : {}),
        })
    )
}

/** The receipt in a chain the upgrades can start from: its record, else the
 * receipt the log issued. */
export const chainReceipt = (sdk: IVitrineSdk, chain: string): string | undefined => {
    try {
        const c = sdk.pure.parseChain(chain)
        const receipt = c.record ?? c.issued
        return receipt && sdk.pure.bytes.fromUtf8(receipt)
    } catch {
        return undefined
    }
}

/** The chain's promise, when it has no receipt yet. */
export const chainPromise = (sdk: IVitrineSdk, chain: string): string | undefined => {
    try {
        const c = sdk.pure.parseChain(chain)
        return !c.record && !c.issued && c.promise ? sdk.pure.bytes.fromUtf8(c.promise) : undefined
    } catch {
        return undefined
    }
}

const statusOf = (result: ChainResult, tier: TierName): TierStatus | undefined =>
    result.tiers.find((t) => t.tier === tier)?.status

/** Whether the result holds a receipt or promise for the voter's entry:
 * a record tier that isn't invalid or failed. */
export const hasReceipt = (result: ChainResult): boolean => {
    const record = statusOf(result, "record")
    return record !== undefined && !BROKEN.includes(record)
}

/** Whether to keep the chain `after` describes instead of the one `before`
 * does: it still holds a receipt and no tier turned invalid, which is damaged
 * data. A tier that turned failed is kept, as the evidence it is. */
export const improves = (before: ChainResult, after: ChainResult): boolean =>
    hasReceipt(after) &&
    !after.tiers.some((t) => t.status === "invalid" && statusOf(before, t.tier) !== "invalid")

/** Merges the blocks of a voter route's answer, a receipt chain of the same
 * policy and log, into `chain`: the ones `take` picks replace the chain's.
 * Undefined when the answer isn't such a chain or adds nothing. */
export const mergeChain = (
    sdk: IVitrineSdk,
    chain: string,
    answer: string,
    take: (block: blocks.Block, current: readonly blocks.Block[]) => boolean,
    acceptedPolicy?: (id: string) => boolean
): string | undefined => {
    const format = {
        header: sdk.pure.CHAIN_HEADER,
        keys: CHAIN_KEYS,
        code: "malformed-chain" as const,
    }
    try {
        const base = sdk.pure.blocks.parseBlockDoc(chain, format)
        const got = sdk.pure.blocks.parseBlockDoc(answer, format)
        const same = base.values.every((v, i) => {
            const incoming = got.values[i]?.join(" ")
            return (
                v.join(" ") === incoming ||
                (i === 0 && incoming !== undefined && acceptedPolicy?.(incoming) === true)
            )
        })
        const incoming = got.blocks.filter((b) => take(b, base.blocks))
        if (!same || incoming.length === 0) return undefined
        const key = (b: blocks.Block) =>
            HUB_BLOCKS.has(b.type) ? `${b.type} ${b.args[0]}` : b.type
        const replaced = new Set(incoming.map(key))
        const merged = sdk.pure.blocks.formatBlockDoc(
            format.header,
            CHAIN_KEYS.map((k, i) => `${k} ${base.values[i].join(" ")}`),
            [...base.blocks.filter((b) => !replaced.has(key(b))), ...incoming]
        )
        return sdk.pure.bytes.fromUtf8(sdk.pure.formatChain(sdk.pure.parseChain(merged)))
    } catch {
        return undefined
    }
}

/** The blocks `POST voter/upgrade` adds: the record and its consistency
 * proof, and the issued receipt when the chain has none. */
export const upgradeBlocks = (block: blocks.Block, current: readonly blocks.Block[]): boolean =>
    block.type === "record" ||
    block.type === "consistency" ||
    (block.type === "issued" && !current.some((b) => b.type === "issued"))

/** The blocks `POST voter/opening?hub=<hub>` adds: that hub's. */
export const hubBlocks =
    (hub: string) =>
    (block: blocks.Block): boolean =>
        HUB_BLOCKS.has(block.type) && block.args[0] === hub
