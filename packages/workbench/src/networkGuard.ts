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
 * reach a service fails visibly instead of contacting it. `allowed` admits other GETs, such
 * as the IVR emulator a framing tool serves from its own origin.
 */
export function installNetworkGuard(
    target: Pick<Window, "fetch" | "location">,
    onBlocked: (request: BlockedRequest) => void,
    allowed: (url: URL) => boolean = () => false
) {
    const original = target.fetch.bind(target)
    target.fetch = (input, init) => {
        const request = input instanceof Request ? input : undefined
        const method = (init?.method ?? request?.method ?? "GET").toUpperCase()
        const url = new URL(request?.url ?? String(input), target.location.href)
        if (method === "GET" && (url.origin === target.location.origin || allowed(url)))
            return original(input, init)
        onBlocked({method, url: url.href})
        return Promise.reject(
            new TypeError(`The workbench blocks ${method} ${url.href} in offline mode`)
        )
    }
    return () => {
        target.fetch = original
    }
}

/**
 * The files of the emulators a framing tool has named, and a way to name one: wasm-bindgen's
 * `<base>.js` and `<base>_bg.wasm`, with the base's query and fragment dropped as
 * `loadIvrEmulator` drops them. Nothing else at the tool's origin is admitted.
 */
export function emulatorAllowance() {
    const bases = new Set<string>()
    const allow = (baseUrl: string) => {
        const base = new URL(baseUrl)
        base.search = ""
        base.hash = ""
        bases.add(base.href)
    }
    const allowed = (url: URL) =>
        url.search === "" &&
        [...bases].some((base) => url.href === `${base}.js` || url.href === `${base}_bg.wasm`)
    return {allow, allowed}
}
