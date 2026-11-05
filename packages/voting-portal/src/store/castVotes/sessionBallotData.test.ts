// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {
    BALLOT_DATA_KEY,
    BALLOT_DATA_EXPIRATION_KEY,
    clearSessionStorageBallotData,
    BALLOT_LOCATOR_KEY,
    readSessionBallotLocator,
    discardForeignSessionBallotLocator,
    snapshotSessionBallotData,
} from "./sessionBallotData"

beforeEach(() => sessionStorage.clear())
afterEach(() => {
    sessionStorage.clear()
})

const owner = {tenantId: "tenant", eventId: "event", userId: "voter"}
const locator = {owner, electionId: "election", ballotId: "FTBE-MHRX"}

it("expiry keeps only a scoped locator and logout removes it", () => {
    sessionStorage.setItem(BALLOT_DATA_KEY, "ciphertext-and-signed-request")
    sessionStorage.setItem(BALLOT_DATA_EXPIRATION_KEY, "1700000000")
    clearSessionStorageBallotData(snapshotSessionBallotData(), locator)
    expect(sessionStorage.getItem(BALLOT_DATA_KEY)).toBeNull()
    expect(sessionStorage.getItem(BALLOT_DATA_EXPIRATION_KEY)).toBeNull()
    expect(JSON.parse(sessionStorage.getItem(BALLOT_LOCATOR_KEY)!)).toEqual(locator)
    expect(readSessionBallotLocator(owner, "election")).toEqual(locator)
    expect(readSessionBallotLocator({...owner, userId: "other"}, "election")).toBeUndefined()
    expect(readSessionBallotLocator(owner, "other-election")).toBeUndefined()
    clearSessionStorageBallotData()
    expect(sessionStorage.getItem(BALLOT_LOCATOR_KEY)).toBeNull()
})

it("late cleanup preserves a newer request and its locator", () => {
    const older = snapshotSessionBallotData()
    sessionStorage.setItem(BALLOT_DATA_KEY, "new-request")
    sessionStorage.setItem(BALLOT_LOCATOR_KEY, JSON.stringify(locator))
    expect(clearSessionStorageBallotData(older)).toBe(false)
    expect(readSessionBallotLocator(owner, "election")).toEqual(locator)
    expect(sessionStorage.getItem(BALLOT_DATA_KEY)).toBe("new-request")
})

it("frees the expired payload before saving the small locator in a full storage area", () => {
    sessionStorage.setItem(BALLOT_DATA_KEY, "large-signed-request")
    const original = Storage.prototype.setItem
    const write = jest.spyOn(Storage.prototype, "setItem").mockImplementation(function (
        this: Storage,
        key: string,
        value: string
    ) {
        if (key === BALLOT_LOCATOR_KEY && this.getItem(BALLOT_DATA_KEY))
            throw new DOMException("Full storage", "QuotaExceededError")
        original.call(this, key, value)
    })
    try {
        expect(() =>
            clearSessionStorageBallotData(snapshotSessionBallotData(), locator)
        ).not.toThrow()
        expect(readSessionBallotLocator(owner, "election")).toEqual(locator)
    } finally {
        write.mockRestore()
    }
})

it("owner changes discard the reference and assurance upgrades preserve it", () => {
    sessionStorage.setItem(BALLOT_LOCATOR_KEY, JSON.stringify(locator))
    discardForeignSessionBallotLocator(owner)
    expect(readSessionBallotLocator(owner, "election")).toEqual(locator)
    discardForeignSessionBallotLocator({...owner, eventId: "other-event"})
    expect(sessionStorage.getItem(BALLOT_LOCATOR_KEY)).toBeNull()
})

it.each(["invalid-json", "null", JSON.stringify({...locator, ballotId: "invalid-id"})])(
    "refuses malformed locator references: %s",
    (saved) => {
        sessionStorage.setItem(BALLOT_LOCATOR_KEY, saved)
        expect(readSessionBallotLocator(owner, "election")).toBeUndefined()
    }
)

it("clears the ballot and expiry together while preserving unrelated session preferences", () => {
    sessionStorage.setItem(BALLOT_DATA_KEY, "synthetic-private-ballot")
    sessionStorage.setItem(BALLOT_DATA_EXPIRATION_KEY, "1700000000")
    sessionStorage.setItem("language", "en")
    clearSessionStorageBallotData()
    clearSessionStorageBallotData() // Repeated cleanup must be harmless after logout/retry.
    expect(sessionStorage.getItem(BALLOT_DATA_KEY)).toBeNull()
    expect(sessionStorage.getItem(BALLOT_DATA_EXPIRATION_KEY)).toBeNull()
    expect(sessionStorage.getItem("language")).toBe("en")
})
