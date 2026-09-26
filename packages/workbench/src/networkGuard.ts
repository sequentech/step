// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

/** Window event carrying a blocked request as `METHOD url` in its detail. */
export const BLOCKED_REQUEST_EVENT = "workbench:blocked-request"

export interface BlockedRequest {
    method: string
    url: string
}

/**
 * Lets only same-origin GET requests through `fetch`: the application's own modules and the
 * sequent-core WASM binary. Anything else is rejected and reported, so a screen that tries to
 * reach a service fails visibly instead of contacting it.
 */
export function installNetworkGuard(
    target: Pick<Window, "fetch" | "location">,
    onBlocked: (request: BlockedRequest) => void
) {
    const original = target.fetch.bind(target)
    target.fetch = (input, init) => {
        const request = input instanceof Request ? input : undefined
        const method = (init?.method ?? request?.method ?? "GET").toUpperCase()
        const url = new URL(request?.url ?? String(input), target.location.href)
        if (url.origin === target.location.origin && method === "GET") return original(input, init)
        onBlocked({method, url: url.href})
        return Promise.reject(
            new TypeError(`The workbench blocks ${method} ${url.href} in offline mode`)
        )
    }
    return () => {
        target.fetch = original
    }
}
