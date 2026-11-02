// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    forgetDeviceReceipts,
    OLD_KEEPING_KEY,
    OLD_RECEIPT_DB,
    OLD_RECEIPT_PREFIX,
} from "./deviceReceipts"

const deleteDatabase = jest.fn()

beforeEach(() => {
    localStorage.clear()
    deleteDatabase.mockClear()
})

describe("forgetting what earlier versions kept on this device", () => {
    it("removes the kept receipts and the keeping choice, and nothing else", () => {
        localStorage.setItem(OLD_KEEPING_KEY, "keep")
        localStorage.setItem(`${OLD_RECEIPT_PREFIX}event-1/election-1`, "{}")
        localStorage.setItem(`${OLD_RECEIPT_PREFIX}event-2/election-9`, "{}")
        localStorage.setItem("i18nextLng", "en")
        forgetDeviceReceipts({localStorage, indexedDB: {deleteDatabase}})
        expect(Object.keys(localStorage)).toEqual(["i18nextLng"])
    })

    it("deletes the receipt store's database", () => {
        forgetDeviceReceipts({localStorage, indexedDB: {deleteDatabase}})
        expect(deleteDatabase).toHaveBeenCalledWith(OLD_RECEIPT_DB)
    })

    it("writes nothing, even when there is nothing to remove", () => {
        const setItem = jest.spyOn(Storage.prototype, "setItem")
        forgetDeviceReceipts({localStorage, indexedDB: {deleteDatabase}})
        forgetDeviceReceipts({localStorage, indexedDB: {deleteDatabase}})
        expect(setItem).not.toHaveBeenCalled()
        expect(localStorage.length).toBe(0)
        setItem.mockRestore()
    })

    it("carries on when the browser blocks site data, or has no stores", () => {
        const blocked = {
            get length(): number {
                throw new DOMException("blocked", "SecurityError")
            },
            key: () => null,
            removeItem: () => undefined,
        }
        const refusing = {
            deleteDatabase: () => {
                throw new DOMException("blocked", "SecurityError")
            },
        }
        expect(() =>
            forgetDeviceReceipts({localStorage: blocked, indexedDB: refusing})
        ).not.toThrow()
        expect(() => forgetDeviceReceipts({})).not.toThrow()
        expect(() => forgetDeviceReceipts()).not.toThrow()
    })
})
