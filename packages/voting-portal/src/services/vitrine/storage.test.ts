// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import * as verify from "@vitrine/verify"
import * as pure from "@vitrine/verify/pure"
import {ballotEntry, chainText, ELECTION_ID, EVENT_ID, KEYS, scenario} from "../../fixtures/vitrine"
import type {IVitrineSdk} from "./sdk"
import {
    EReceiptKeeping,
    forgetReceipts,
    IKeptReceipt,
    keepReceipt,
    keptReceipts,
    receiptKeeping,
    setReceiptKeeping,
} from "./storage"

/** Vitrine's receipt store keeps chains in IndexedDB, which jsdom lacks. */
class MemoryReceiptStore {
    static readonly chains = new Map<string, string>()

    async put(chain: string, opts: {election?: string} = {}) {
        MemoryReceiptStore.chains.set(opts.election ?? "", chain)
    }

    async get(election: string) {
        const chain = MemoryReceiptStore.chains.get(election)
        return chain === undefined ? undefined : {election, chain}
    }

    async delete(election: string) {
        MemoryReceiptStore.chains.delete(election)
    }
}

const sdk = {
    verify: {...verify, ReceiptStore: MemoryReceiptStore},
    pure,
} as unknown as IVitrineSdk

const OTHER_ELECTION = "9a8b7c6d-5e4f-4321-8765-0fedcba98765"
const persist = jest.fn(async () => true)

const s = scenario()
const chainFor = (ballotId: string): string => {
    const index = s.ballots.add(ballotEntry(ballotId))
    const receipt = s.ballots.receipt(index, index + 1, KEYS.witnesses.slice(0, 2))
    return chainText(s.policyId, "ballots", {issued: pure.bytes.utf8(receipt)})
}

const kept = (overrides: Partial<IKeptReceipt> = {}): IKeptReceipt => ({
    eventId: EVENT_ID,
    electionId: ELECTION_ID,
    ballotId: "aa".repeat(32),
    policyId: s.policyId,
    savedAt: 1,
    ...overrides,
})

beforeAll(() => {
    Object.defineProperty(navigator, "storage", {value: {persist}, configurable: true})
})

beforeEach(() => {
    localStorage.clear()
    MemoryReceiptStore.chains.clear()
})

describe("receipts kept on this device", () => {
    it("keeps one receipt per election, shown only to the voter who cast it", async () => {
        const first = chainFor("01".repeat(32))
        const second = chainFor("02".repeat(32))
        expect(await keepReceipt(sdk, "voter-1", kept({chain: first}))).toBe(true)
        expect(await keepReceipt(sdk, "voter-1", kept({chain: second, savedAt: 2}))).toBe(true)
        const mine = await keptReceipts(sdk, "voter-1", EVENT_ID)
        expect(mine).toHaveLength(1)
        expect(mine[0]).toMatchObject({electionId: ELECTION_ID, chain: second, savedAt: 2})
        expect(await keptReceipts(sdk, "voter-2", EVENT_ID)).toEqual([])
        expect(persist).toHaveBeenCalledTimes(1)
        // The voter's ID stays off the device.
        expect(JSON.stringify(Object.entries(localStorage))).not.toContain("voter-1")
    })

    it("keeps an entry that waits for its receipt as its time", async () => {
        expect(await keepReceipt(sdk, "voter-1", kept({acceptedAt: 1_799_999_995}))).toBe(true)
        expect(await keptReceipts(sdk, "voter-1", EVENT_ID)).toEqual([
            kept({acceptedAt: 1_799_999_995}),
        ])
    })

    it("keeps nothing it couldn't show again", async () => {
        expect(await keepReceipt(sdk, "voter-1", kept())).toBe(false)
        expect(await keptReceipts(sdk, "voter-1", EVENT_ID)).toEqual([])
    })

    it("drops a chain that isn't the one it kept", async () => {
        await keepReceipt(sdk, "voter-1", kept({chain: chainFor("03".repeat(32))}))
        MemoryReceiptStore.chains.set(`${EVENT_ID}/${ELECTION_ID}`, chainFor("04".repeat(32)))
        expect(await keptReceipts(sdk, "voter-1", EVENT_ID)).toEqual([])
    })

    it("forgets the voter's receipts for the event, and only theirs", async () => {
        await keepReceipt(sdk, "voter-1", kept({chain: chainFor("05".repeat(32))}))
        await keepReceipt(
            sdk,
            "voter-2",
            kept({electionId: OTHER_ELECTION, chain: chainFor("06".repeat(32))})
        )
        await forgetReceipts(sdk, "voter-1", EVENT_ID)
        expect(await keptReceipts(sdk, "voter-1", EVENT_ID)).toEqual([])
        expect(await keptReceipts(sdk, "voter-2", EVENT_ID)).toHaveLength(1)
        expect(MemoryReceiptStore.chains.has(`${EVENT_ID}/${ELECTION_ID}`)).toBe(false)
    })

    it("remembers that the voter doesn't want receipts kept", () => {
        expect(receiptKeeping()).toBe(EReceiptKeeping.KEEP)
        setReceiptKeeping(EReceiptKeeping.FORGET)
        expect(receiptKeeping()).toBe(EReceiptKeeping.FORGET)
        expect(localStorage.getItem("vitrine-receipt-keeping")).toBe(EReceiptKeeping.FORGET)
    })
})
