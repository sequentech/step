// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {isAbort, type MockRequest, type MockResponse} from "../mocks/http"
import type {ViolationLog} from "../mocks/violations"

/** A service mock that answers the requests it handles, such as `S3Mock`. */
export interface FetchService {
    handles(url: URL): boolean
    handle(request: MockRequest): MockResponse | Promise<MockResponse>
}

/**
 * Answers the page's `fetch` calls from service mocks inside the browser, as
 * `routePortal` does for Playwright journeys; stories use it where no page
 * routing exists. Same-origin requests reach the Storybook server, which only
 * serves modules and assets. Any other request is a violation and fails like an
 * unreachable server. Returns a function that restores the original `fetch`.
 */
export function routeFetch(services: FetchService[], violations: ViolationLog): () => void {
    const original = globalThis.fetch
    globalThis.fetch = async (input, init) => {
        const request = new Request(input, init)
        const url = new URL(request.url)
        if (url.origin === globalThis.location.origin) return original(request)
        const service = services.find((candidate) => candidate.handles(url))
        if (!service) {
            violations.add(`Unexpected request: ${request.method} ${url}`)
            throw new TypeError(`Failed to fetch ${url}`)
        }
        const response: MockResponse = await service.handle({
            method: request.method,
            url,
            headers: Object.fromEntries(request.headers),
            body: ["GET", "HEAD"].includes(request.method) ? undefined : await request.text(),
        })
        if (isAbort(response)) throw new TypeError(`Failed to fetch ${url}: ${response.abort}`)
        const body =
            response.body instanceof Uint8Array ? response.body.slice().buffer : response.body
        return new Response(body, {status: response.status, headers: response.headers})
    }
    return () => {
        globalThis.fetch = original
    }
}
