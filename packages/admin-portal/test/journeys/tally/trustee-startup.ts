// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {BrowserContext} from "@playwright/test"
import type {AdminPortal} from "../fixtures"

/** The vendored rayon helper fetches itself before starting real browser workers. */
export async function serveTrusteeWorkerHelper(context: BrowserContext, portal: AdminPortal) {
    const helper = (url: URL) =>
        url.origin === portal.origin &&
        /^\/braid-wasm\/snippets\/wasm-bindgen-rayon-[a-f0-9]+\/src\/workerHelpers\.no-bundler\.js$/.test(
            url.pathname
        )
    await context.route(helper, async (route) => {
        if (route.request().method() !== "GET") {
            portal.violations.add(`Unexpected trustee helper request: ${route.request().method()}`)
            return route.abort("blockedbyclient")
        }
        await route.continue()
    })
    return () => context.unroute(helper)
}
