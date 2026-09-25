// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {defineBrowserCommand} from "@vitest/browser-playwright"
import type {Page, Route} from "playwright"

const guards = new WeakMap<Page, {unexpected: string[]; route: (route: Route) => Promise<void>}>()

export const startNetworkGuard = defineBrowserCommand(async ({page}) => {
    const origin = new URL(page.url()).origin
    const unexpected: string[] = []
    const route = async (request: Route) => {
        const url = new URL(request.request().url())
        // Vite modules, local static assets and WASM are the only real requests.
        // Application responses must be provided by a story's boundary fixture.
        const localAsset =
            url.origin === origin &&
            (url.pathname.startsWith("/@") ||
                url.pathname.includes("/node_modules/") ||
                /\.(?:[cm]?[jt]sx?|css|wasm|png|jpe?g|svg|woff2?|ttf|ico|map)$/.test(url.pathname))
        if (localAsset) return request.continue()
        unexpected.push(`${request.request().method()} ${url.href}`)
        await request.abort("blockedbyclient")
    }
    guards.set(page, {unexpected, route})
    await page.route("**/*", route)
})

export const finishNetworkGuard = defineBrowserCommand(async ({page}) => {
    const guard = guards.get(page)
    if (!guard) throw new Error("Network guard was not installed")
    await page.unroute("**/*", guard.route)
    guards.delete(page)
    return guard.unexpected
})
