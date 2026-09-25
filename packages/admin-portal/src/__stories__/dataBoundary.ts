// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {testDataProvider, type DataProvider} from "react-admin"

export function dataBoundary(handlers: Partial<DataProvider>) {
    const calls: {method: string; args: unknown[]}[] = []
    const unexpected: string[] = []
    const provider = new Proxy(testDataProvider(), {
        get(target, key, receiver) {
            const original: unknown = Reflect.get(target, key, receiver)
            if (typeof original !== "function" || typeof key !== "string") return original
            return (...args: unknown[]) => {
                calls.push({method: key, args})
                const handler: unknown = handlers[key]
                if (typeof handler !== "function") {
                    unexpected.push(`${key}(${String(args[0])})`)
                    return Promise.reject(new Error(`Unexpected data operation: ${key}`))
                }
                return Reflect.apply(handler, handlers, args)
            }
        },
    })
    return {provider, calls, unexpected}
}
