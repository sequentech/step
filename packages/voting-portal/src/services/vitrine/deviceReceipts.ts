// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The voting portal keeps no receipts or Tracking IDs in the browser (design
// board DevicePlan): with a shared computer or any flaw in the page, a stored
// receipt says which ballot is whose. Earlier versions kept them, in
// localStorage and in Vitrine's receipt store, so the portal removes what
// they left each time it starts. It finds nothing after the first time.

/** The keep-or-forget choice earlier versions stored. */
export const OLD_KEEPING_KEY = "vitrine-receipt-keeping"
/** The start of the key of each receipt earlier versions kept. */
export const OLD_RECEIPT_PREFIX = "vitrine-receipt:"
/** The IndexedDB database of Vitrine's receipt store (its `STORE_DB_NAME`). */
export const OLD_RECEIPT_DB = "vitrine-receipts"

export interface IDeviceStores {
    localStorage?: Pick<Storage, "length" | "key" | "removeItem">
    indexedDB?: Pick<IDBFactory, "deleteDatabase">
}

const storesOf = (scope: typeof globalThis): IDeviceStores => {
    const stores: IDeviceStores = {}
    // Either can throw when the browser blocks site data.
    try {
        stores.localStorage = scope.localStorage
    } catch {
        // Nothing to clean there.
    }
    try {
        stores.indexedDB = scope.indexedDB
    } catch {
        // As above.
    }
    return stores
}

/** Removes the receipts and Tracking IDs earlier versions kept on this device. */
export const forgetDeviceReceipts = (stores: IDeviceStores = storesOf(globalThis)): void => {
    const local = stores.localStorage
    if (local) {
        try {
            const old: string[] = []
            for (let i = 0; i < local.length; i++) {
                const key = local.key(i)
                if (key === OLD_KEEPING_KEY || key?.startsWith(OLD_RECEIPT_PREFIX)) old.push(key)
            }
            old.forEach((key) => local.removeItem(key))
        } catch {
            // The browser may refuse; nothing new is written either way.
        }
    }
    try {
        stores.indexedDB?.deleteDatabase(OLD_RECEIPT_DB)
    } catch {
        // As above.
    }
}
